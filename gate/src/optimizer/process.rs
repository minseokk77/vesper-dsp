#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, SetPriorityClass, HIGH_PRIORITY_CLASS,
};

/// 현재 Vesper Gate 프로세스를 윈도우 스케줄러 최우선 순위(High Priority)로 승격
pub fn boost_process_priority() {
    #[cfg(target_os = "windows")]
    unsafe {
        let handle = GetCurrentProcess();
        let _ = SetPriorityClass(handle, HIGH_PRIORITY_CLASS);
    }
}
