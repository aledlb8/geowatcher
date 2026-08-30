//! Tauri wiring: state, commands, tray, and the update pump.

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, State, WindowEvent};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

use crate::config::Settings;
use crate::player;
use crate::supervisor::{Supervisor, Update};
use crate::twitch::TwitchClient;

/// Event name the frontend listens on.
const UPDATE_EVENT: &str = "watcher://update";

/// Shared application state.
#[derive(Debug)]
pub struct AppState {
    settings: Mutex<Settings>,
    supervisor: Mutex<Supervisor>,
    settings_path: PathBuf,
    /// Most recent stream URL per channel, for opening a viewer on click.
    streams: Mutex<std::collections::HashMap<String, String>>,
}

impl AppState {
    fn save(&self) -> Result<(), String> {
        let settings = self.settings.lock().map_err(|_| "settings lock poisoned")?;
        settings
            .save(&self.settings_path)
            .map_err(|err| err.to_string())
    }

    /// Restarts watchers so a settings change takes effect.
    fn resync(&self) -> Result<(), String> {
        let settings = self.settings.lock().map_err(|_| "settings lock poisoned")?;
        let mut supervisor = self
            .supervisor
            .lock()
            .map_err(|_| "supervisor lock poisoned")?;

        supervisor.stop_all();
        for channel in settings.channels.iter().filter(|c| c.enabled) {
            supervisor.watch(channel, &settings);
        }
        Ok(())
    }
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<Settings, String> {
    state
        .settings
        .lock()
        .map(|s| s.clone())
        .map_err(|_| "settings lock poisoned".to_owned())
}

#[tauri::command]
fn add_channel(login: String, state: State<'_, AppState>) -> Result<bool, String> {
    let added = {
        let mut settings = state.settings.lock().map_err(|_| "lock poisoned")?;
        settings.add_channel(&login)
    };
    if added {
        state.save()?;
        state.resync()?;
    }
    Ok(added)
}

#[tauri::command]
fn remove_channel(login: String, state: State<'_, AppState>) -> Result<bool, String> {
    let removed = {
        let mut settings = state.settings.lock().map_err(|_| "lock poisoned")?;
        settings.remove_channel(&login)
    };
    if removed {
        state.save()?;
        state.resync()?;
    }
    Ok(removed)
}

#[tauri::command]
fn set_threshold(
    login: Option<String>,
    percent: f64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut settings = state.settings.lock().map_err(|_| "lock poisoned")?;
        match login {
            // No channel means the global default.
            None => settings.alert_at = percent,
            Some(login) => {
                let channel = settings
                    .channel_mut(&login)
                    .ok_or_else(|| format!("no such channel: {login}"))?;
                channel.alert_at = Some(percent);
            }
        }
    }
    state.save()?;
    state.resync()
}

#[tauri::command]
fn set_enabled(login: String, enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    {
        let mut settings = state.settings.lock().map_err(|_| "lock poisoned")?;
        let channel = settings
            .channel_mut(&login)
            .ok_or_else(|| format!("no such channel: {login}"))?;
        channel.enabled = enabled;
    }
    state.save()?;
    state.resync()
}

/// Opens a channel in mpv, or the browser when mpv is unavailable.
#[tauri::command]
fn open_channel(login: String, state: State<'_, AppState>) -> Result<String, String> {
    let mpv_path = {
        let settings = state.settings.lock().map_err(|_| "lock poisoned")?;
        settings.mpv_path.clone()
    };
    let url = state
        .streams
        .lock()
        .map_err(|_| "lock poisoned")?
        .get(&login)
        .cloned();

    player::open(&login, url.as_deref(), &mpv_path)
        .map(|opened| format!("{opened:?}").to_lowercase())
        .map_err(|err| err.to_string())
}

/// Channels currently live in the Geometry Dash directory.
#[tauri::command]
async fn discover(limit: Option<u32>) -> Result<Vec<serde_json::Value>, String> {
    let client = TwitchClient::new().map_err(|err| err.to_string())?;
    let channels = client
        .live_channels_for_game("Geometry Dash", limit.unwrap_or(20))
        .await
        .map_err(|err| err.to_string())?;

    Ok(channels
        .into_iter()
        .map(|c| serde_json::json!({ "login": c.login, "viewers": c.viewers }))
        .collect())
}

/// Forwards watcher updates to the frontend, raising notifications on alerts.
async fn pump(app: AppHandle, mut updates: UnboundedReceiver<Update>) {
    while let Some(update) = updates.recv().await {
        if let Update::Alert {
            login,
            value,
            stream_url,
        } = &update
        {
            remember_stream(&app, login, stream_url.as_deref());
            notify(&app, login, *value);
        }
        let _ = app.emit(UPDATE_EVENT, &update);
    }
}

fn remember_stream(app: &AppHandle, login: &str, url: Option<&str>) {
    let Some(url) = url else { return };
    let Some(state) = app.try_state::<AppState>() else {
        return;
    };
    let Ok(mut streams) = state.streams.lock() else {
        return;
    };
    streams.insert(login.to_owned(), url.to_owned());
}

fn notify(app: &AppHandle, login: &str, value: f64) {
    let result = app
        .notification()
        .builder()
        .title(format!("{login} — {value:.2}%"))
        .body("Real attempt. Click to watch.")
        .show();

    if let Err(err) = result {
        tracing::warn!("could not show notification: {err}");
    }
}

/// Tray icon with a menu, so the app keeps watching with no window open.
fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Show GeoWatcher", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(
            app.default_window_icon().cloned().ok_or_else(|| {
                tauri::Error::AssetNotFound("default window icon missing".to_owned())
            })?,
        )
        .tooltip("GeoWatcher")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => reveal(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn reveal(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Builds and runs the application.
///
/// # Panics
///
/// Panics if the webview or the platform event loop cannot be initialised.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "geowatcher=info".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            get_settings,
            add_channel,
            remove_channel,
            set_threshold,
            set_enabled,
            open_channel,
            discover,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            let settings_path = crate::config::default_path(&app.path().app_config_dir()?);

            let settings = Settings::load(&settings_path).unwrap_or_else(|err| {
                // Never discard a channel list because of a bad parse; start
                // empty for this session and leave the file untouched.
                tracing::error!("could not load settings: {err}");
                Settings::default()
            });

            let (tx, rx) = unbounded_channel();
            app.manage(AppState {
                settings: Mutex::new(settings),
                supervisor: Mutex::new(Supervisor::new(tx)),
                settings_path,
                streams: Mutex::new(std::collections::HashMap::new()),
            });

            build_tray(&handle)?;
            tauri::async_runtime::spawn(pump(handle.clone(), rx));

            // Watchers are started on the async runtime, where tokio::spawn works.
            tauri::async_runtime::spawn(async move {
                if let Some(state) = handle.try_state::<AppState>() {
                    if let Err(err) = state.resync() {
                        tracing::error!("could not start watchers: {err}");
                    }
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing hides to the tray; quitting is done from the tray menu.
            if matches!(event, WindowEvent::CloseRequested { .. }) {
                let _ = window.hide();
                if let Some(api) = as_close_api(event) {
                    api.prevent_close();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Borrows the close API out of a window event.
fn as_close_api(event: &WindowEvent) -> Option<&tauri::CloseRequestApi> {
    match event {
        WindowEvent::CloseRequested { api, .. } => Some(api),
        _ => None,
    }
}
