pub fn redact_message(message: &str, secret: &str) -> String {
    let redacted = if secret.is_empty() {
        message.to_string()
    } else {
        message.replace(secret, "<redacted>")
    };
    redact_postgres_url(&redacted)
}

pub fn redact_postgres_url(value: &str) -> String {
    let mut output = String::new();
    for token in value.split_whitespace() {
        if token.starts_with("postgres://") || token.starts_with("postgresql://") {
            output.push_str("<redacted>");
        } else {
            if !output.is_empty() {
                output.push(' ');
            }
            output.push_str(token);
        }
    }
    if output.is_empty() && !value.is_empty() {
        "<redacted>".to_string()
    } else {
        output
    }
}
