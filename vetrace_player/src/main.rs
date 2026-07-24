use std::io;
use std::process::ExitCode;

use vetrace_player::{
    EXIT_SUCCESS, EXIT_USAGE, HELP, ParseOutcome, parse_env, run, write_error_diagnostic,
};

fn main() -> ExitCode {
    match parse_env() {
        Ok(ParseOutcome::Help) => {
            print!("{HELP}");
            ExitCode::from(EXIT_SUCCESS)
        }
        Ok(ParseOutcome::Version) => {
            println!("vetrace-player {}", env!("CARGO_PKG_VERSION"));
            ExitCode::from(EXIT_SUCCESS)
        }
        Ok(ParseOutcome::RuntimeInfo) => {
            println!(
                "{{\"protocol\":1,\"player_version\":\"{}\",\"capabilities\":[\"screen_ui_text_v1\",\"lua_ui_buttons_v1\",\"lua_project_modules_v1\",\"lua_physics_activation_v1\"]}}",
                env!("CARGO_PKG_VERSION")
            );
            ExitCode::from(EXIT_SUCCESS)
        }
        Ok(ParseOutcome::Run(args)) => {
            let mut output = io::stdout();
            let mut diagnostics = io::stderr();
            match run(args, &mut output, &mut diagnostics) {
                Ok(()) => ExitCode::from(EXIT_SUCCESS),
                Err(error) => {
                    let _ = write_error_diagnostic(&error, &mut diagnostics);
                    ExitCode::from(error.exit_code())
                }
            }
        }
        Err(error) => {
            eprintln!("error: {error}\n\nUse --help to see valid options.");
            ExitCode::from(EXIT_USAGE)
        }
    }
}
