use std::sync::{Mutex, PoisonError};

use tauri::{AppHandle, Manager, RunEvent, Window, WindowEvent};
use telai_stream::{DEFAULT_PORT, Hub, Server};

use crate::live::LiveSession;
use crate::{assets, commands, live, tray};

pub const MAIN_WINDOW: &str = "main";

/// Origins of the app window, allowed to open the preview stream. The Vite
/// dev server serves the window in development builds only.
#[cfg(not(debug_assertions))]
const APP_ORIGINS: &[&str] = &["http://tauri.localhost"];
#[cfg(debug_assertions)]
const APP_ORIGINS: &[&str] = &["http://tauri.localhost", "http://localhost:5173"];

pub struct AppState {
    pub hub: Hub,
    pub port: u16,
    pub url: String,
    /// Why the player server is not running, when it is not.
    pub server_error: Mutex<Option<String>>,
    pub live: Mutex<Option<LiveSession>>,
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_main_window(app);
        }))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(tauri_plugin_log::log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            setup(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::status,
            commands::list_windows,
            commands::start,
            commands::stop,
            commands::open_player,
            commands::retry_server,
        ])
        .on_window_event(hide_instead_of_closing_while_live)
        .build(tauri::generate_context!())
        .expect("the Tauri application starts")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                live::stop(app);
            }
        });
}

pub fn show_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };

    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

fn setup(app: &AppHandle) -> tauri::Result<()> {
    app.manage(AppState {
        hub: Hub::new(),
        port: DEFAULT_PORT,
        url: format!("http://127.0.0.1:{DEFAULT_PORT}"),
        server_error: Mutex::new(None),
        live: Mutex::new(None),
    });
    start_server(app);

    tray::create(app)
}

/// Binds the player server and runs it in the background. A failure is kept
/// in the state, where the window reads it and offers to try again.
pub fn start_server(app: &AppHandle) {
    let state = app.state::<AppState>();
    let error = match tauri::async_runtime::block_on(Server::bind(state.port)) {
        Ok(server) => {
            let hub = state.hub.clone();
            let assets = assets::source(app.clone());
            tauri::async_runtime::spawn(async move {
                if let Err(error) = server.serve(hub, assets, APP_ORIGINS).await {
                    tracing::error!(%error, "the player server stopped");
                }
            });

            None
        }
        Err(error) => {
            tracing::error!(%error, port = state.port, "the player server could not start");
            Some(error.to_string())
        }
    };

    *state
        .server_error
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = error;
}

/// Closing the window while streaming keeps the stream running in the tray.
/// When idle, closing quits.
fn hide_instead_of_closing_while_live(window: &Window, event: &WindowEvent) {
    let WindowEvent::CloseRequested { api, .. } = event else {
        return;
    };

    if live::is_live(window.app_handle()) {
        api.prevent_close();
        let _ = window.hide();
    }
}
