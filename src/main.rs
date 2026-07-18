use std::env;
use std::process::ExitCode;

use dbstate::{run_cli, run_service, service_usage, usage, OutputFormat};

mod commit_message;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if matches!(args.as_slice(), [arg] if arg == "--help" || arg == "-h") {
        println!("{}\n  {}", usage(), commit_message::usage());
        return ExitCode::SUCCESS;
    }
    if matches!(args.as_slice(), [command, help] if command == "commit-message" && (help == "--help" || help == "-h")) {
        println!("{}", commit_message::usage());
        return ExitCode::SUCCESS;
    }
    if matches!(args.first(), Some(command) if command == "commit-message") {
        let cwd = match env::current_dir() {
            Ok(cwd) => cwd,
            Err(error) => {
                eprintln!("Could not read current directory: {error}");
                return ExitCode::from(2);
            }
        };
        return match commit_message::run(&args[1..], &cwd) {
            Ok(result) => {
                match result.format {
                    commit_message::OutputFormat::Text => print!("{}", result.output),
                    commit_message::OutputFormat::Json => println!("{}", result.output),
                }
                ExitCode::from(result.exit_code)
            }
            Err(message) => {
                eprintln!("{message}");
                ExitCode::from(2)
            }
        };
    }
    if matches!(args.as_slice(), [command, help] if command == "serve" && (help == "--help" || help == "-h"))
    {
        println!("{}", service_usage());
        return ExitCode::SUCCESS;
    }
    if matches!(args.first(), Some(command) if command == "serve") {
        let serve_args = &args[1..];
        return match run_service(serve_args, env::current_dir().as_deref()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(message) => {
                eprintln!("{message}");
                ExitCode::from(2)
            }
        };
    }

    let result = run_cli(&args, env::current_dir().as_deref());

    match result {
        Ok(cli_result) => {
            match cli_result.format {
                OutputFormat::Json => println!("{}", cli_result.output.to_json()),
                OutputFormat::Text => print!("{}", cli_result.output.to_text()),
            }
            ExitCode::from(cli_result.exit_code)
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}
