use crate::Database;
use learning_core::{PackState, PackTicket};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;
type Result<T> = std::result::Result<T, String>;
async fn background<T: Send + 'static>(
    action: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T> {
    tauri::async_runtime::spawn_blocking(action)
        .await
        .map_err(|_| "The pack operation could not finish.".to_string())?
}
#[tauri::command]
pub async fn pack_state(app: AppHandle) -> Result<PackState> {
    background(move || {
        Ok(app
            .state::<Database>()
            .0
            .lock()
            .map_err(|_| "Local storage unavailable.")?
            .pack_state())
    })
    .await
}
#[tauri::command]
pub async fn choose_pack_folder(app: AppHandle) -> Result<Option<PackTicket>> {
    background(move || {
        app.state::<Database>()
            .0
            .lock()
            .map_err(|_| "Local storage unavailable.")?
            .cancel_pack();
        let Some(folder) = app.dialog().file().blocking_pick_folder() else {
            return Ok(None);
        };
        let path = folder
            .into_path()
            .map_err(|_| "Choose a local pack folder.")?;
        app.state::<Database>()
            .0
            .lock()
            .map_err(|_| "Local storage unavailable.")?
            .prepare_pack(&path)
            .map(Some)
    })
    .await
}
#[tauri::command]
pub async fn install_pack(app: AppHandle, token: String) -> Result<PackState> {
    background(move || {
        app.state::<Database>()
            .0
            .lock()
            .map_err(|_| "Local storage unavailable.")?
            .install_pack(&token)
    })
    .await
}
#[tauri::command]
pub async fn cancel_pack_install(app: AppHandle) -> Result<()> {
    background(move || {
        app.state::<Database>()
            .0
            .lock()
            .map_err(|_| "Local storage unavailable.")?
            .cancel_pack();
        Ok(())
    })
    .await
}
#[tauri::command]
pub async fn change_pack(
    app: AppHandle,
    action: String,
    pack_id: String,
    version: u32,
) -> Result<PackState> {
    background(move || {
        app.state::<Database>()
            .0
            .lock()
            .map_err(|_| "Local storage unavailable.")?
            .change_pack(&action, &pack_id, version)
    })
    .await
}
