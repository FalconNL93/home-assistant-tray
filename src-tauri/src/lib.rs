use std::sync::Mutex;

use tauri::{command, AppHandle, Builder, Manager, State, Url, WebviewWindow, Window, WindowEvent};

mod state;
use state::Config;

mod tray;
use tray::Tray;

#[command]
fn save_url(url: &str, config: State<Mutex<Config>>, window: WebviewWindow, app: AppHandle) -> Result<(), String> {
    let mut config = config.lock().unwrap();

    config.update_url(url).map_err(|err| format!("Unable to update config: \n{}", err))?;

    window.close().unwrap();

    app.get_webview_window("main").map(|w| {
        let url = Url::parse(config.url.as_ref().unwrap()).unwrap();
        w.navigate(url).ok();
    });

    Ok(())
}


#[command]
fn get_url(config: State<Mutex<Config>>) -> Result<Option<String>, String> {
  let config = config.lock().unwrap();
  Ok(config.url.clone())
}

fn on_window_event(window: &Window, event: &WindowEvent) {
    let app_handle = window.app_handle();
    let config = app_handle.state::<Mutex<Config>>();
    let config = config.lock().unwrap();

    match event {
        WindowEvent::CloseRequested { api, .. } => {
            if config.url.is_some() {
                api.prevent_close();
                window.hide().unwrap();
            } else {
                app_handle.exit(0);
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


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![save_url, get_url])
        .setup(|app| {
            let _tray = Tray::new(app);

            #[cfg(desktop)]
            {
                app.handle().plugin(tauri_plugin_positioner::init())?;
            }


            let config_path = app.path().local_data_dir().unwrap().join("config.toml");
            let config = Config::from_file(config_path.to_str().unwrap()).unwrap();

            if config.url.is_none() {
                app.get_webview_window("config").map(|w| w.show().ok());
            } else {
                app.get_webview_window("main").map(|w| {
                    let url = Url::parse(config.url.as_ref().unwrap()).unwrap();
                    w.navigate(url).ok();
                });
            }

            app.manage(Mutex::new(config));
            Ok(())
        })
        .on_window_event(on_window_event)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
