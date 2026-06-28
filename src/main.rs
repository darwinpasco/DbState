use std::env;
use std::process::ExitCode;

use dbstate::{run_cli, OutputFormat};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let result = run_cli(&args, env::current_dir().as_deref());

    match result {
        Ok(cli_result) => {
            match cli_result.format {
                OutputFormat::Json => println!("{}", cli_result.report.to_json()),
                OutputFormat::Text => print!("{}", cli_result.report.to_text()),
            }
            ExitCode::from(cli_result.exit_code)
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::from(2)
        }
    }
}
