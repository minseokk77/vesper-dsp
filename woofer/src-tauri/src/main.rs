#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio_engine;

#[tauri::command]
fn get_audio_devices() -> Vec<String> {
    audio_engine::get_available_devices()
}

#[tauri::command]
fn get_device_sample_rate(device_name: String) -> Result<u32, String> {
    audio_engine::get_device_sample_rate(&device_name)
}

#[tauri::command]
fn get_device_bit_depth(device_name: String) -> Result<String, String> {
    audio_engine::get_device_bit_depth(&device_name)
}

#[tauri::command]
fn get_device_supported_sample_rates(device_name: String) -> Result<Vec<u32>, String> {
    audio_engine::get_device_supported_sample_rates(&device_name)
}

#[tauri::command]
fn start_sync(
    app: tauri::AppHandle,
    source: String,
    earphone: String,
    speaker: String,
    delay: u32,
    lpf_hz: f32,
    lpf_slope: u32,
    headroom_db: f32,
    earphone_target_sr: Option<u32>,
    speaker_target_sr: Option<u32>,
    earphone_filter: String,
    speaker_filter: String,
) -> Result<(), String> {
    audio_engine::start_audio_sync(
        app,
        &source,
        &earphone,
        &speaker,
        delay,
        lpf_hz,
        lpf_slope,
        headroom_db,
        earphone_target_sr,
        speaker_target_sr,
        &earphone_filter,
        &speaker_filter,
    )
}

#[tauri::command]
fn stop_sync() -> Result<String, String> {
    match audio_engine::stop_audio_sync() {
        Ok(_) => Ok("Sync stopped".to_string()),
        Err(e) => Err(e),
    }
}

#[tauri::command]
fn get_engine_status() -> audio_engine::EngineStatus {
    audio_engine::get_engine_status()
}

#[tauri::command]
fn set_earphone_mute_cmd(muted: bool) {
    audio_engine::set_earphone_mute(muted);
}

#[tauri::command]
fn set_speaker_mute_cmd(muted: bool) {
    audio_engine::set_speaker_mute(muted);
}

#[tauri::command]
fn apply_earphone_eq_profile(profile: audio_engine::EqProfile) {
    audio_engine::update_earphone_eq_profile(profile);
}

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

fn main() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        if shortcut.matches(Modifiers::CONTROL | Modifiers::ALT, Code::KeyW) {
                            let _ = app.emit("toggle-sync", ());
                        }
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .setup(|app| {
            let args: Vec<String> = std::env::args().collect();
            if !args.contains(&"--autostart".to_string()) {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                }
            }

            let ctrl_alt_w = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyW);
            if let Err(e) = app.global_shortcut().register(ctrl_alt_w) {
                eprintln!("Failed to register global shortcut: {}", e);
            }

            let open_i = MenuItem::with_id(app, "open", "열기", true, None::<&str>)?;

            let preset_movie =
                MenuItem::with_id(app, "preset_movie", "Movie 프리셋", true, None::<&str>)?;
            let preset_music =
                MenuItem::with_id(app, "preset_music", "Music 프리셋", true, None::<&str>)?;
            let preset_gaming =
                MenuItem::with_id(app, "preset_gaming", "Gaming 프리셋", true, None::<&str>)?;
            let preset_submenu = Submenu::with_items(
                app,
                "프리셋 로드",
                true,
                &[&preset_movie, &preset_music, &preset_gaming],
            )?;

            let signal_earphone_i = MenuItem::with_id(
                app,
                "signal_earphone",
                "이어폰 신호 보기",
                true,
                None::<&str>,
            )?;
            let signal_speaker_i =
                MenuItem::with_id(app, "signal_speaker", "우퍼 신호 보기", true, None::<&str>)?;
            let signal_submenu = Submenu::with_items(
                app,
                "시그널 모니터링",
                true,
                &[&signal_earphone_i, &signal_speaker_i],
            )?;
            let settings_i = MenuItem::with_id(app, "settings", "환경설정", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "완전 종료", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;

            let menu = Menu::with_items(
                app,
                &[
                    &open_i,
                    &preset_submenu,
                    &signal_submenu,
                    &settings_i,
                    &separator,
                    &quit_i,
                ],
            )?;

            let tray_icon_bytes = include_bytes!("../icons/tray_icon.png");
            let img = image::load_from_memory(tray_icon_bytes)
                .expect("Failed to load tray icon")
                .into_rgba8();
            let width = img.width();
            let height = img.height();
            let tray_image = tauri::image::Image::new_owned(img.into_raw(), width, height);

            tauri::tray::TrayIconBuilder::new()
                .icon(tray_image)
                .tooltip("Vesper Woofer")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                    "preset_movie" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.emit("load-preset", "Movie");
                        }
                    }
                    "preset_music" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.emit("load-preset", "Music");
                        }
                    }
                    "preset_gaming" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.emit("load-preset", "Gaming");
                        }
                    }
                    "signal_earphone" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let is_visible = window.is_visible().unwrap_or(false);
                            let is_minimized = window.is_minimized().unwrap_or(false);
                            if is_visible && !is_minimized {
                                let _ = window.set_focus();
                            } else {
                                let _ = window.emit("open-signal", "earphone");
                            }
                        }
                    }
                    "signal_speaker" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let is_visible = window.is_visible().unwrap_or(false);
                            let is_minimized = window.is_minimized().unwrap_or(false);
                            if is_visible && !is_minimized {
                                let _ = window.set_focus();
                            } else {
                                let _ = window.emit("open-signal", "speaker");
                            }
                        }
                    }
                    "settings" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let is_visible = window.is_visible().unwrap_or(false);
                            let is_minimized = window.is_minimized().unwrap_or(false);
                            if is_visible && !is_minimized {
                                let _ = window.set_focus();
                            } else {
                                let _ = window.emit("open-settings", ());
                            }
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.unminimize();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::Resized(_) => {
                if window.is_minimized().unwrap_or(false) {
                    let _ = window.emit("window-minimized", ());
                }
            }
            // 창 닫기 버튼을 누르면 완전히 종료되지 않고 트레이로 숨기기
            tauri::WindowEvent::CloseRequested { api, .. } => {
                let _ = window.emit("window-minimized", ());
                api.prevent_close(); // 창 닫기 차단
                let _ = window.hide(); // 창을 숨김 (트레이 아이콘만 남음)
            }
            _ => {}
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_audio_devices,
            get_device_sample_rate,
            get_device_bit_depth,
            get_device_supported_sample_rates,
            start_sync,
            stop_sync,
            get_engine_status,
            set_earphone_mute_cmd,
            set_speaker_mute_cmd,
            apply_earphone_eq_profile
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
