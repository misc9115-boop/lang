//! Loopback-only browser integration fixture. Never linked into the desktop app.
//! Uses the real learning core with a disposable SQLite database and draft content.
use chrono::Utc;
use learning_core::{
    development_lesson, read_backup_file, write_new_backup, BackupController, Store,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Read;
use std::{fs, path::PathBuf};
use tiny_http::{Header, Method, Response, Server};

fn field<T: serde::de::DeserializeOwned>(args: &Value, name: &str) -> Result<T, String> {
    serde_json::from_value(args.get(name).ok_or(format!("Missing {name}"))?.clone())
        .map_err(|e| e.to_string())
}
struct TestFiles {
    root: PathBuf,
    controller: BackupController,
    chosen: Option<PathBuf>,
    cancel_next: bool,
}
fn call(
    store: &mut Store,
    files: &mut TestFiles,
    cmd: &str,
    args: &Value,
) -> Result<Value, String> {
    let time = Utc::now();
    let session = || field::<String>(args, "sessionId");
    let exercise = || field::<String>(args, "exerciseId");
    let value = match cmd {
        "test_cancel_next_dialog" => {
            files.cancel_next = true;
            Value::Null
        }
        "test_corrupt_selected_backup" => {
            let selected = files.chosen.as_ref().ok_or("No test export exists")?;
            let bytes = read_backup_file(selected)?;
            let mut data: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            data["body"]["data"]["settings"]["level"] = json!("B2");
            fs::write(selected, data.to_string()).map_err(|e| e.to_string())?;
            Value::Null
        }
        "export_learner_backup" => {
            if files.cancel_next {
                files.cancel_next = false;
                Value::Null
            } else {
                let path = files
                    .root
                    .join(format!("export-{}.langbackup", time.timestamp_micros()));
                write_new_backup(&path, &store.export_backup(time)?)?;
                files.chosen = Some(path.clone());
                json!(path)
            }
        }
        "choose_learner_backup" => {
            files.controller.cancel();
            if files.cancel_next {
                files.cancel_next = false;
                Value::Null
            } else if let Some(path) = &files.chosen {
                json!(files
                    .controller
                    .prepare(store, read_backup_file(path)?, time)?)
            } else {
                Value::Null
            }
        }
        "restore_learner_backup" => json!(files.controller.restore(
            store,
            &field::<String>(args, "token")?,
            &files.root.join("recovery"),
            time
        )?),
        "cancel_backup_import" => {
            files.controller.cancel();
            Value::Null
        }
        "list_recovery_backups" => {
            let directory = files.root.join("recovery");
            let mut records = vec![];
            if directory.exists() {
                for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
                    let entry = entry.map_err(|e| e.to_string())?;
                    let saved_at: chrono::DateTime<Utc> = entry
                        .metadata()
                        .and_then(|m| m.modified())
                        .map_err(|e| e.to_string())?
                        .into();
                    records
                        .push(json!({"id":entry.file_name().to_string_lossy(),"savedAt":saved_at}));
                }
            }
            records.sort_by(|a, b| b["savedAt"].as_str().cmp(&a["savedAt"].as_str()));
            json!(records)
        }
        "preview_recovery_backup" => {
            let id = field::<String>(args, "id")?;
            if !id.starts_with("pre-import-")
                || !id.ends_with(".langbackup")
                || !id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
            {
                return Err("Invalid test recovery filename".into());
            }
            json!(files.controller.prepare(
                store,
                read_backup_file(&files.root.join("recovery").join(id))?,
                time
            )?)
        }
        "app_state" => json!(store.state(time)?),
        "save_settings" => json!(store.save_settings(field(args, "settings")?, time)?),
        "start_lesson" => json!(store.start(&field::<String>(args, "lessonId")?, time)?),
        "save_draft" => {
            store.save_draft(&session()?, &exercise()?, field(args, "answer")?)?;
            Value::Null
        }
        "submit_answer" => json!(store.submit(
            &session()?,
            &exercise()?,
            field(args, "answer")?,
            &field::<String>(args, "requestId")?,
            time
        )?),
        "show_hint" => json!(store.hint(&session()?, &exercise()?)?),
        "reveal_answer" => json!(store.reveal(&session()?, &exercise()?, time)?),
        "advance_session" => json!(store.advance(&session()?, &exercise()?, time)?),
        "report_answer" => {
            store.report_answer(
                &session()?,
                &exercise()?,
                &field::<String>(args, "note")?,
                time,
            )?;
            Value::Null
        }
        _ => return Err("Unsupported test command".into()),
    };
    Ok(value)
}
#[derive(Deserialize)]
struct Request {
    cmd: String,
    args: Value,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.get(1).is_some_and(|v| v == "--seed-native-root") {
        let root = PathBuf::from(
            args.get(2)
                .ok_or("Provide an isolated native-test directory")?,
        );
        if !root.is_absolute() || root.join("seed-source.db").exists() {
            return Err("Native seed needs an absolute, unused test directory".into());
        }
        fs::create_dir_all(root.join("backups"))?;
        let mut source = Store::open(
            &root.join("seed-source.db"),
            vec![development_lesson()?],
            true,
        )?;
        source.save_settings(
            learning_core::Settings {
                level: "A1".into(),
                time_zone: "America/Chicago".into(),
                daily_goal: 10,
                theme: "light".into(),
                reduced_motion: true,
                text_scale: 100,
            },
            Utc::now(),
        )?;
        let session = source.start("dev.a1.cafe", Utc::now())?;
        source.reveal(&session.id, "dev.cafe.choice", Utc::now())?;
        source.advance(&session.id, "dev.cafe.choice", Utc::now())?;
        source.save_draft(
            &session.id,
            "dev.cafe.translation",
            learning_core::Answer::Text("Un ca".into()),
        )?;
        write_new_backup(
            &root.join("backups/pre-import-native-fixture.langbackup"),
            &source.export_backup(Utc::now())?,
        )?;
        println!("Created an unreviewed native-test recovery fixture in isolated storage.");
        return Ok(());
    }
    let directory = tempfile::tempdir()?;
    let mut store = Store::open(
        &directory.path().join("ui-test.db"),
        vec![development_lesson()?],
        true,
    )?;
    let server = Server::http("127.0.0.1:1421").map_err(std::io::Error::other)?;
    let mut files = TestFiles {
        root: directory.path().to_path_buf(),
        controller: BackupController::default(),
        chosen: None,
        cancel_next: false,
    };
    eprintln!("Disposable Rust/SQLite UI driver: http://127.0.0.1:1421");
    for mut request in server.incoming_requests() {
        let response = if request.method() == &Method::Options {
            json!(null)
        } else if request.method() == &Method::Get && request.url() == "/health" {
            json!({"ok":true})
        } else if request.method() == &Method::Post && request.url() == "/invoke" {
            let mut body = String::new();
            match request.as_reader().take(16384).read_to_string(&mut body) {
                Ok(_) => match serde_json::from_str::<Request>(&body) {
                    Ok(input) => match call(&mut store, &mut files, &input.cmd, &input.args) {
                        Ok(value) => json!({"value":value}),
                        Err(error) => json!({"error":error}),
                    },
                    Err(error) => json!({"error":error.to_string()}),
                },
                Err(error) => json!({"error":error.to_string()}),
            }
        } else {
            json!({"error":"Unknown test route"})
        };
        let headers = [
            ("Content-Type", "application/json"),
            ("Access-Control-Allow-Origin", "http://127.0.0.1:1420"),
            ("Access-Control-Allow-Methods", "POST, GET, OPTIONS"),
            ("Access-Control-Allow-Headers", "Content-Type"),
        ];
        let mut response = Response::from_string(response.to_string());
        for (name, value) in headers {
            response.add_header(Header::from_bytes(name, value).unwrap());
        }
        request.respond(response)?;
    }
    Ok(())
}
