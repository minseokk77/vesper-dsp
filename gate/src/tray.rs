#[cfg(target_os = "windows")]
pub fn run_tray_thread(dashboard_url: String) {
    std::thread::spawn(move || {
        unsafe {
            win32_tray_loop(dashboard_url);
        }
    });
}

#[cfg(not(target_os = "windows"))]
pub fn run_tray_thread(_dashboard_url: String) {}

#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::*;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Shell::*;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use crate::autostart::{enable_autostart, disable_autostart, is_autostart_enabled};

#[cfg(target_os = "windows")]
const WM_TRAY_CALLBACK: u32 = WM_USER + 100;
#[cfg(target_os = "windows")]
const IDM_OPEN_DASHBOARD: usize = 1001;
#[cfg(target_os = "windows")]
const IDM_OPEN_CONFIG: usize = 1002;
#[cfg(target_os = "windows")]
const IDM_TOGGLE_AUTOSTART: usize = 1003;
#[cfg(target_os = "windows")]
const IDM_EXIT: usize = 1004;

#[cfg(target_os = "windows")]
const TRAY_HEARTBEAT_TIMER_ID: usize = 999;

#[cfg(target_os = "windows")]
static mut GLOBAL_DASHBOARD_URL: Option<String> = None;
#[cfg(target_os = "windows")]
static mut WM_TASKBAR_CREATED: u32 = 0;
#[cfg(target_os = "windows")]
static mut GLOBAL_NID: Option<NOTIFYICONDATAW> = None;

#[cfg(target_os = "windows")]
unsafe fn win32_tray_loop(dashboard_url: String) {
    GLOBAL_DASHBOARD_URL = Some(dashboard_url);

    let class_name = wide_str("VesperGateTrayClass");
    let window_name = wide_str("VesperGateTrayWindow");

    let h_instance = windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null());

    let wnd_class = WNDCLASSEXW {
        cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
        style: 0,
        lpfnWndProc: Some(tray_wnd_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: h_instance,
        hIcon: std::ptr::null_mut(),
        hCursor: std::ptr::null_mut(),
        hbrBackground: std::ptr::null_mut(),
        lpszMenuName: std::ptr::null(),
        lpszClassName: class_name.as_ptr(),
        hIconSm: std::ptr::null_mut(),
    };

    RegisterClassExW(&wnd_class);

    // HWND_MESSAGE 대신 숨겨진 최상위 윈도우 생성 (작업 표시줄 재생성 및 브로드캐스트 수신 필수)
    let hwnd = CreateWindowExW(
        0,
        class_name.as_ptr(),
        window_name.as_ptr(),
        WS_POPUP,
        0,
        0, 0, 0,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        h_instance,
        std::ptr::null(),
    );

    if hwnd == std::ptr::null_mut() {
        return;
    }

    // 윈도우 작업 표시줄(Explorer.exe) 재시작/DPI변경 감지 메시지 등록
    let taskbar_created_str = wide_str("TaskbarCreated");
    WM_TASKBAR_CREATED = RegisterWindowMessageW(taskbar_created_str.as_ptr());

    // 임베드된 커스텀 모던 아이콘(리소스 ID 1) 로드, 실패 시 IDI_APPLICATION 폴백
    let mut h_icon = LoadIconW(h_instance, 1 as *const u16);
    if h_icon == std::ptr::null_mut() {
        h_icon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);
    }

    let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY_CALLBACK;
    nid.hIcon = h_icon;

    // 툴팁 설정 ("Vesper Gate (로컬 만능 게이트웨이 & 보안 쉴드)")
    let tip = wide_str("Vesper Gate (로컬 만능 게이트웨이 & 보안 쉴드)");
    let copy_len = tip.len().min(nid.szTip.len() - 1);
    std::ptr::copy_nonoverlapping(tip.as_ptr(), nid.szTip.as_mut_ptr(), copy_len);

    GLOBAL_NID = Some(nid);

    // 트레이 아이콘 최초 등록
    Shell_NotifyIconW(NIM_ADD, &nid);

    // 5초마다 트레이 아이콘 생존 확인 및 자동 복구(Auto-Heal) 타이머 작동
    SetTimer(hwnd, TRAY_HEARTBEAT_TIMER_ID, 5000, None);

    // Win32 메시지 루프
    let mut msg: MSG = std::mem::zeroed();
    while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    KillTimer(hwnd, TRAY_HEARTBEAT_TIMER_ID);
    Shell_NotifyIconW(NIM_DELETE, &nid);
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn tray_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // 1. 작업 표시줄(Explorer.exe) 재시작 시 트레이 아이콘 즉시 자동 재등록
    if WM_TASKBAR_CREATED != 0 && msg == WM_TASKBAR_CREATED {
        if let Some(ref nid) = GLOBAL_NID {
            Shell_NotifyIconW(NIM_ADD, nid);
        }
        return 0;
    }

    match msg {
        WM_TIMER => {
            if wparam == TRAY_HEARTBEAT_TIMER_ID {
                // 5초마다 트레이 아이콘 상태 보존 확인 (MODIFY 시도 후 실패 시 ADD)
                if let Some(ref nid) = GLOBAL_NID {
                    let res = Shell_NotifyIconW(NIM_MODIFY, nid);
                    if res == 0 {
                        Shell_NotifyIconW(NIM_ADD, nid);
                    }
                }
            }
            0
        }
        WM_TRAY_CALLBACK => {
            let event = lparam as u32;
            if event == WM_RBUTTONUP || event == WM_CONTEXTMENU {
                // 우클릭 시 컨텍스트 팝업 메뉴 표시
                let mut pt: POINT = std::mem::zeroed();
                GetCursorPos(&mut pt);

                let hmenu = CreatePopupMenu();
                let autostart_text = if is_autostart_enabled() {
                    wide_str("🔄 부팅 시 자동 시작 (ON)")
                } else {
                    wide_str("🔄 부팅 시 자동 시작 (OFF)")
                };

                let menu_dash = wide_str("🌐 웹 대시보드 열기");
                let menu_cfg = wide_str("⚙️ 설정 파일 열기");
                let menu_exit = wide_str("❌ 게이트웨이 종료");

                AppendMenuW(hmenu, MF_STRING, IDM_OPEN_DASHBOARD, menu_dash.as_ptr());
                AppendMenuW(hmenu, MF_STRING, IDM_OPEN_CONFIG, menu_cfg.as_ptr());
                AppendMenuW(hmenu, MF_STRING, IDM_TOGGLE_AUTOSTART, autostart_text.as_ptr());
                AppendMenuW(hmenu, MF_SEPARATOR, 0, std::ptr::null());
                AppendMenuW(hmenu, MF_STRING, IDM_EXIT, menu_exit.as_ptr());

                SetForegroundWindow(hwnd);
                TrackPopupMenuEx(hmenu, TPM_RIGHTBUTTON | TPM_BOTTOMALIGN, pt.x, pt.y, hwnd, std::ptr::null());
                DestroyMenu(hmenu);
            } else if event == WM_LBUTTONUP || event == WM_LBUTTONDBLCLK {
                // 좌클릭 또는 더블클릭 시 웹 대시보드 열기
                open_dashboard();
            }
            0
        }
        WM_COMMAND => {
            let id = wparam as usize;
            match id {
                IDM_OPEN_DASHBOARD => open_dashboard(),
                IDM_OPEN_CONFIG => open_config_file(),
                IDM_TOGGLE_AUTOSTART => {
                    if is_autostart_enabled() {
                        let _ = disable_autostart();
                    } else {
                        let _ = enable_autostart();
                    }
                }
                IDM_EXIT => {
                    if let Some(ref nid) = GLOBAL_NID {
                        Shell_NotifyIconW(NIM_DELETE, nid);
                    }
                    crate::daemon::remove_pid_file();
                    std::process::exit(0);
                }
                _ => {}
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[cfg(target_os = "windows")]
unsafe fn open_dashboard() {
    if let Some(ref url) = GLOBAL_DASHBOARD_URL {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", url])
            .creation_flags(0x08000000)
            .spawn();
    }
}

#[cfg(target_os = "windows")]
fn open_config_file() {
    if let Ok(path) = crate::config::GatewayConfig::config_path() {
        let _ = std::process::Command::new("notepad")
            .arg(path)
            .creation_flags(0x08000000)
            .spawn();
    }
}

#[cfg(target_os = "windows")]
fn wide_str(s: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    std::ffi::OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}
