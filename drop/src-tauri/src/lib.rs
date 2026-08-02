mod network;

use network::NetworkState;
#[cfg(target_os = "windows")]
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter,
};
use tauri::{AppHandle, Manager};
#[cfg(target_os = "windows")]
use tauri_plugin_autostart::ManagerExt;
#[cfg(target_os = "android")]
use tauri_plugin_content_access::ContentAccessExt;

#[cfg(target_os = "windows")]
fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
    let _ = app.emit("app-foregrounded", ());
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(target_os = "windows")]
    let builder = builder
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_main_window(app);
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--background"])
                .build(),
        );

    builder
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_content_access::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_log::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            network::send_files,
            network::cancel_transfer,
            network::open_received_folder,
            network::request_local_network_access,
            network::get_app_settings,
            network::set_device_name,
            network::set_receive_directory,
            network::forget_device,
            network::respond_to_incoming,
            network::pair_by_code,
            network::pair_from_qr,
            network::get_pairing_payload,
            network::set_background_receive,
            network::scan_pairing_qr,
            network::pick_folder,
            get_autostart_enabled,
            set_autostart_enabled
        ])
        .setup(|app| {
            #[cfg(target_os = "windows")]
            {
                let open_item = MenuItem::with_id(app, "open", "열기", true, None::<&str>)?;
                let quit_item = MenuItem::with_id(app, "quit", "종료", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&open_item, &quit_item])?;
                let mut tray = TrayIconBuilder::new()
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .tooltip("Vesper Drop · 수신 대기 중")
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "open" => show_main_window(app),
                        "quit" => app.exit(0),
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if matches!(
                            event,
                            TrayIconEvent::Click {
                                button: MouseButton::Left,
                                button_state: MouseButtonState::Up,
                                ..
                            }
                        ) {
                            show_main_window(tray.app_handle());
                        }
                    });
                if let Some(icon) = app.default_window_icon() {
                    tray = tray.icon(icon.clone());
                }
                tray.build(app)?;
            }

            let handle = app.handle().clone();
            #[cfg(target_os = "android")]
            let device_name = handle
                .content_access()
                .device_name()
                .ok()
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| "Android".to_owned());
            #[cfg(not(target_os = "android"))]
            let device_name = hostname::get()
                .ok()
                .and_then(|name| name.into_string().ok())
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| "Vesper Drop 기기".to_owned());
            let state = NetworkState::load(&handle, device_name)?;
            app.manage(state.clone());

            let broadcast_state = state.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = network::start_discovery_broadcaster(broadcast_state).await {
                    log::error!("{error}");
                }
            });
            let discovery_handle = handle.clone();
            let discovery_state = state.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) =
                    network::start_discovery_listener(discovery_handle.clone(), discovery_state)
                        .await
                {
                    log::error!("{error}");
                    let _ = tauri::Emitter::emit(&discovery_handle, "network-error", error);
                }
            });
            let server_state = state.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = network::start_tcp_server(handle.clone(), server_state).await {
                    log::error!("{error}");
                    let _ = tauri::Emitter::emit(&handle, "network-error", error);
                }
            });
            #[cfg(target_os = "windows")]
            if std::env::args().any(|argument| argument == "--background") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            #[cfg(target_os = "windows")]
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.app_handle().emit("app-backgrounded", ());
                let _ = window.hide();
            }
            #[cfg(not(target_os = "windows"))]
            let _ = (window, event);
        })
        .run(tauri::generate_context!())
        .expect("error while running Vesper Drop");
}

#[tauri::command]
fn get_autostart_enabled(app: AppHandle) -> Result<bool, String> {
    #[cfg(target_os = "windows")]
    return app
        .autolaunch()
        .is_enabled()
        .map_err(|error| format!("자동 시작 상태를 확인할 수 없습니다: {error}"));
    #[cfg(not(target_os = "windows"))]
    {
        let _ = app;
        Ok(false)
    }
}

#[tauri::command]
fn set_autostart_enabled(app: AppHandle, enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        if enabled {
            app.autolaunch()
                .enable()
                .map_err(|error| format!("자동 시작을 켤 수 없습니다: {error}"))?;
        } else {
            app.autolaunch()
                .disable()
                .map_err(|error| format!("자동 시작을 끌 수 없습니다: {error}"))?;
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = (app, enabled);
    Ok(())
}
