//! Developer/host tool. No download, extraction, approval or learner-data writes.
use learning_core::{
    content::ContentTrust,
    pack_library::{LibraryMode, PackLibrary},
};
use std::{env, ffi::OsString, path::Path, process::ExitCode};

const USAGE: &str = "Usage: pack_library [--development] <catalogue.sqlite3> install <pack-folder> | list | active | verify <pack-id> <version> | activate <pack-id> <version> | deactivate <pack-id> | remove <pack-id> <version>";
fn run(args: &[OsString]) -> Result<serde_json::Value, (u8, String)> {
    let usage = || (2, USAGE.to_string());
    let (mode, args) = if args.first().is_some_and(|a| a == "--development") {
        (LibraryMode::Development, &args[1..])
    } else {
        (LibraryMode::ReviewedOnly, args)
    };
    let command = args.get(1).and_then(|a| a.to_str()).ok_or_else(usage)?;
    let count = match command {
        "list" | "active" => 2,
        "install" | "deactivate" => 3,
        "verify" | "activate" | "remove" => 4,
        _ => return Err(usage()),
    };
    if args.len() != count {
        return Err(usage());
    }
    let id = if matches!(command, "verify" | "activate" | "remove" | "deactivate") {
        args[2].to_str().ok_or_else(usage)?
    } else {
        ""
    };
    let version = if count == 4 {
        args[3]
            .to_str()
            .and_then(|v| v.parse::<u32>().ok())
            .filter(|v| *v > 0)
            .ok_or_else(usage)?
    } else {
        0
    };
    let path = Path::new(&args[0]);
    if command != "install" && !path.is_file() {
        return Err((1, "The selected pack catalogue does not exist.".into()));
    }
    let fail = |e| (1, e);
    let mut library = PackLibrary::open(path, ContentTrust::default(), mode).map_err(fail)?;
    let output = match command {
        "install" => serde_json::to_value(
            library
                .install_directory(Path::new(&args[2]))
                .map_err(fail)?,
        ),
        "list" => serde_json::to_value(library.entries().map_err(fail)?),
        "verify" => serde_json::to_value(library.verify(id, version).map_err(fail)?.report),
        "active" => serde_json::to_value(
            library
                .active_lessons()
                .map_err(fail)?
                .iter()
                .map(|l| l.summary())
                .collect::<Vec<_>>(),
        ),
        "activate" => {
            library.activate(id, version).map_err(fail)?;
            Ok(serde_json::json!({"activated":id,"version":version}))
        }
        "deactivate" => {
            library.deactivate(id).map_err(fail)?;
            Ok(serde_json::json!({"deactivated":id}))
        }
        "remove" => {
            library.remove_version(id, version).map_err(fail)?;
            Ok(serde_json::json!({"removedPayloads":id,"version":version}))
        }
        _ => unreachable!(),
    };
    output.map_err(|e| (1, e.to_string()))
}
fn main() -> ExitCode {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    match run(&args) {
        Ok(value) => {
            println!("{value}");
            eprintln!("Offline text library only. No editorial approval, course completeness or learner progress is certified.");
            ExitCode::SUCCESS
        }
        Err((code, message)) => {
            eprintln!("{message}");
            ExitCode::from(code)
        }
    }
}
