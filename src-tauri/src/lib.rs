use std::sync::Mutex;

use tauri::{command, AppHandle, Builder, Manager, State, Url, WebviewWindow, Window, WindowEvent};

mod state;
use state::Config;

mod tray;
use tray::Tray;

mod ha_ws;

#[command]
fn save_url(url: &str, config: State<Mutex<Config>>, window: WebviewWindow, app: AppHandle) -> Result<(), String> {
    let mut config = config.lock().unwrap();

    config.update_url(url).map_err(|err| format!("Unable to update config: \n{}", err))?;

    window.close().unwrap();

    if let Some(w) = app.get_webview_window("main") {
        if let Some(url_str) = &config.url {
            if let Ok(url) = Url::parse(url_str) {
                w.navigate(url).ok();
            }
        }
        w.show().ok();
        w.set_focus().ok();
    }

    Ok(())
}


#[command]
fn get_url(config: State<Mutex<Config>>) -> Result<Option<String>, String> {
  let config = config.lock().unwrap();
  Ok(config.url.clone())
}

#[command]
fn save_token(token: &str, config: State<Mutex<Config>>, app: AppHandle) -> Result<(), String> {
    let (url, tok) = {
        let mut config = config.lock().unwrap();
        config.update_token(token).map_err(|e| format!("{e}"))?;
        (config.url.clone(), config.token.clone())
    };

    if let (Some(url), Some(tok)) = (url, tok) {
        tauri::async_runtime::spawn(ha_ws::start(app, url, tok));
    }

    Ok(())
}

#[command]
fn get_token(config: State<Mutex<Config>>) -> Result<Option<String>, String> {
    let config = config.lock().unwrap();
    Ok(config.token.clone())
}

#[derive(serde::Serialize)]
struct Prefs {
    device_name: Option<String>,
    auto_update: bool,
    notifications_enabled: bool,
}

#[command]
fn get_prefs(config: State<Mutex<Config>>) -> Result<Prefs, String> {
    let config = config.lock().unwrap();
    Ok(Prefs {
        device_name: config.device_name.clone(),
        auto_update: config.auto_update,
        notifications_enabled: config.notifications_enabled,
    })
}

#[command]
fn save_prefs(
    device_name: Option<String>,
    auto_update: bool,
    notifications_enabled: bool,
    config: State<Mutex<Config>>,
    window: WebviewWindow,
) -> Result<(), String> {
    let mut config = config.lock().unwrap();
    config
        .update_prefs(device_name, auto_update, notifications_enabled)
        .map_err(|e| format!("{e}"))?;
    window.close().unwrap();
    Ok(())
}

#[command]
async fn check_update_now(app: AppHandle) -> Result<String, String> {
    #[cfg(desktop)]
    {
        use tauri_plugin_updater::UpdaterExt;
        let updater = app.updater().map_err(|e| format!("{e}"))?;
        match updater.check().await.map_err(|e| format!("{e}"))? {
            Some(update) => Ok(format!("Update available: {}", update.version)),
            None => Ok("You are on the latest version.".to_string()),
        }
    }
    #[cfg(not(desktop))]
    Ok("Updates not supported on this platform.".to_string())
}

fn on_window_event(window: &Window, event: &WindowEvent) {
    let app_handle = window.app_handle();
    let config = app_handle.state::<Mutex<Config>>();
    let config = config.lock().unwrap();

    match event {
        WindowEvent::CloseRequested { api, .. } => {
            if window.label() == "main" {
                if config.url.is_some() {
                    api.prevent_close();
                    window.hide().unwrap();
                } else {
                    app_handle.exit(0);
                }
            }
        }
        WindowEvent::Focused(focused) => {
            if !focused && config.url.is_some() && window.label() == "main" {
                window.hide().unwrap();
            }
        }
        _ => {}
    }
}


#[cfg(desktop)]
async fn check_for_updates(app: AppHandle) {
    use tauri_plugin_notification::NotificationExt;
    use tauri_plugin_updater::UpdaterExt;

    let updater = match app.updater() {
        Ok(u) => u,
        Err(_) => return,
    };

    let update = match updater.check().await {
        Ok(Some(u)) => u,
        _ => return,
    };

    let _ = app
        .notification()
        .builder()
        .title("Update available")
        .body(format!("Version {} is downloading and will install on next launch.", update.version).as_str())
        .show();

    if update
        .download_and_install(|_chunk, _total| {}, || {})
        .await
        .is_ok()
    {
        app.restart();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![save_url, get_url, save_token, get_token, get_prefs, save_prefs, check_update_now])
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let _tray = Tray::new(app);

            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_positioner::init())?;

            let config_path = app.path().local_data_dir().unwrap().join("config.toml");
            let config = Config::from_file(config_path.to_str().unwrap()).unwrap();

            if config.url.is_none() {
                if let Some(w) = app.get_webview_window("config") {
                    w.show().ok();
                }
            } else if let Some(url_str) = &config.url {
                if let Ok(url) = Url::parse(url_str) {
                    if let Some(w) = app.get_webview_window("main") {
                        w.navigate(url).ok();
                    }
                }
            }

            // Start HA WebSocket listener if credentials are already configured
            if let (Some(url), Some(token)) = (config.url.clone(), config.token.clone()) {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(ha_ws::start(handle, url, token));
            }

            #[cfg(desktop)]
            if config.auto_update {
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(check_for_updates(handle));
            }

            app.manage(Mutex::new(config));
            Ok(())
        })
        .on_window_event(on_window_event)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
