#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio_engine;

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// 작업 스케줄러 작업 이름
const TASK_NAME: &str = "VesperDSP";
/// CREATE_NO_WINDOW 플래그 (콘솔 창 없이 프로세스 실행)
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Windows 작업 스케줄러에 VesperDSP를 Priority=1(HIGH) 로 등록합니다.
/// 일반 레지스트리 Run 키보다 CPU 우선순위가 높아 부팅 직후 더 빠르게 초기화됩니다.
#[tauri::command]
fn enable_priority_autostart() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let exe = std::env::current_exe()
            .map_err(|e| format!("실행 파일 경로 오류: {e}"))?;
        let exe_str = exe.to_string_lossy().to_string();

        // 현재 로그인된 사용자 계정 조회
        let who = std::process::Command::new("whoami")
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| e.to_string())?;
        let username = String::from_utf8_lossy(&who.stdout).trim().to_string();

        // Priority=1 → HIGH_PRIORITY_CLASS (부팅 후 즉시 CPU 우선 선점)
        let xml = format!(
r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{username}</UserId>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{username}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>1</Priority>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{exe_str}</Command>
      <Arguments>--autostart</Arguments>
    </Exec>
  </Actions>
</Task>"#
        );

        // schtasks는 UTF-16 LE BOM 인코딩 XML 요구
        let temp_path = std::env::temp_dir().join("VesperDSP_task.xml");
        let utf16_bytes: Vec<u8> = [0xFF, 0xFE]
            .iter()
            .copied()
            .chain(xml.encode_utf16().flat_map(|c| c.to_le_bytes()))
            .collect();
        std::fs::write(&temp_path, utf16_bytes)
            .map_err(|e| format!("임시 XML 파일 생성 실패: {e}"))?;

        let result = std::process::Command::new("schtasks")
            .args(["/create", "/tn", TASK_NAME, "/xml", temp_path.to_str().unwrap(), "/f"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        let _ = std::fs::remove_file(&temp_path);

        let out = result.map_err(|e| format!("schtasks 실행 실패: {e}"))?;
        if !out.status.success() {
            return Err(format!("작업 스케줄러 등록 실패: {}", String::from_utf8_lossy(&out.stderr)));
        }
    }
    Ok(())
}

/// 작업 스케줄러에서 VesperDSP 자동 시작 작업을 제거합니다.
#[tauri::command]
fn disable_priority_autostart() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("schtasks")
            .args(["/delete", "/tn", TASK_NAME, "/f"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        // 작업이 없는 경우에도 에러 무시
    }
    Ok(())
}

/// 작업 스케줄러에 VesperDSP 자동 시작 작업이 등록되어 있는지 확인합니다.
#[tauri::command]
fn is_priority_autostart_enabled() -> bool {
    #[cfg(target_os = "windows")]
    {
        return std::process::Command::new("schtasks")
            .args(["/query", "/tn", TASK_NAME])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
    }
    #[allow(unreachable_code)]
    false
}

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
    let args: Vec<String> = std::env::args().collect();
    let is_autostart = args.contains(&"--autostart".to_string());

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
        .plugin(if is_autostart {
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(tauri_plugin_window_state::StateFlags::all() ^ tauri_plugin_window_state::StateFlags::VISIBLE)
                .build()
        } else {
            tauri_plugin_window_state::Builder::default().build()
        })
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(move |app| {
            if let Some(window) = app.get_webview_window("main") {
                if is_autostart {
                    // window_state 플러그인이 이전 가시 상태를 복원할 수 있으므로
                    // 자동 시작 시에는 명시적으로 숨김 처리
                    let _ = window.hide();
                } else {
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
            get_stream_info,
            enable_priority_autostart,
            disable_priority_autostart,
            is_priority_autostart_enabled
        ])
        .run(tauri::generate_context!())
        .expect("error while running Vesper DSP");
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
