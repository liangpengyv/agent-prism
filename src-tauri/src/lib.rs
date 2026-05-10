mod billing;
mod commands;
mod data_source;
mod store;

use commands::{get_summary, get_threads, refresh, get_by_project, get_by_model, get_by_date, get_budget, set_budget, get_prices, set_prices, reset_prices, get_last_selected_agent, set_last_selected_agent, get_app_version, check_update};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    ActivationPolicy, Emitter, Manager, RunEvent,
};

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = app.set_activation_policy(ActivationPolicy::Regular);
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn hide_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
        let _ = app.set_activation_policy(ActivationPolicy::Accessory);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 启动时无可见窗口，切为 Accessory（Dock 图标隐藏）
            let _ = app.set_activation_policy(ActivationPolicy::Accessory);

            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let show = MenuItem::with_id(app, "show", "打开看板", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;

            // 后台定时轮询：每 30s 推送 data-updated 事件
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(30));
                    let _ = app_handle.emit("data-updated", ());
                }
            });

            // 拦截关闭事件，改为 hide + 切换 Dock 策略
            if let Some(window) = app.get_webview_window("main") {
                let app_handle = app.handle().clone();
                let win = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win.hide();
                        let _ = app_handle.set_activation_policy(ActivationPolicy::Accessory);
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
        .invoke_handler(tauri::generate_handler![
            get_summary, get_threads, refresh,
            get_by_project, get_by_model, get_by_date,
            get_budget, set_budget,
            get_prices, set_prices, reset_prices,
            get_last_selected_agent, set_last_selected_agent,
            get_app_version, check_update
        ])
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
