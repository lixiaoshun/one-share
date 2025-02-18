mod discovery;
mod screen_share;
mod transfer;

use discovery::{Discovery, PeerInfo};
use screen_share::ScreenShare;
use tauri::State;
use tokio::sync::mpsc;
use transfer::FileTransfer;

use std::sync::Mutex;

#[derive(Default)]
pub struct AppState {
    discovery: Mutex<Option<Discovery>>,
    screen_share: Mutex<Option<ScreenShare>>,
}

#[tauri::command]
async fn start_discovery(state: State<'_, AppState>) -> Result<(), String> {
    let (discovery, _rx) = Discovery::new().map_err(|e| e.to_string())?;
    discovery.start_discovery().map_err(|e| e.to_string())?;
    *state.discovery.lock().unwrap() = Some(discovery);
    Ok(())
}

#[tauri::command]
async fn get_peers(state: State<'_, AppState>) -> Result<Vec<PeerInfo>, String> {
    if let Some(discovery) = &*state.discovery.lock().unwrap() {
        Ok(discovery.get_peers())
    } else {
        Err("Discovery service not initialized".to_string())
    }
}

#[tauri::command]
async fn start_screen_share(state: State<'_, AppState>) -> Result<(), String> {
    let screen_share = ScreenShare::new().await.map_err(|e| e.to_string())?;
    screen_share
        .start_sharing()
        .await
        .map_err(|e| e.to_string())?;
    *state.screen_share.lock().unwrap() = Some(screen_share);
    Ok(())
}

#[tauri::command]
async fn stop_screen_share(state: State<'_, AppState>) -> Result<(), String> {
    let screen_share = state.screen_share.lock().unwrap().take();
    if let Some(screen_share) = screen_share {
        screen_share
            .stop_sharing()
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Screen share not initialized".to_string())
    }
}

#[tauri::command]
async fn send_file(path: String, peer: PeerInfo) -> Result<(), String> {
    let (progress_tx, _progress_rx) = mpsc::channel(100);
    let transfer = FileTransfer::new(progress_tx);

    let mut stream =
        std::net::TcpStream::connect((peer.ip, peer.port)).map_err(|e| e.to_string())?;

    transfer
        .send_file(path, &mut stream)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn init_app() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            start_discovery,
            get_peers,
            start_screen_share,
            stop_screen_share,
            send_file
        ])
        .run(tauri::generate_context!())
        .expect("Error while running tauri application");
}
