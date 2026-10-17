#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(all(feature = "native-test", not(debug_assertions)))]
compile_error!("The native test server must never be included in a release build.");
mod desktop_backups;
mod desktop_packs;

use chrono::Utc;
use learning_core::{Answer, AppState, SessionView, Settings, Store, StudyWorkspace};
use std::sync::Mutex;
use tauri::{Manager, State};

struct Database(Mutex<StudyWorkspace>);
struct ProfileLock {
    _file: std::fs::File,
}
type Result<T> = std::result::Result<T, String>;

fn with_store<T>(
    database: State<'_, Database>,
    action: impl FnOnce(&mut StudyWorkspace) -> Result<T>,
) -> Result<T> {
    let mut store = database
        .0
        .lock()
        .map_err(|_| "Local storage is unavailable. Restart the application.".to_string())?;
    action(&mut store)
}

#[tauri::command]
async fn app_state(app: tauri::AppHandle) -> Result<AppState> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Database>()
            .0
            .lock()
            .map_err(|_| "Local storage unavailable.".to_string())?
            .state(Utc::now())
    })
    .await
    .map_err(|_| "Content checks could not finish.".to_string())?
}
#[tauri::command]
fn save_settings(database: State<'_, Database>, settings: Settings) -> Result<AppState> {
    with_store(database, |store| store.save_settings(settings, Utc::now()))
}
#[tauri::command]
async fn start_lesson(app: tauri::AppHandle, lesson_id: String) -> Result<SessionView> {
    tauri::async_runtime::spawn_blocking(move || {
        let database = app.state::<Database>();
        let mut store = database
            .0
            .lock()
            .map_err(|_| "Local storage unavailable.".to_string())?;
        store.pack_state();
        store.start(&lesson_id, Utc::now())
    })
    .await
    .map_err(|_| "Content checks could not finish.".to_string())?
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

#[tauri::command]
fn review_overview(
    database: State<'_, Database>,
) -> Result<learning_core::review_session::ReviewOverview> {
    with_store(database, |s| {
        s.pack_state();
        s.review_overview(Utc::now())
    })
}
#[tauri::command]
fn start_review(
    database: State<'_, Database>,
    practice: bool,
) -> Result<learning_core::review_session::ReviewSessionView> {
    with_store(database, |s| {
        s.pack_state();
        s.start_review(practice, Utc::now())
    })
}
#[tauri::command]
fn review_draft(
    database: State<'_, Database>,
    session_id: String,
    exercise_id: String,
    answer: Answer,
) -> Result<learning_core::review_session::ReviewSessionView> {
    with_store(database, |s| {
        s.review_draft(&session_id, &exercise_id, answer, Utc::now())
    })
}
#[tauri::command]
fn review_submit(
    database: State<'_, Database>,
    session_id: String,
    exercise_id: String,
    answer: Answer,
    request_id: String,
) -> Result<learning_core::review_session::ReviewSessionView> {
    with_store(database, |s| {
        s.review_submit(&session_id, &exercise_id, answer, &request_id, Utc::now())
    })
}
#[tauri::command]
fn review_hint(
    database: State<'_, Database>,
    session_id: String,
    exercise_id: String,
) -> Result<learning_core::review_session::ReviewSessionView> {
    with_store(database, |s| {
        s.review_hint(&session_id, &exercise_id, Utc::now())
    })
}
#[tauri::command]
fn review_reveal(
    database: State<'_, Database>,
    session_id: String,
    exercise_id: String,
) -> Result<learning_core::review_session::ReviewSessionView> {
    with_store(database, |s| {
        s.review_reveal(&session_id, &exercise_id, Utc::now())
    })
}
#[tauri::command]
fn advance_review(
    database: State<'_, Database>,
    session_id: String,
    exercise_id: String,
    easy: bool,
) -> Result<learning_core::review_session::ReviewSessionView> {
    with_store(database, |s| {
        s.advance_review(&session_id, &exercise_id, easy, Utc::now())
    })
}
#[tauri::command]
fn abort_review(
    database: State<'_, Database>,
    session_id: String,
) -> Result<learning_core::review_session::ReviewSessionView> {
    with_store(database, |s| s.abort_review(&session_id, Utc::now()))
}

fn main() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_dialog::init());
    #[cfg(feature = "native-test")]
    let builder = builder
        .plugin(
            tauri::plugin::Builder::<_, ()>::new("native-test-before")
                .on_webview_ready(|view| {
                    eprintln!(
                        "native-test: webview created before driver hook: {}",
                        view.label()
                    )
                })
                .build(),
        )
        .plugin(tauri_plugin_wdio_webdriver::init())
        .plugin(
            tauri::plugin::Builder::<_, ()>::new("native-test-after")
                .on_webview_ready(|view| {
                    eprintln!("native-test: driver hook finished: {}", view.label())
                })
                .build(),
        )
        .on_page_load(|view, payload| {
            eprintln!("native-test: page {:?}: {}", payload.event(), view.label())
        });
    let context = tauri::generate_context!();
    #[cfg(feature = "native-test")]
    let mut context = context;
    #[cfg(feature = "native-test")]
    {
        let directory = std::env::var_os("LANG_NATIVE_TEST_DATA_DIR")
            .expect("Native tests require an isolated data directory");
        let directory = std::path::PathBuf::from(directory);
        assert!(
            directory.is_absolute(),
            "Native test data directory must be absolute"
        );
        // WKWebView ignores data_directory; ephemeral webview storage also keeps
        // native tests isolated on macOS. Learner SQLite remains in the test root.
        context.config_mut().app.windows[0].incognito = true;
        #[cfg(not(target_os = "macos"))]
        {
            context.config_mut().app.windows[0].data_directory = Some(directory.join("webview"));
        }
        let window = &context.config().app.windows[0];
        eprintln!(
            "native-test: entering event loop; window={}, create={}, visible={}",
            window.label, window.create, window.visible
        );
    }
    builder
        .setup(|app| {
            #[cfg(feature = "native-test")]
            eprintln!("native-test: application setup entered");
            #[cfg(not(feature = "native-test"))]
            let directory = app.path().app_local_data_dir()?;
            #[cfg(feature = "native-test")]
            let directory = std::path::PathBuf::from(
                std::env::var_os("LANG_NATIVE_TEST_DATA_DIR")
                    .expect("Native tests require isolated storage"),
            );
            std::fs::create_dir_all(&directory)?;
            let development = cfg!(debug_assertions);
            let profile_lock = learning_core::lock_profile(&directory.join(if development {
                "development.lock"
            } else {
                "learner.lock"
            }))
            .map_err(std::io::Error::other)?;
            app.manage(ProfileLock {
                _file: profile_lock,
            });
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
            let library = learning_core::pack_library::PackLibrary::open(
                &directory.join(if development {
                    "development-packs.sqlite3"
                } else {
                    "packs.sqlite3"
                }),
                learning_core::content::ContentTrust::default(),
                if development {
                    learning_core::pack_library::LibraryMode::Development
                } else {
                    learning_core::pack_library::LibraryMode::ReviewedOnly
                },
            );
            let mut workspace = StudyWorkspace::new(store, library);
            workspace
                .migrate_schema(&directory.join("backups"), Utc::now())
                .map_err(std::io::Error::other)?;
            app.manage(Database(Mutex::new(workspace)));
            app.manage(desktop_backups::BackupServices {
                controller: Mutex::new(learning_core::BackupController::default()),
                recovery_directory: directory.join("backups"),
            });
            #[cfg(feature = "native-test")]
            eprintln!("native-test: local storage and commands ready");
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
            review_overview,
            start_review,
            review_draft,
            review_submit,
            review_hint,
            review_reveal,
            advance_review,
            abort_review,
            report_answer,
            desktop_backups::export_learner_backup,
            desktop_backups::choose_learner_backup,
            desktop_backups::restore_learner_backup,
            desktop_backups::cancel_backup_import,
            desktop_backups::list_recovery_backups,
            desktop_backups::preview_recovery_backup,
            desktop_packs::pack_state,
            desktop_packs::choose_pack_folder,
            desktop_packs::install_pack,
            desktop_packs::cancel_pack_install,
            desktop_packs::change_pack
        ])
        .run(context)
        .expect("The desktop application could not start");
}
