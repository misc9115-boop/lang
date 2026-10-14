#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Utc;
use learning_core::{Answer, AppState, SessionView, Settings, Store};
use std::sync::Mutex;
use tauri::{Manager, State};

struct Database(Mutex<Store>);
type Result<T> = std::result::Result<T, String>;

fn with_store<T>(
    database: State<'_, Database>,
    action: impl FnOnce(&mut Store) -> Result<T>,
) -> Result<T> {
    let mut store = database
        .0
        .lock()
        .map_err(|_| "Local storage is unavailable. Restart the application.".to_string())?;
    action(&mut store)
}

#[tauri::command]
fn app_state(database: State<'_, Database>) -> Result<AppState> {
    with_store(database, |store| store.state(Utc::now()))
}
#[tauri::command]
fn save_settings(database: State<'_, Database>, settings: Settings) -> Result<AppState> {
    with_store(database, |store| store.save_settings(settings, Utc::now()))
}
#[tauri::command]
fn start_lesson(database: State<'_, Database>, lesson_id: String) -> Result<SessionView> {
    with_store(database, |store| store.start(&lesson_id, Utc::now()))
}
#[tauri::command]
fn save_draft(
    database: State<'_, Database>,
    session_id: String,
    exercise_id: String,
    answer: Answer,
) -> Result<()> {
    with_store(database, |store| {
        store.save_draft(&session_id, &exercise_id, answer)
    })
}
#[tauri::command]
fn submit_answer(
    database: State<'_, Database>,
    session_id: String,
    exercise_id: String,
    answer: Answer,
    request_id: String,
) -> Result<SessionView> {
    with_store(database, |store| {
        store.submit(&session_id, &exercise_id, answer, &request_id, Utc::now())
    })
}
#[tauri::command]
fn show_hint(
    database: State<'_, Database>,
    session_id: String,
    exercise_id: String,
) -> Result<SessionView> {
    with_store(database, |store| store.hint(&session_id, &exercise_id))
}
#[tauri::command]
fn reveal_answer(
    database: State<'_, Database>,
    session_id: String,
    exercise_id: String,
) -> Result<SessionView> {
    with_store(database, |store| {
        store.reveal(&session_id, &exercise_id, Utc::now())
    })
}
#[tauri::command]
fn advance_session(
    database: State<'_, Database>,
    session_id: String,
    exercise_id: String,
) -> Result<SessionView> {
    with_store(database, |store| {
        store.advance(&session_id, &exercise_id, Utc::now())
    })
}
#[tauri::command]
fn report_answer(
    database: State<'_, Database>,
    session_id: String,
    exercise_id: String,
    note: String,
) -> Result<()> {
    with_store(database, |store| {
        store.report_answer(&session_id, &exercise_id, &note, Utc::now())
    })
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let directory = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&directory)?;
            let development = cfg!(debug_assertions);
            #[cfg(debug_assertions)]
            let lessons = vec![learning_core::development_lesson().map_err(std::io::Error::other)?];
            #[cfg(not(debug_assertions))]
            let lessons = vec![];
            let filename = if development {
                "development.sqlite3"
            } else {
                "learner.sqlite3"
            };
            let store = Store::open(&directory.join(filename), lessons, development)
                .map_err(std::io::Error::other)?;
            app.manage(Database(Mutex::new(store)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_state,
            save_settings,
            start_lesson,
            save_draft,
            submit_answer,
            show_hint,
            reveal_answer,
            advance_session,
            report_answer
        ])
        .run(tauri::generate_context!())
        .expect("The desktop application could not start");
}
