use learning_core::content::{validate_pack, ContentTrust};
use std::{env, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if args.len() != 1 {
        eprintln!("Usage: cargo run -p learning-core --example check_pack -- <pack-folder>");
        return ExitCode::from(2);
    }
    match validate_pack(&PathBuf::from(&args[0]), &ContentTrust::default()) {
        Ok(pack) => match serde_json::to_string_pretty(&pack.report) {
            Ok(report) => {
                println!("{report}");
                eprintln!("Structural preflight passed. This does not install content, approve editorial claims, or certify curriculum completeness.");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("Pack rejected: {error}");
            ExitCode::FAILURE
        }
    }
}
