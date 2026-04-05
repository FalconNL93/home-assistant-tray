use tauri::{
  menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem}, 
  tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent}, 
  App, AppHandle, Manager, Runtime
};
use tauri_plugin_positioner::{Position, WindowExt};

pub struct Tray<R: Runtime> {
  _tray: TrayIcon<R>
}

impl<R: Runtime> Tray<R> {
  pub fn new(manager: &App<R>,) -> Result<Self, Box<dyn std::error::Error>> {
    let tray_menu = Self::build_menu(manager)?;

    let tray = Tray { 
      _tray: TrayIconBuilder::new()
      .icon(manager.default_window_icon().unwrap().clone())
      .menu(&tray_menu)
      .on_tray_icon_event(Self::on_tray_icon_event)
      .on_menu_event(Self::on_menu_event)
      .show_menu_on_left_click(false)
      .build(manager)?,
    };

    Ok(tray)
  }

  fn build_menu(manager: &App<R>) -> Result<Menu<R>, Box<dyn std::error::Error>> {
    let quit = MenuItem::with_id(manager, "quit", "Quit", true, None::<&str>)?;
    let config = MenuItem::with_id(manager, "config", "Config", true, None::<&str>)?;
    let prefs = MenuItem::with_id(manager, "prefs", "Preferences", true, None::<&str>)?;
    let show = MenuItem::with_id(manager, "show", "Show", true, None::<&str>)?;

    let menu = Menu::with_items(manager, &[
      &show,
      &config,
      &prefs,
      &PredefinedMenuItem::separator(manager)?,
      &quit,
    ])?;

    Ok(menu)
  }

  fn on_menu_event(app: &AppHandle<R>, event: MenuEvent) {
    match event.id.as_ref() {
      "quit" => {
        app.exit(0);
      }
      "config" => {
        app.get_webview_window("config").map(|w| w.show().ok());
      }
      "prefs" => {
        app.get_webview_window("prefs").map(|w| {
          w.show().ok();
          w.set_focus().ok();
        });
      }
      "show" => {
        app.get_webview_window("main").map(|w| {
          w.show().ok();
          w.set_focus().ok();
          w.move_window(Position::TrayCenter).ok();
        });
      }
      _ => {}
    }
  }

  fn on_tray_icon_event(tray_handle: &TrayIcon<R>, event: TrayIconEvent) {
    tauri_plugin_positioner::on_tray_event(tray_handle.app_handle(), &event);

    match event {
      tauri::tray::TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } => {
        tray_handle.app_handle().get_webview_window("main").map(|w| {
          w.show().ok();
          w.set_focus().ok();
          w.move_window(Position::TrayCenter).ok();
        });
      }
      _ => {}
    }
  }
}