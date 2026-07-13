use crate::object_ddl::service_object_ddl_endpoint;
use crate::postgres::{invalid_postgres_url_message, is_postgres_connection_url};
use crate::ui;
use crate::workspace::{
    resolve_service_workspace, validate_browse_directory_value, workspace_directory_listing,
    workspace_directory_listing_json, workspace_root_candidates, workspace_roots_json,
};
use crate::*;
use ::postgres::{Client, NoTls};
use serde_yaml::{Mapping, Value};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::Duration;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceConfig {
    pub host: String,
    pub port: u16,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 4587,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceHttpResponse {
    pub status_code: u16,
    pub content_type: String,
    pub body: String,
}

pub fn parse_service_args(args: &[String]) -> Result<ServiceConfig, String> {
    let mut config = ServiceConfig::default();
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--host" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--host requires a value".to_string())?;
                if value.trim().is_empty() {
                    return Err("--host must not be empty".to_string());
                }
                config.host = value.to_string();
                index += 2;
            }
            "--port" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--port requires a value".to_string())?;
                config.port = value
                    .parse::<u16>()
                    .map_err(|_| "--port must be a number between 1 and 65535".to_string())?;
                if config.port == 0 {
                    return Err("--port must be a number between 1 and 65535".to_string());
                }
                index += 2;
            }
            "--format" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--format requires a value".to_string())?;
                if value != "json" && value != "text" {
                    return Err("--format must be either json or text".to_string());
                }
                index += 2;
            }
            "--json" => {
                index += 1;
            }
            "--help" | "-h" => return Err(service_usage()),
            value if value.starts_with('-') => {
                return Err(format!("Unknown serve option: {value}"));
            }
            value => return Err(format!("Unexpected serve argument: {value}")),
        }
    }

    Ok(config)
}

pub fn service_usage() -> String {
    "Usage:\n  dbstate serve [--host <host>] [--port <port>] [--format json|--json]".to_string()
}

pub fn run_service(
    args: &[String],
    current_dir: Result<&Path, &std::io::Error>,
) -> Result<(), String> {
    let cwd = current_dir.map_err(|error| format!("Could not read current directory: {error}"))?;
    let config = parse_service_args(args)?;
    let address = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&address)
        .map_err(|error| format!("Could not start DbState service on {address}: {error}"))?;

    println!("DbState Service listening on http://{address}");
    if config.host == "127.0.0.1" || config.host == "localhost" {
        println!("Default service binding is local-only.");
    } else {
        println!("Non-local host binding was explicitly requested. Do not expose v0.1 publicly.");
    }
    println!("DbState Service does not execute generated SQL or apply changes to databases.");

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(error) = handle_http_connection(&mut stream, cwd) {
                    let response = service_error_response(
                        500,
                        "service",
                        &format!("Internal service error: {error}"),
                    );
                    let _ = write_http_response(&mut stream, &response);
                }
            }
            Err(error) => {
                eprintln!("DbState Service connection error: {error}");
            }
        }
    }

    Ok(())
}

fn handle_http_connection(stream: &mut TcpStream, cwd: &Path) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| format!("Could not set read timeout: {error}"))?;
    let request = read_http_request(stream)?;
    let response = service_response(&request.method, &request.path, &request.body, cwd);
    write_http_response(stream, &response)
}

#[derive(Debug, Clone)]
struct HttpRequest {
    method: String,
    path: String,
    body: String,
}

fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    let mut header_end = None;

    while header_end.is_none() {
        let read = stream
            .read(&mut chunk)
            .map_err(|error| format!("Could not read HTTP request: {error}"))?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        header_end = find_header_end(&buffer);
        if buffer.len() > 64 * 1024 {
            return Err("HTTP request header is too large.".to_string());
        }
    }

    let header_end = header_end.ok_or_else(|| "Invalid HTTP request.".to_string())?;
    let header_text = String::from_utf8_lossy(&buffer[..header_end]);
    let mut lines = header_text.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| "HTTP request line is missing.".to_string())?;
    let mut parts = request_line.split_whitespace();
    let method = parts
        .next()
        .ok_or_else(|| "HTTP method is missing.".to_string())?
        .to_string();
    let path = parts
        .next()
        .ok_or_else(|| "HTTP path is missing.".to_string())?
        .split('?')
        .next()
        .unwrap_or("")
        .to_string();

    let mut content_length = 0_usize;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.trim().eq_ignore_ascii_case("content-length") {
            content_length = value
                .trim()
                .parse::<usize>()
                .map_err(|_| "Invalid Content-Length header.".to_string())?;
        }
    }

    let body_start = header_end + 4;
    while buffer.len() < body_start + content_length {
        let read = stream
            .read(&mut chunk)
            .map_err(|error| format!("Could not read HTTP request body: {error}"))?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
    }

    if buffer.len() < body_start + content_length {
        return Err("HTTP request body ended before Content-Length was satisfied.".to_string());
    }

    let body =
        String::from_utf8_lossy(&buffer[body_start..body_start + content_length]).to_string();

    Ok(HttpRequest { method, path, body })
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn write_http_response(
    stream: &mut TcpStream,
    response: &ServiceHttpResponse,
) -> Result<(), String> {
    let reason = match response.status_code {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        409 => "Conflict",
        500 => "Internal Server Error",
        503 => "Service Unavailable",
        _ => "OK",
    };
    let http_response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{}",
        response.status_code,
        reason,
        response.content_type,
        response.body.len(),
        response.body
    );
    stream
        .write_all(http_response.as_bytes())
        .map_err(|error| format!("Could not write HTTP response: {error}"))
}

pub fn service_response(method: &str, path: &str, body: &str, cwd: &Path) -> ServiceHttpResponse {
    let path = path.split('?').next().unwrap_or(path);
    match (method, path) {
        ("GET", "/") | ("GET", "/ui") | ("GET", "/ui/") => {
            service_static_response(200, "text/html; charset=utf-8", ui::index_html())
        }
        ("GET", "/ui/app.css") => {
            service_static_response(200, "text/css; charset=utf-8", ui::app_css())
        }
        ("GET", "/ui/app.js") => {
            service_static_response(200, "application/javascript; charset=utf-8", ui::app_js())
        }
        ("GET", "/health") | ("GET", "/api/v1/health") => service_health_response(),
        ("GET", "/api/v1/workspace/roots") => service_workspace_roots(cwd),
        ("POST", "/api/v1/workspace/list-directories") => {
            service_workspace_list_directories(body, cwd)
        }
        ("POST", "/api/v1/workspace/validate") => service_workspace_validate(body, cwd),
        ("GET", "/api/v1/connections/profiles") => service_connection_profiles_list(),
        ("POST", "/api/v1/connections/profiles") => service_connection_profile_create(body),
        ("POST", "/api/v1/connections/test") => service_connection_test(body),
        ("POST", "/api/v1/repo/status") => service_cli_endpoint(
            "repo status",
            body,
            cwd,
            &["repo", "status", "--format", "json"],
        ),
        ("POST", "/api/v1/init/plan") => service_init_plan_endpoint(body, cwd),
        ("POST", "/api/v1/init/write") => service_init_write_endpoint(body, cwd),
        ("POST", "/api/v1/postgres/inspect") => service_postgres_endpoint(
            "inspect postgres",
            body,
            cwd,
            &["inspect", "postgres"],
            ScopeRequirement::Optional,
            EndpointScopeKind::SchemaTable,
        ),
        ("POST", "/api/v1/postgres/compare") => service_postgres_endpoint(
            "compare postgres",
            body,
            cwd,
            &["compare", "postgres"],
            ScopeRequirement::Required,
            EndpointScopeKind::SchemaTable,
        ),
        ("POST", "/api/v1/postgres/plan") => service_postgres_endpoint(
            "plan postgres",
            body,
            cwd,
            &["plan", "postgres"],
            ScopeRequirement::Required,
            EndpointScopeKind::SchemaTable,
        ),
        ("POST", "/api/v1/postgres/data-compare") => service_postgres_endpoint(
            "data-compare postgres",
            body,
            cwd,
            &["data-compare", "postgres"],
            ScopeRequirement::Required,
            EndpointScopeKind::DataCompare,
        ),
        ("POST", "/api/v1/postgres/object-ddl") => service_object_ddl_endpoint(body, cwd),
        ("POST", "/api/v1/postgres/repository-sync/preview") => {
            service_repository_sync_endpoint("repository-sync preview", body, cwd, true)
        }
        ("POST", "/api/v1/postgres/repository-sync/write") => {
            service_repository_sync_endpoint("repository-sync write", body, cwd, false)
        }
        ("POST", "/api/v1/postgres/release/preview") => {
            service_release_endpoint("release preview", body, cwd, true)
        }
        ("POST", "/api/v1/postgres/release/write") => {
            service_release_endpoint("release write", body, cwd, false)
        }
        ("POST", "/api/v1/releases/artifact-preview") => {
            service_release_artifact_preview_endpoint(body, cwd)
        }
        _ if method == "PUT" && path.starts_with("/api/v1/connections/profiles/") => {
            service_connection_profile_update(path, body)
        }
        _ if method == "DELETE" && path.starts_with("/api/v1/connections/profiles/") => {
            service_connection_profile_delete(path)
        }
        _ => service_error_response(404, "service", "Unknown DbState Service route."),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeRequirement {
    Required,
    Optional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EndpointScopeKind {
    SchemaTable,
    DataCompare,
}

pub fn service_route_definitions() -> Vec<(&'static str, &'static str)> {
    vec![
        ("GET", "/"),
        ("GET", "/ui"),
        ("GET", "/ui/"),
        ("GET", "/ui/app.css"),
        ("GET", "/ui/app.js"),
        ("GET", "/health"),
        ("GET", "/api/v1/health"),
        ("GET", "/api/v1/workspace/roots"),
        ("POST", "/api/v1/workspace/list-directories"),
        ("POST", "/api/v1/workspace/validate"),
        ("GET", "/api/v1/connections/profiles"),
        ("POST", "/api/v1/connections/profiles"),
        ("PUT", "/api/v1/connections/profiles/{name}"),
        ("DELETE", "/api/v1/connections/profiles/{name}"),
        ("POST", "/api/v1/connections/test"),
        ("POST", "/api/v1/repo/status"),
        ("POST", "/api/v1/init/plan"),
        ("POST", "/api/v1/init/write"),
        ("POST", "/api/v1/postgres/inspect"),
        ("POST", "/api/v1/postgres/compare"),
        ("POST", "/api/v1/postgres/plan"),
        ("POST", "/api/v1/postgres/data-compare"),
        ("POST", "/api/v1/postgres/object-ddl"),
        ("POST", "/api/v1/postgres/repository-sync/preview"),
        ("POST", "/api/v1/postgres/repository-sync/write"),
        ("POST", "/api/v1/postgres/release/preview"),
        ("POST", "/api/v1/postgres/release/write"),
        ("POST", "/api/v1/releases/artifact-preview"),
    ]
}

fn service_health_response() -> ServiceHttpResponse {
    let mut body = String::new();
    body.push('{');
    write_json_string_field(&mut body, "command", "service health", true);
    write_json_bool_field(&mut body, "success", true);
    write_json_string_field(&mut body, "service", "dbstate", false);
    write_json_string_field(&mut body, "apiVersion", "v1", false);
    write_json_array_field(&mut body, "warnings", &[]);
    write_json_array_field(&mut body, "errors", &[]);
    body.push('}');
    ServiceHttpResponse {
        status_code: 200,
        content_type: "application/json; charset=utf-8".to_string(),
        body,
    }
}

fn service_workspace_roots(cwd: &Path) -> ServiceHttpResponse {
    let mut roots = workspace_root_candidates(cwd);
    roots.sort_by(|left, right| {
        left.path
            .to_ascii_lowercase()
            .cmp(&right.path.to_ascii_lowercase())
    });
    roots.dedup_by(|left, right| left.path.eq_ignore_ascii_case(&right.path));
    service_json_response(200, &workspace_roots_json(&roots, cwd))
}

fn service_workspace_list_directories(body: &str, cwd: &Path) -> ServiceHttpResponse {
    let request = match parse_service_request(body) {
        Ok(request) => request,
        Err(error) => return service_error_response(400, "workspace list-directories", &error),
    };
    if let Err(error) = validate_service_request_is_safe(&request) {
        return service_error_response(400, "workspace list-directories", &error);
    }
    let path_value = request_string(&request, "path").unwrap_or_else(|| display_path(cwd));
    let directory = match validate_browse_directory_value(&path_value) {
        Ok(path) => path,
        Err(error) => return service_error_response(400, "workspace list-directories", &error),
    };
    let report = match workspace_directory_listing(&directory) {
        Ok(report) => report,
        Err(error) => return service_error_response(400, "workspace list-directories", &error),
    };
    service_json_response(200, &workspace_directory_listing_json(&report))
}

fn service_workspace_validate(body: &str, cwd: &Path) -> ServiceHttpResponse {
    service_cli_endpoint("workspace validate", body, cwd, &["repo", "status"])
}

fn service_connection_profiles_list() -> ServiceHttpResponse {
    match load_connection_profiles() {
        Ok(store) => service_json_response(
            200,
            &connection_profiles_json("connections profiles", true, &store, &[], &[]),
        ),
        Err(error) => service_error_response(400, "connections profiles", &error),
    }
}

fn service_connection_profile_create(body: &str) -> ServiceHttpResponse {
    let request = match parse_service_request(body) {
        Ok(request) => request,
        Err(error) => return service_error_response(400, "connections profiles", &error),
    };
    if let Err(error) = validate_service_request_is_safe(&request) {
        return service_error_response(400, "connections profiles", &error);
    }
    let profile = match parse_connection_profile(&request) {
        Ok(profile) => profile,
        Err(error) => return service_error_response(400, "connections profiles", &error),
    };
    let mut store = match load_connection_profiles() {
        Ok(store) => store,
        Err(error) => return service_error_response(400, "connections profiles", &error),
    };
    if store
        .profiles
        .iter()
        .any(|item| item.name.eq_ignore_ascii_case(&profile.name))
    {
        return service_error_response(
            400,
            "connections profiles",
            "Connection profile already exists.",
        );
    }
    store.profiles.push(profile);
    if let Err(error) = save_connection_profiles(&store) {
        return service_error_response(400, "connections profiles", &error);
    }
    service_json_response(
        200,
        &connection_profiles_json("connections profiles", true, &store, &[], &[]),
    )
}

fn service_connection_profile_update(path: &str, body: &str) -> ServiceHttpResponse {
    let name = match profile_name_from_path(path) {
        Ok(name) => name,
        Err(error) => return service_error_response(400, "connections profiles", &error),
    };
    let request = match parse_service_request(body) {
        Ok(request) => request,
        Err(error) => return service_error_response(400, "connections profiles", &error),
    };
    if let Err(error) = validate_service_request_is_safe(&request) {
        return service_error_response(400, "connections profiles", &error);
    }
    let mut profile = match parse_connection_profile(&request) {
        Ok(profile) => profile,
        Err(error) => return service_error_response(400, "connections profiles", &error),
    };
    if !profile.name.eq_ignore_ascii_case(&name) {
        profile.name = name.clone();
        if let Err(error) = validate_connection_profile(&profile) {
            return service_error_response(400, "connections profiles", &error);
        }
    }
    let mut store = match load_connection_profiles() {
        Ok(store) => store,
        Err(error) => return service_error_response(400, "connections profiles", &error),
    };
    let Some(existing) = store
        .profiles
        .iter_mut()
        .find(|item| item.name.eq_ignore_ascii_case(&name))
    else {
        return service_error_response(
            404,
            "connections profiles",
            "Connection profile was not found.",
        );
    };
    *existing = profile;
    if let Err(error) = validate_unique_profile_names(&store.profiles) {
        return service_error_response(400, "connections profiles", &error);
    }
    if let Err(error) = save_connection_profiles(&store) {
        return service_error_response(400, "connections profiles", &error);
    }
    service_json_response(
        200,
        &connection_profiles_json("connections profiles", true, &store, &[], &[]),
    )
}

fn service_connection_profile_delete(path: &str) -> ServiceHttpResponse {
    let name = match profile_name_from_path(path) {
        Ok(name) => name,
        Err(error) => return service_error_response(400, "connections profiles", &error),
    };
    let mut store = match load_connection_profiles() {
        Ok(store) => store,
        Err(error) => return service_error_response(400, "connections profiles", &error),
    };
    let before = store.profiles.len();
    store
        .profiles
        .retain(|item| !item.name.eq_ignore_ascii_case(&name));
    if store.profiles.len() == before {
        return service_error_response(
            404,
            "connections profiles",
            "Connection profile was not found.",
        );
    }
    if let Err(error) = save_connection_profiles(&store) {
        return service_error_response(400, "connections profiles", &error);
    }
    service_json_response(
        200,
        &connection_profiles_json("connections profiles", true, &store, &[], &[]),
    )
}

fn service_connection_test(body: &str) -> ServiceHttpResponse {
    let request = match parse_service_request(body) {
        Ok(request) => request,
        Err(error) => return service_error_response(400, "connections test", &error),
    };
    if let Err(error) = validate_service_request_is_safe(&request) {
        return service_error_response(400, "connections test", &error);
    }
    let resolved = match resolve_service_postgres_connection(&request) {
        Ok(Some(resolved)) => resolved,
        Ok(None) => {
            return service_error_response(
                400,
                "connections test",
                "Missing PostgreSQL connection. Provide postgresUrl, connection.profileName, or DBSTATE_POSTGRES_URL.",
            )
        }
        Err(error) => return service_error_response(400, "connections test", &error),
    };
    let report = test_postgres_connection(&resolved);
    let status = if report.success { 200 } else { 503 };
    service_json_response(status, &connection_test_json(&report))
}

pub(crate) fn service_json_response(status_code: u16, body: &str) -> ServiceHttpResponse {
    ServiceHttpResponse {
        status_code,
        content_type: "application/json; charset=utf-8".to_string(),
        body: body.to_string(),
    }
}

fn profile_name_from_path(path: &str) -> Result<String, String> {
    let prefix = "/api/v1/connections/profiles/";
    let Some(name) = path.strip_prefix(prefix) else {
        return Err("Connection profile name is missing.".to_string());
    };
    let name = percent_decode_path_segment(name)?;
    validate_profile_name(&name)?;
    Ok(name)
}

fn percent_decode_path_segment(value: &str) -> Result<String, String> {
    let bytes = value.as_bytes();
    let mut output = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return Err("Invalid encoded profile name.".to_string());
            }
            let hex = std::str::from_utf8(&bytes[index + 1..index + 3])
                .map_err(|_| "Invalid encoded profile name.".to_string())?;
            let value = u8::from_str_radix(hex, 16)
                .map_err(|_| "Invalid encoded profile name.".to_string())?;
            output.push(value);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(output).map_err(|_| "Invalid encoded profile name.".to_string())
}

fn connection_profiles_json(
    command: &str,
    success: bool,
    store: &ConnectionProfileStore,
    warnings: &[String],
    errors: &[String],
) -> String {
    let mut json = String::new();
    json.push('{');
    write_json_string_field(&mut json, "command", command, true);
    write_json_bool_field(&mut json, "success", success);
    json.push_str(",\"profiles\":[");
    let mut profiles = store.profiles.clone();
    profiles.sort_by(|left, right| left.name.cmp(&right.name));
    for (index, profile) in profiles.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push_str(&profile.to_json_object());
    }
    json.push(']');
    write_json_array_field(&mut json, "warnings", warnings);
    write_json_array_field(&mut json, "errors", errors);
    json.push('}');
    json
}

fn test_postgres_connection(resolved: &ResolvedPostgresConnection) -> ConnectionTestReport {
    let mut report = ConnectionTestReport {
        command: "connections test".to_string(),
        success: false,
        database_type: "postgresql".to_string(),
        connection_source: resolved.source.clone(),
        database_name: None,
        database_user: None,
        warnings: Vec::new(),
        errors: Vec::new(),
    };

    if !is_postgres_connection_url(&resolved.url) {
        report.errors.push(invalid_postgres_url_message());
        return report;
    }

    match Client::connect(&resolved.url, NoTls) {
        Ok(mut client) => {
            match client.query_one("SELECT current_database(), current_user", &[]) {
                Ok(row) => {
                    report.database_name = Some(row.get::<_, String>(0));
                    report.database_user = Some(row.get::<_, String>(1));
                    report.success = true;
                }
                Err(_) => report
                    .errors
                    .push("PostgreSQL connection test failed while running a read-only query.".to_string()),
            }
        }
        Err(_) => report.errors.push(
            "PostgreSQL connection failed. Verify the session-only connection values, credentials, network, and database availability.".to_string(),
        ),
    }
    report
}

fn connection_test_json(report: &ConnectionTestReport) -> String {
    let mut json = String::new();
    json.push('{');
    write_json_string_field(&mut json, "command", &report.command, true);
    write_json_bool_field(&mut json, "success", report.success);
    write_json_string_field(&mut json, "databaseType", &report.database_type, false);
    write_json_string_field(
        &mut json,
        "connectionSource",
        &report.connection_source,
        false,
    );
    write_json_optional_string_field(&mut json, "databaseName", report.database_name.as_deref());
    write_json_optional_string_field(&mut json, "databaseUser", report.database_user.as_deref());
    write_json_array_field(&mut json, "warnings", &report.warnings);
    write_json_array_field(&mut json, "errors", &report.errors);
    json.push('}');
    json
}

fn service_static_response(
    status_code: u16,
    content_type: &str,
    body: &str,
) -> ServiceHttpResponse {
    ServiceHttpResponse {
        status_code,
        content_type: content_type.to_string(),
        body: body.to_string(),
    }
}

fn service_init_plan_endpoint(body: &str, cwd: &Path) -> ServiceHttpResponse {
    let request = match parse_service_request(body) {
        Ok(request) => request,
        Err(error) => return service_error_response(400, "init plan", &error),
    };
    if let Err(error) = validate_service_request_is_safe(&request) {
        return service_error_response(400, "init plan", &error);
    }
    let workspace =
        match resolve_service_workspace(request_string(&request, "repositoryPath").as_deref(), cwd)
        {
            Ok(workspace) => workspace,
            Err(error) => return service_error_response(400, "init plan", &error),
        };
    if matches!(request_bool(&request, "dryRun"), Some(false)) {
        return service_error_response(
            400,
            "init plan",
            "Slice 11 service supports init planning only. Use dryRun true or omit dryRun.",
        );
    }
    let args = vec![
        "init".to_string(),
        "--dry-run".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];
    service_run_cli("init plan", &workspace, args)
}

fn service_init_write_endpoint(body: &str, cwd: &Path) -> ServiceHttpResponse {
    let request = match parse_service_request(body) {
        Ok(request) => request,
        Err(error) => return service_error_response(400, "init write", &error),
    };
    if let Err(error) = validate_service_request_is_safe(&request) {
        return service_error_response(400, "init write", &error);
    }
    let confirmed = matches!(
        request_bool(&request, "confirmInitializeProject"),
        Some(true)
    ) && matches!(
        request_string(&request, "confirmationText").as_deref(),
        Some("INITIALIZE DBSTATE PROJECT")
    );
    if !confirmed {
        return service_error_response(
            400,
            "init write",
            "Initialization requires confirmInitializeProject true and confirmationText INITIALIZE DBSTATE PROJECT.",
        );
    }
    let workspace =
        match resolve_service_workspace(request_string(&request, "repositoryPath").as_deref(), cwd)
        {
            Ok(workspace) => workspace,
            Err(error) => return service_error_response(400, "init write", &error),
        };
    match init_project(&workspace, false) {
        Ok(report) => {
            let status = if report.success {
                200
            } else if !report.is_git_repository {
                400
            } else {
                409
            };
            service_json_response(status, &report.to_json())
        }
        Err(error) => service_error_response(400, "init write", &error),
    }
}

fn service_cli_endpoint(
    command: &str,
    body: &str,
    cwd: &Path,
    base_args: &[&str],
) -> ServiceHttpResponse {
    let request = match parse_service_request(body) {
        Ok(request) => request,
        Err(error) => return service_error_response(400, command, &error),
    };
    if let Err(error) = validate_service_request_is_safe(&request) {
        return service_error_response(400, command, &error);
    }
    let workspace =
        match resolve_service_workspace(request_string(&request, "repositoryPath").as_deref(), cwd)
        {
            Ok(workspace) => workspace,
            Err(error) => return service_error_response(400, command, &error),
        };
    let mut args: Vec<String> = base_args.iter().map(|value| (*value).to_string()).collect();
    args.extend(["--format".to_string(), "json".to_string()]);
    service_run_cli(command, &workspace, args)
}

fn service_postgres_endpoint(
    command: &str,
    body: &str,
    cwd: &Path,
    base_args: &[&str],
    scope_requirement: ScopeRequirement,
    scope_kind: EndpointScopeKind,
) -> ServiceHttpResponse {
    let request = match parse_service_request(body) {
        Ok(request) => request,
        Err(error) => return service_error_response(400, command, &error),
    };
    if let Err(error) = validate_service_request_is_safe(&request) {
        return service_error_response(400, command, &error);
    }
    let workspace =
        match resolve_service_workspace(request_string(&request, "repositoryPath").as_deref(), cwd)
        {
            Ok(workspace) => workspace,
            Err(error) => return service_error_response(400, command, &error),
        };

    let mut args: Vec<String> = base_args.iter().map(|value| (*value).to_string()).collect();
    match resolve_service_postgres_connection(&request) {
        Ok(Some(connection)) => {
            if connection.source != "environment" {
                args.push("--url".to_string());
                args.push(connection.url);
            }
        }
        Ok(None) => {}
        Err(error) => return service_error_response(400, command, &error),
    }

    match service_scope_args(&request, scope_requirement, scope_kind) {
        Ok(scope_args) => args.extend(scope_args),
        Err(error) => return service_error_response(400, command, &error),
    }

    for include in request_string_array(&request, "include") {
        args.push("--include".to_string());
        args.push(include);
    }
    for exclude in request_string_array(&request, "exclude") {
        args.push("--exclude".to_string());
        args.push(exclude);
    }

    args.extend(["--format".to_string(), "json".to_string()]);
    service_run_cli(command, &workspace, args)
}

fn service_release_endpoint(
    command: &str,
    body: &str,
    cwd: &Path,
    dry_run: bool,
) -> ServiceHttpResponse {
    let request = match parse_service_request(body) {
        Ok(request) => request,
        Err(error) => return service_error_response(400, command, &error),
    };
    if let Err(error) = validate_service_request_is_safe(&request) {
        return service_error_response(400, command, &error);
    }
    if !dry_run {
        let confirmed = matches!(
            request_bool(&request, "confirmReleaseArtifacts"),
            Some(true)
        ) && matches!(
            request_string(&request, "confirmationText").as_deref(),
            Some("GENERATE RELEASE ARTIFACTS")
        );
        if !confirmed {
            return service_error_response(
                400,
                command,
                "Release artifact generation requires confirmReleaseArtifacts true and confirmationText GENERATE RELEASE ARTIFACTS.",
            );
        }
    }
    let workspace =
        match resolve_service_workspace(request_string(&request, "repositoryPath").as_deref(), cwd)
        {
            Ok(workspace) => workspace,
            Err(error) => return service_error_response(400, command, &error),
        };
    let release_name = match request_string(&request, "releaseName") {
        Some(value) if !value.trim().is_empty() => value,
        _ => {
            return service_error_response(
                400,
                command,
                "Release name is required. Provide releaseName.",
            )
        }
    };

    let mut args = vec!["release".to_string(), "postgres".to_string()];
    match resolve_service_postgres_connection(&request) {
        Ok(Some(connection)) => {
            if connection.source != "environment" {
                args.push("--url".to_string());
                args.push(connection.url);
            }
        }
        Ok(None) => {}
        Err(error) => return service_error_response(400, command, &error),
    }
    match service_scope_args(
        &request,
        ScopeRequirement::Required,
        EndpointScopeKind::SchemaTable,
    ) {
        Ok(scope_args) => args.extend(scope_args),
        Err(error) => return service_error_response(400, command, &error),
    }
    args.push("--name".to_string());
    args.push(release_name);
    for include in request_string_array(&request, "include") {
        args.push("--include".to_string());
        args.push(include);
    }
    for exclude in request_string_array(&request, "exclude") {
        args.push("--exclude".to_string());
        args.push(exclude);
    }
    if dry_run {
        args.push("--dry-run".to_string());
    }
    args.extend(["--format".to_string(), "json".to_string()]);
    service_run_cli(command, &workspace, args)
}

const MAX_RELEASE_ARTIFACT_PREVIEW_BYTES: usize = 1024 * 1024;

fn service_release_artifact_preview_endpoint(body: &str, cwd: &Path) -> ServiceHttpResponse {
    let command = "release artifact preview";
    let request = match parse_service_request(body) {
        Ok(request) => request,
        Err(error) => return service_error_response(400, command, &error),
    };
    if let Err(error) = validate_service_request_is_safe(&request) {
        return service_error_response(400, command, &error);
    }
    let repository_path = match request_string(&request, "repositoryPath") {
        Some(value) if !value.trim().is_empty() => value,
        _ => {
            return service_error_response(
                400,
                command,
                "repositoryPath is required for release artifact preview.",
            )
        }
    };
    let workspace = match resolve_service_workspace(Some(repository_path.as_str()), cwd) {
        Ok(workspace) => workspace,
        Err(error) => return service_error_response(400, command, &error),
    };
    let project = status_report(&workspace, CommandKind::RepoStatus);
    if !project.is_git_repository {
        return service_error_response(
            400,
            command,
            "Selected repositoryPath is not inside a Git repository.",
        );
    }
    if project.dbstate_project_status != DbStateProjectStatus::CompleteDbStateStructure {
        return service_error_response(
            409,
            command,
            "Selected repositoryPath does not contain complete DbState project structure.",
        );
    }
    let artifact_path = match request_string(&request, "artifactPath") {
        Some(value) if !value.trim().is_empty() => value,
        _ => {
            return service_error_response(
                400,
                command,
                "artifactPath is required for release artifact preview.",
            )
        }
    };
    let artifact_path = match normalize_release_artifact_preview_path(&artifact_path) {
        Ok(value) => value,
        Err(error) => return service_error_response(400, command, &error),
    };
    let artifact_type = match release_artifact_preview_type(&artifact_path) {
        Some(value) => value,
        None => {
            return service_error_response(
                400,
                command,
                "Release artifact preview supports only .sql, .summary.md, .risk.json, and .manifest.json files.",
            )
        }
    };

    let releases_dir = workspace.join("database").join("releases");
    let artifact_file = workspace.join(Path::new(&artifact_path));
    let releases_canonical = match fs::canonicalize(&releases_dir) {
        Ok(value) => value,
        Err(_) => {
            return service_error_response(
                400,
                command,
                "database/releases does not exist in the selected repository.",
            )
        }
    };
    let metadata = match fs::symlink_metadata(&artifact_file) {
        Ok(value) => value,
        Err(_) => {
            return service_error_response(
                404,
                command,
                "Release artifact was not found under database/releases.",
            )
        }
    };
    if metadata.file_type().is_symlink() {
        return service_error_response(
            400,
            command,
            "Release artifact preview does not follow symlinks.",
        );
    }
    if metadata.is_dir() {
        return service_error_response(
            400,
            command,
            "Release artifact preview requires a file, not a directory.",
        );
    }
    if !metadata.is_file() {
        return service_error_response(400, command, "Release artifact is not a regular file.");
    }
    let artifact_canonical = match fs::canonicalize(&artifact_file) {
        Ok(value) => value,
        Err(_) => {
            return service_error_response(
                404,
                command,
                "Release artifact was not found under database/releases.",
            )
        }
    };
    if !artifact_canonical.starts_with(&releases_canonical) {
        return service_error_response(
            400,
            command,
            "Release artifact path must stay under database/releases.",
        );
    }

    let mut file = match fs::File::open(&artifact_canonical) {
        Ok(value) => value,
        Err(_) => {
            return service_error_response(
                400,
                command,
                "Release artifact could not be opened for read-only preview.",
            )
        }
    };
    let mut bytes = Vec::new();
    let read_limit = MAX_RELEASE_ARTIFACT_PREVIEW_BYTES + 1;
    if let Err(error) = Read::by_ref(&mut file)
        .take(read_limit as u64)
        .read_to_end(&mut bytes)
    {
        return service_error_response(
            400,
            command,
            &format!("Release artifact could not be read for preview: {error}"),
        );
    }
    let truncated = bytes.len() > MAX_RELEASE_ARTIFACT_PREVIEW_BYTES;
    if truncated {
        bytes.truncate(MAX_RELEASE_ARTIFACT_PREVIEW_BYTES);
    }
    let content = String::from_utf8_lossy(&bytes).to_string();
    let warnings = if truncated {
        vec!["Release artifact preview was truncated at 1 MiB.".to_string()]
    } else {
        Vec::new()
    };
    service_release_artifact_preview_json(
        &artifact_path,
        artifact_type,
        &content,
        truncated,
        &warnings,
    )
}

fn normalize_release_artifact_preview_path(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("artifactPath is required for release artifact preview.".to_string());
    }
    if trimmed.contains('\0') {
        return Err("artifactPath contains an invalid null byte.".to_string());
    }
    if trimmed.contains(':') || trimmed.starts_with('/') || trimmed.starts_with('\\') {
        return Err("artifactPath must be a relative path under database/releases.".to_string());
    }
    if trimmed.to_ascii_lowercase().contains("%2e") {
        return Err("artifactPath must not contain encoded traversal segments.".to_string());
    }
    let normalized = trimmed.replace('\\', "/");
    let parts: Vec<&str> = normalized.split('/').collect();
    if parts
        .iter()
        .any(|part| part.is_empty() || *part == "." || *part == "..")
    {
        return Err("artifactPath must not contain traversal segments.".to_string());
    }
    if parts.len() < 3 || parts[0] != "database" || parts[1] != "releases" {
        return Err("artifactPath must stay under database/releases.".to_string());
    }
    Ok(parts.join("/"))
}

fn release_artifact_preview_type(path: &str) -> Option<&'static str> {
    if path.ends_with(".summary.md") {
        Some("summary")
    } else if path.ends_with(".risk.json") {
        Some("risk")
    } else if path.ends_with(".manifest.json") {
        Some("manifest")
    } else if path.ends_with(".sql") {
        Some("sql")
    } else {
        None
    }
}

fn service_release_artifact_preview_json(
    artifact_path: &str,
    artifact_type: &str,
    content: &str,
    truncated: bool,
    warnings: &[String],
) -> ServiceHttpResponse {
    let file_name = artifact_path.rsplit('/').next().unwrap_or(artifact_path);
    let mut body = String::new();
    body.push('{');
    write_json_string_field(&mut body, "command", "release artifact preview", true);
    write_json_bool_field(&mut body, "success", true);
    write_json_string_field(&mut body, "artifactPath", artifact_path, false);
    write_json_string_field(&mut body, "fileName", file_name, false);
    write_json_string_field(&mut body, "artifactType", artifact_type, false);
    write_json_string_field(&mut body, "content", content, false);
    write_json_bool_field(&mut body, "truncated", truncated);
    write_json_array_field(&mut body, "warnings", warnings);
    write_json_array_field(&mut body, "errors", &[]);
    body.push('}');
    ServiceHttpResponse {
        status_code: 200,
        content_type: "application/json; charset=utf-8".to_string(),
        body,
    }
}

fn service_repository_sync_endpoint(
    command: &str,
    body: &str,
    cwd: &Path,
    dry_run: bool,
) -> ServiceHttpResponse {
    let request = match parse_service_request(body) {
        Ok(request) => request,
        Err(error) => return service_error_response(400, command, &error),
    };
    if let Err(error) = validate_service_request_is_safe(&request) {
        return service_error_response(400, command, &error);
    }
    if !dry_run {
        let confirmed = matches!(request_bool(&request, "confirmRepositoryWrite"), Some(true))
            && matches!(
                request_string(&request, "confirmationText").as_deref(),
                Some("WRITE REPOSITORY FILES")
            );
        if !confirmed {
            return service_error_response(
                400,
                command,
                "Repository file write requires confirmRepositoryWrite true and confirmationText WRITE REPOSITORY FILES.",
            );
        }
    }
    let workspace =
        match resolve_service_workspace(request_string(&request, "repositoryPath").as_deref(), cwd)
        {
            Ok(workspace) => workspace,
            Err(error) => return service_error_response(400, command, &error),
        };

    let mut args = vec!["sync".to_string(), "postgres".to_string()];
    match resolve_service_postgres_connection(&request) {
        Ok(Some(connection)) => {
            if connection.source != "environment" {
                args.push("--url".to_string());
                args.push(connection.url);
            }
        }
        Ok(None) => {}
        Err(error) => return service_error_response(400, command, &error),
    }

    match service_scope_args(
        &request,
        ScopeRequirement::Required,
        EndpointScopeKind::SchemaTable,
    ) {
        Ok(scope_args) => args.extend(scope_args),
        Err(error) => return service_error_response(400, command, &error),
    }
    if dry_run {
        args.push("--dry-run".to_string());
    }
    args.extend(["--format".to_string(), "json".to_string()]);
    service_run_cli(command, &workspace, args)
}

fn service_run_cli(command: &str, cwd: &Path, args: Vec<String>) -> ServiceHttpResponse {
    match run_cli(&args, Ok(cwd)) {
        Ok(result) => {
            let body = result.output.to_json();
            let status_code = service_status_from_cli_result(result.exit_code, &body);
            ServiceHttpResponse {
                status_code,
                content_type: "application/json; charset=utf-8".to_string(),
                body,
            }
        }
        Err(error) => service_error_response(400, command, &error),
    }
}

fn service_status_from_cli_result(exit_code: u8, body: &str) -> u16 {
    if exit_code == 0 {
        200
    } else if body.contains("PostgreSQL connection failed") {
        503
    } else if body.contains("DbState PostgreSQL project structure is incomplete") {
        409
    } else {
        400
    }
}

pub(crate) fn parse_service_request(body: &str) -> Result<Value, String> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Ok(Value::Mapping(Mapping::new()));
    }
    if !((trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']')))
    {
        return Err("Invalid JSON request body.".to_string());
    }
    serde_yaml::from_str::<Value>(trimmed).map_err(|_| "Invalid JSON request body.".to_string())
}

pub(crate) fn validate_service_request_is_safe(request: &Value) -> Result<(), String> {
    for key in ["write", "apply", "execute", "directApply", "mutateDatabase"] {
        if matches!(request_bool(request, key), Some(true)) {
            return Err("Slice 11 service endpoints are read-only or plan-only and do not support write, apply, execute, or database mutation requests.".to_string());
        }
    }
    Ok(())
}

fn config_dir() -> Result<PathBuf, String> {
    if let Ok(value) = env::var("DBSTATE_CONFIG_DIR") {
        if !value.trim().is_empty() {
            return Ok(PathBuf::from(value));
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(value) = env::var("APPDATA") {
            if !value.trim().is_empty() {
                return Ok(PathBuf::from(value).join("DbState"));
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(value) = env::var("HOME") {
            if !value.trim().is_empty() {
                return Ok(PathBuf::from(value)
                    .join("Library")
                    .join("Application Support")
                    .join("DbState"));
            }
        }
    }

    if let Ok(value) = env::var("XDG_CONFIG_HOME") {
        if !value.trim().is_empty() {
            return Ok(PathBuf::from(value).join("dbstate"));
        }
    }
    if let Ok(value) = env::var("HOME") {
        if !value.trim().is_empty() {
            return Ok(PathBuf::from(value).join(".config").join("dbstate"));
        }
    }
    Err("Could not resolve DbState config directory. Set DBSTATE_CONFIG_DIR.".to_string())
}

fn profile_file_path() -> Result<PathBuf, String> {
    Ok(config_dir()?.join(PROFILE_FILE_NAME))
}

pub fn load_connection_profiles() -> Result<ConnectionProfileStore, String> {
    let path = profile_file_path()?;
    load_connection_profiles_from_path(&path)
}

pub(crate) fn load_connection_profiles_from_path(
    path: &Path,
) -> Result<ConnectionProfileStore, String> {
    if !path.exists() {
        return Ok(ConnectionProfileStore {
            profiles: Vec::new(),
        });
    }
    let content = fs::read_to_string(path)
        .map_err(|error| format!("Could not read connection profiles: {error}"))?;
    let value: Value = serde_yaml::from_str(&content)
        .map_err(|_| "Connection profile file is invalid JSON.".to_string())?;
    reject_secret_fields(&value)?;
    parse_profile_store(&value)
}

pub(crate) fn save_connection_profiles(store: &ConnectionProfileStore) -> Result<(), String> {
    let path = profile_file_path()?;
    save_connection_profiles_to_path(&path, store)
}

fn save_connection_profiles_to_path(
    path: &Path,
    store: &ConnectionProfileStore,
) -> Result<(), String> {
    validate_unique_profile_names(&store.profiles)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create DbState config directory: {error}"))?;
    }
    let json = profile_store_to_json(store);
    fs::write(path, json).map_err(|error| format!("Could not write connection profiles: {error}"))
}

fn profile_store_to_json(store: &ConnectionProfileStore) -> String {
    let mut profiles = store.profiles.clone();
    profiles.sort_by(|left, right| left.name.cmp(&right.name));
    let mut json = String::new();
    json.push('{');
    write_json_i32_field(&mut json, "version", PROFILE_FILE_VERSION, true);
    json.push_str(",\"profiles\":[");
    for (index, profile) in profiles.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        json.push_str(&profile.to_json_object());
    }
    json.push_str("]}\n");
    json
}

fn parse_profile_store(value: &Value) -> Result<ConnectionProfileStore, String> {
    let version = request_i64(value, "version")
        .ok_or_else(|| "Connection profile file must include version 1.".to_string())?;
    if version != PROFILE_FILE_VERSION as i64 {
        return Err("Connection profile file version is not supported.".to_string());
    }
    let profiles_value = mapping_get(value, "profiles")
        .ok_or_else(|| "Connection profile file must include profiles array.".to_string())?;
    let Value::Sequence(items) = profiles_value else {
        return Err("Connection profile file profiles value must be an array.".to_string());
    };
    let mut profiles = Vec::new();
    for item in items {
        profiles.push(parse_connection_profile(item)?);
    }
    validate_unique_profile_names(&profiles)?;
    Ok(ConnectionProfileStore { profiles })
}

pub(crate) fn parse_connection_profile(value: &Value) -> Result<ConnectionProfile, String> {
    reject_secret_fields(value)?;
    let name = required_profile_string(value, "name")?;
    let host = required_profile_string(value, "host")?;
    let database = required_profile_string(value, "database")?;
    let username = required_profile_string(value, "username")?;
    let port = request_i64(value, "port").ok_or_else(|| "Profile port is required.".to_string())?;
    if !(1..=65535).contains(&port) {
        return Err("Profile port must be between 1 and 65535.".to_string());
    }
    let ssl_mode = request_string(value, "sslMode").unwrap_or_else(|| "prefer".to_string());
    let description = request_string(value, "description").filter(|value| !value.trim().is_empty());
    let default_schema =
        request_string(value, "defaultSchema").filter(|value| !value.trim().is_empty());
    let profile = ConnectionProfile {
        name,
        host,
        port: port as u16,
        database,
        username,
        ssl_mode,
        description,
        default_schema,
    };
    validate_connection_profile(&profile)?;
    Ok(profile)
}

fn required_profile_string(value: &Value, key: &str) -> Result<String, String> {
    let Some(text) = request_string(value, key) else {
        return Err(format!("Profile {key} is required."));
    };
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(format!("Profile {key} is required."));
    }
    if trimmed.contains('\0') {
        return Err(format!("Profile {key} contains an invalid null byte."));
    }
    Ok(trimmed.to_string())
}

pub(crate) fn validate_connection_profile(profile: &ConnectionProfile) -> Result<(), String> {
    validate_profile_name(&profile.name)?;
    validate_profile_component("host", &profile.host)?;
    validate_profile_component("database", &profile.database)?;
    validate_profile_component("username", &profile.username)?;
    if profile.port == 0 {
        return Err("Profile port must be between 1 and 65535.".to_string());
    }
    if is_profile_host_url_like(&profile.host) {
        return Err("Profile host must be a host name or address, not a URL.".to_string());
    }
    if !ALLOWED_SSL_MODES.contains(&profile.ssl_mode.as_str()) {
        return Err(
            "Profile sslMode must be one of disable, prefer, require, verify-ca, or verify-full."
                .to_string(),
        );
    }
    if let Some(description) = &profile.description {
        validate_profile_component("description", description)?;
    }
    if let Some(default_schema) = &profile.default_schema {
        validate_profile_component("defaultSchema", default_schema)?;
    }
    Ok(())
}

fn validate_profile_name(name: &str) -> Result<(), String> {
    validate_profile_component("name", name)?;
    if !name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' || ch == ' ')
    {
        return Err(
            "Profile name may contain only letters, numbers, spaces, dash, underscore, and dot."
                .to_string(),
        );
    }
    Ok(())
}

fn validate_profile_component(field: &str, value: &str) -> Result<(), String> {
    if value.contains('\0') {
        return Err(format!("Profile {field} contains an invalid null byte."));
    }
    if contains_postgres_url(value) {
        return Err(format!(
            "Profile {field} must not contain a PostgreSQL URL."
        ));
    }
    Ok(())
}

pub(crate) fn validate_unique_profile_names(profiles: &[ConnectionProfile]) -> Result<(), String> {
    let mut names = BTreeSet::new();
    for profile in profiles {
        if !names.insert(profile.name.to_ascii_lowercase()) {
            return Err(format!(
                "Connection profile '{}' already exists.",
                profile.name
            ));
        }
    }
    Ok(())
}

fn reject_secret_fields(value: &Value) -> Result<(), String> {
    match value {
        Value::Mapping(mapping) => {
            for (key, value) in mapping {
                if let Value::String(key) = key {
                    if is_forbidden_profile_field(key) {
                        return Err(format!(
                            "Connection profiles must not include secret field '{}'.",
                            safe_field_name(key)
                        ));
                    }
                }
                reject_secret_fields(value)?;
            }
        }
        Value::Sequence(values) => {
            for value in values {
                reject_secret_fields(value)?;
            }
        }
        Value::String(value) if contains_postgres_url(value) => {
            return Err("Connection profiles must not contain PostgreSQL URLs.".to_string());
        }
        _ => {}
    }
    Ok(())
}

fn is_forbidden_profile_field(key: &str) -> bool {
    FORBIDDEN_PROFILE_FIELDS
        .iter()
        .any(|field| key.eq_ignore_ascii_case(field))
}

fn safe_field_name(key: &str) -> String {
    if is_forbidden_profile_field(key) {
        "<redacted-field>".to_string()
    } else {
        key.to_string()
    }
}

fn contains_postgres_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("postgres://") || lower.contains("postgresql://")
}

fn is_profile_host_url_like(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    lower.starts_with("postgres://")
        || lower.starts_with("postgresql://")
        || lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("ssh://")
}

fn request_i64(request: &Value, key: &str) -> Option<i64> {
    match mapping_get(request, key) {
        Some(Value::Number(value)) => value.as_i64(),
        Some(Value::String(value)) => value.parse::<i64>().ok(),
        _ => None,
    }
}

fn nested_request_string(request: &Value, object_key: &str, key: &str) -> Option<String> {
    mapping_get(request, object_key).and_then(|value| request_string(value, key))
}

pub(crate) fn resolve_service_postgres_connection(
    request: &Value,
) -> Result<Option<ResolvedPostgresConnection>, String> {
    if let Some(url) =
        request_string(request, "postgresUrl").filter(|value| !value.trim().is_empty())
    {
        return Ok(Some(ResolvedPostgresConnection {
            url,
            source: "requestUrl".to_string(),
        }));
    }

    if let Some(profile_name) = nested_request_string(request, "connection", "profileName")
        .filter(|value| !value.trim().is_empty())
    {
        let store = load_connection_profiles()?;
        let Some(profile) = store
            .profiles
            .iter()
            .find(|profile| profile.name.eq_ignore_ascii_case(&profile_name))
        else {
            return Err("Connection profile was not found.".to_string());
        };
        let password = nested_request_string(request, "connection", "password");
        return Ok(Some(ResolvedPostgresConnection {
            url: profile_to_postgres_url(profile, password.as_deref()),
            source: "profile".to_string(),
        }));
    }

    if let Ok(url) = env::var("DBSTATE_POSTGRES_URL") {
        if !url.trim().is_empty() {
            return Ok(Some(ResolvedPostgresConnection {
                url,
                source: "environment".to_string(),
            }));
        }
    }

    Ok(None)
}

fn profile_to_postgres_url(profile: &ConnectionProfile, password: Option<&str>) -> String {
    let mut url = String::new();
    url.push_str("postgres://");
    url.push_str(&percent_encode_url_component(&profile.username));
    if let Some(password) = password.filter(|value| !value.is_empty()) {
        url.push(':');
        url.push_str(&percent_encode_url_component(password));
    }
    url.push('@');
    url.push_str(&profile.host);
    url.push(':');
    write!(url, "{}", profile.port).ok();
    url.push('/');
    url.push_str(&percent_encode_url_component(&profile.database));
    url.push_str("?sslmode=");
    url.push_str(&percent_encode_url_component(&profile.ssl_mode));
    url
}

fn percent_encode_url_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        let ch = *byte as char;
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~') {
            encoded.push(ch);
        } else {
            write!(encoded, "%{byte:02X}").ok();
        }
    }
    encoded
}

fn service_scope_args(
    request: &Value,
    requirement: ScopeRequirement,
    kind: EndpointScopeKind,
) -> Result<Vec<String>, String> {
    let scope = request_string(request, "scope");
    let schema = request_string(request, "schema")
        .or_else(|| request_string_array(request, "schemas").into_iter().next());
    let table = request_string(request, "table")
        .or_else(|| request_string_array(request, "tables").into_iter().next());

    if scope.is_none() && schema.is_none() && table.is_none() {
        return match requirement {
            ScopeRequirement::Optional => Ok(Vec::new()),
            ScopeRequirement::Required => {
                Err("Missing scope selection. Provide scope \"all\", schema, or table.".to_string())
            }
        };
    }

    if let Some(scope) = scope {
        match scope.as_str() {
            "all" => return Ok(vec!["--all".to_string()]),
            "schema" => {
                if kind == EndpointScopeKind::DataCompare {
                    return Err("schema scope is not supported for data-compare. Use scope \"all\" or table.".to_string());
                }
                let schema =
                    schema.ok_or_else(|| "schema scope requires a schema value.".to_string())?;
                return Ok(vec!["--schema".to_string(), schema]);
            }
            "table" => {
                let table =
                    table.ok_or_else(|| "table scope requires a table value.".to_string())?;
                return Ok(vec!["--table".to_string(), table]);
            }
            other => {
                return Err(format!(
                    "Invalid scope '{other}'. Supported scopes are all, schema, and table."
                ));
            }
        }
    }

    if let Some(table) = table {
        Ok(vec!["--table".to_string(), table])
    } else if let Some(schema) = schema {
        if kind == EndpointScopeKind::DataCompare {
            Err(
                "schema scope is not supported for data-compare. Use scope \"all\" or table."
                    .to_string(),
            )
        } else {
            Ok(vec!["--schema".to_string(), schema])
        }
    } else {
        Ok(Vec::new())
    }
}

pub(crate) fn request_string(request: &Value, key: &str) -> Option<String> {
    match mapping_get(request, key) {
        Some(Value::String(value)) => Some(value.to_string()),
        Some(Value::Number(value)) => Some(value.to_string()),
        _ => None,
    }
}

fn request_bool(request: &Value, key: &str) -> Option<bool> {
    match mapping_get(request, key) {
        Some(Value::Bool(value)) => Some(*value),
        _ => None,
    }
}

fn request_string_array(request: &Value, key: &str) -> Vec<String> {
    match mapping_get(request, key) {
        Some(Value::Sequence(values)) => values
            .iter()
            .filter_map(|value| match value {
                Value::String(value) => Some(value.to_string()),
                Value::Number(value) => Some(value.to_string()),
                _ => None,
            })
            .collect(),
        Some(Value::String(value)) => vec![value.to_string()],
        _ => Vec::new(),
    }
}

fn mapping_get<'a>(request: &'a Value, key: &str) -> Option<&'a Value> {
    let Value::Mapping(mapping) = request else {
        return None;
    };
    mapping.get(Value::String(key.to_string()))
}

pub(crate) fn service_error_response(
    status_code: u16,
    command: &str,
    message: &str,
) -> ServiceHttpResponse {
    let mut body = String::new();
    body.push('{');
    write_json_string_field(&mut body, "command", command, true);
    write_json_bool_field(&mut body, "success", false);
    write_json_array_field(&mut body, "warnings", &[]);
    write_json_array_field(&mut body, "errors", &[redact_message(message, "")]);
    body.push('}');
    ServiceHttpResponse {
        status_code,
        content_type: "application/json; charset=utf-8".to_string(),
        body,
    }
}
