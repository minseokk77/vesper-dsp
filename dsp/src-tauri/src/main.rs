#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio_engine;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[tauri::command]
fn get_audio_devices(is_asio: bool) -> Vec<String> {
    audio_engine::get_available_devices(is_asio)
}

#[tauri::command]
fn get_source_devices(is_asio: bool) -> Vec<String> {
    audio_engine::get_source_devices(is_asio)
}

#[tauri::command]
fn get_output_devices(is_asio: bool) -> Vec<String> {
    audio_engine::get_output_devices(is_asio)
}


#[tauri::command]
fn get_device_sample_rate(device_name: String, is_asio: bool) -> Result<u32, String> {
    audio_engine::get_device_sample_rate(&device_name, is_asio)
}

#[tauri::command]
fn get_device_bit_depth(device_name: String, is_asio: bool) -> Result<String, String> {
    audio_engine::get_device_bit_depth(&device_name, is_asio)
}

#[tauri::command]
fn get_device_supported_bit_depths(
    device_name: String,
    is_asio: bool,
) -> Result<Vec<audio_engine::BitDepthOption>, String> {
    audio_engine::get_device_supported_bit_depths(&device_name, is_asio)
}

#[tauri::command]
fn get_device_supported_sample_rates(
    device_name: String,
    is_asio: bool,
) -> Result<Vec<u32>, String> {
    audio_engine::get_device_supported_sample_rates(&device_name, is_asio)
}

#[tauri::command]
fn start_dsp(
    app: tauri::AppHandle,
    source: String,
    output: String,
    is_asio: bool,
    headroom_db: f32,
    target_sample_rate: Option<u32>,
    filter_type: String,
    output_sample_format: Option<String>,
) -> Result<(), String> {
    audio_engine::start_dsp_engine(
        app,
        &source,
        &output,
        is_asio,
        headroom_db,
        target_sample_rate,
        &filter_type,
        output_sample_format.as_deref(),
    )
}

#[tauri::command]
fn stop_dsp() -> Result<String, String> {
    audio_engine::stop_dsp_engine().map(|_| "DSP stopped".to_string())
}

#[tauri::command]
fn set_mute(muted: bool) {
    audio_engine::set_output_mute(muted);
}

#[tauri::command]
fn apply_output_eq_profile(profile: audio_engine::EqProfile) {
    audio_engine::update_output_eq_profile(profile);
}

#[tauri::command]
fn get_stream_info() -> Option<audio_engine::StreamInfo> {
    audio_engine::get_current_stream_info()
}

// Rust-side window spawn removed in favor of JS frontend spawn to prevent thread deadlocks.

fn main() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state == ShortcutState::Pressed
                        && shortcut.matches(Modifiers::CONTROL | Modifiers::ALT, Code::KeyW)
                    {
                        let _ = app.emit("toggle-sync", ());
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_process::init())
        .plugin(if std::env::args().any(|arg| arg == "--autostart") {
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(tauri_plugin_window_state::StateFlags::all() ^ tauri_plugin_window_state::StateFlags::VISIBLE)
                .build()
        } else {
            tauri_plugin_window_state::Builder::default().build()
        })
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .setup(|app| {
            let args: Vec<String> = std::env::args().collect();
            if args.contains(&"--autostart".to_string()) {
                #[cfg(windows)]
                elevate_process_priority();
            } else {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                }
            }

            let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyW);
            if let Err(error) = app.global_shortcut().register(shortcut) {
                eprintln!("Failed to register global shortcut: {error}");
            }

            let open = MenuItem::with_id(app, "open", "열기", true, None::<&str>)?;
            let signal = MenuItem::with_id(app, "signal", "시그널 경로 보기", true, None::<&str>)?;
            let preset_movie =
                MenuItem::with_id(app, "preset_movie", "Movie 프리셋", true, None::<&str>)?;
            let preset_music =
                MenuItem::with_id(app, "preset_music", "Music 프리셋", true, None::<&str>)?;
            let preset_gaming =
                MenuItem::with_id(app, "preset_gaming", "Gaming 프리셋", true, None::<&str>)?;
            let presets = Submenu::with_items(
                app,
                "프리셋",
                true,
                &[&preset_movie, &preset_music, &preset_gaming],
            )?;
            let settings = MenuItem::with_id(app, "settings", "환경설정", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "완전 종료", true, None::<&str>)?;
            let separator = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(
                app,
                &[&open, &presets, &signal, &settings, &separator, &quit],
            )?;

            let tray_icon_bytes = include_bytes!("../icons/tray_icon.png");
            let image = image::load_from_memory(tray_icon_bytes)
                .expect("Failed to load tray icon")
                .into_rgba8();
            let tray_image = tauri::image::Image::new_owned(
                image.clone().into_raw(),
                image.width(),
                image.height(),
            );

            tauri::tray::TrayIconBuilder::new()
                .icon(tray_image)
                .tooltip("Vesper DSP")
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_main_window(app),
                    "signal" => {
                        show_main_window(app);
                        let _ = app.emit("open-signal", ());
                    }
                    "preset_movie" => emit_main(app, "load-preset", "Movie"),
                    "preset_music" => emit_main(app, "load-preset", "Music"),
                    "preset_gaming" => emit_main(app, "load-preset", "Gaming"),
                    "settings" => {
                        show_main_window(app);
                        let _ = app.emit("open-settings", ());
                    }
                    "quit" => std::process::exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if matches!(
                        event,
                        tauri::tray::TrayIconEvent::Click {
                            button: tauri::tray::MouseButton::Left,
                            button_state: tauri::tray::MouseButtonState::Up,
                            ..
                        }
                    ) {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_audio_devices,
            get_source_devices,
            get_output_devices,
            get_device_sample_rate,
            get_device_bit_depth,
            get_device_supported_bit_depths,
            get_device_supported_sample_rates,
            start_dsp,
            stop_dsp,
            set_mute,
            apply_output_eq_profile,
            get_stream_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vesper DSP");
}

#[cfg(windows)]
fn elevate_process_priority() {
    use windows::Win32::System::Threading::{GetCurrentProcess, SetPriorityClass, HIGH_PRIORITY_CLASS};
    unsafe {
        let _ = SetPriorityClass(GetCurrentProcess(), HIGH_PRIORITY_CLASS);
    }
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn emit_main<S: serde::Serialize + Clone>(app: &tauri::AppHandle, event: &str, payload: S) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit(event, payload);
    }
}
