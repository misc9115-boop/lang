//! Loopback-only browser integration fixture. Never linked into the desktop app.
//! Uses the real learning core with a disposable SQLite database and draft content.
use chrono::Utc;
use learning_core::{development_lesson, Store};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::Read;
use tiny_http::{Header, Method, Response, Server};

fn field<T: serde::de::DeserializeOwned>(args: &Value, name: &str) -> Result<T, String> {
    serde_json::from_value(args.get(name).ok_or(format!("Missing {name}"))?.clone())
        .map_err(|e| e.to_string())
}
fn call(store: &mut Store, cmd: &str, args: &Value) -> Result<Value, String> {
    let time = Utc::now();
    let session = || field::<String>(args, "sessionId");
    let exercise = || field::<String>(args, "exerciseId");
    let value = match cmd {
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
    let directory = tempfile::tempdir()?;
    let mut store = Store::open(
        &directory.path().join("ui-test.db"),
        vec![development_lesson()?],
        true,
    )?;
    let server = Server::http("127.0.0.1:1421").map_err(std::io::Error::other)?;
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
                    Ok(input) => match call(&mut store, &input.cmd, &input.args) {
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
