mod billing;
mod commands;
mod data_source;
mod store;

use commands::{get_summary, get_threads, refresh};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, RunEvent,
};

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = app.set_dock_visibility(true);
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn hide_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
        let _ = app.set_dock_visibility(false);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 启动时窗口隐藏，Dock 图标也隐藏
            let _ = app.set_dock_visibility(false);

            let quit = MenuItem::with_id(app, "quit", "退出 AgentPrism", true, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "打开看板", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            // 拦截关闭事件，改为 hide
            if let Some(window) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                let win = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win.hide();
                        let _ = app_handle.set_dock_visibility(false);
                    }
                });
            }

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "show" => show_main_window(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            if window.is_visible().unwrap_or(false) {
                                hide_main_window(app);
                            } else {
                                show_main_window(app);
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_summary, get_threads, refresh])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let RunEvent::Reopen { has_visible_windows, .. } = event {
                if !has_visible_windows {
                    show_main_window(app);
                }
            }
        });
}
