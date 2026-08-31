#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio_engine;
mod apo_manager;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

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
#[allow(clippy::too_many_arguments)]
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
    apo_manager::sync_apo_eq_profile(profile.enabled, profile.preamp_gain as f32, &profile.bands);
    audio_engine::update_output_eq_profile(profile);
}

#[tauri::command]
fn is_apo_active() -> bool {
    apo_manager::is_apo_active()
}

#[tauri::command]
fn auto_bind_device(device_name: String) -> Result<String, String> {
    apo_manager::auto_bind_device_apo(&device_name)
}

#[tauri::command]
fn get_stream_info() -> Option<audio_engine::StreamInfo> {
    audio_engine::get_current_stream_info()
}

#[tauri::command]
fn get_engine_status() -> audio_engine::EngineStatus {
    audio_engine::get_engine_status()
}

#[tauri::command]
fn get_factory_presets() -> Vec<audio_engine::DspFactoryPreset> {
    audio_engine::get_native_factory_presets()
}

#[tauri::command]
fn blink_tray_icon(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn(async move {
        if let Some(tray) = app.tray_by_id("main_tray") {
            let transparent = tauri::image::Image::new_owned(vec![0u8; 32 * 32 * 4], 32, 32);
            let _ = tray.set_icon(Some(transparent));
            
            std::thread::sleep(std::time::Duration::from_millis(300));
            
            let tray_icon_bytes = include_bytes!("../icons/tray_icon.png");
            if let Ok(image) = image::load_from_memory(tray_icon_bytes) {
                let rgba = image.into_rgba8();
                let tray_image = tauri::image::Image::new_owned(
                    rgba.clone().into_raw(),
                    rgba.width(),
                    rgba.height(),
                );
                let _ = tray.set_icon(Some(tray_image));
            }
        }
    });
    Ok(())
}

// Rust-side window spawn removed in favor of JS frontend spawn to prevent thread deadlocks.

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
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

            let main_i = MenuItem::with_id(app, "main", "메인 창 열기", true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let quit_i = MenuItem::with_id(app, "quit", "Vesper DSP 종료", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&main_i, &sep1, &quit_i])?;

            let tray_icon_bytes = include_bytes!("../icons/tray_icon.png");
            let image = image::load_from_memory(tray_icon_bytes)
                .map_err(|e| format!("Failed to decode tray icon: {e}"))?;
            let rgba = image.into_rgba8();
            let tray_image =
                tauri::image::Image::new_owned(rgba.clone().into_raw(), rgba.width(), rgba.height());

            let _tray = tauri::tray::TrayIconBuilder::with_id("main_tray")
                .icon(tray_image)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "main" => {
                        show_main_window(app);
                    }
                    "quit" => {
                        app.exit(0);
                    }
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
                            let is_visible = window.is_visible().unwrap_or(false);
                            let is_minimized = window.is_minimized().unwrap_or(false);

                            if !is_visible || is_minimized {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            } else {
                                let _ = window.hide();
                            }
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
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
            get_stream_info,
            get_engine_status,
            get_factory_presets,
            is_apo_active,
            auto_bind_device,
            blink_tray_icon
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vesper DSP");
}

#[cfg(windows)]
fn elevate_process_priority() {
    use windows::Win32::System::Threading::{
        GetCurrentProcess, SetPriorityClass, HIGH_PRIORITY_CLASS,
    };
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
