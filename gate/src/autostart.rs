use crate::error::{GatewayError, Result};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const REG_RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const REG_UNINSTALL_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\vesper_gate";
const APP_NAME: &str = "vesper_gate";
const DISPLAY_NAME: &str = "Vesper Gate (로컬 만능 게이트웨이 & 프록시)";

/// 윈도우 제어판 '프로그램 추가/제거' 및 설정 '설치된 앱'에 정식 앱으로 자동 등록
pub fn register_control_panel() -> Result<()> {
    let current_exe = std::env::current_exe()?;
    let exe_path = current_exe.to_string_lossy();
    let uninstall_cmd = format!("\"{}\" uninstall", exe_path);

    #[cfg(target_os = "windows")]
    {
        let commands = [
            ("DisplayName", DISPLAY_NAME),
            ("DisplayVersion", "0.1.0"),
            ("Publisher", "Vesper Ecosystem"),
            ("UninstallString", &uninstall_cmd),
            ("QuietUninstallString", &uninstall_cmd),
            ("DisplayIcon", &exe_path),
        ];

        for (name, val) in commands {
            let _ = std::process::Command::new("reg")
                .args(["add", REG_UNINSTALL_KEY, "/v", name, "/t", "REG_SZ", "/d", val, "/f"])
                .creation_flags(0x08000000)
                .status();
        }
    }

    Ok(())
}

/// 윈도우 제어판에서 등록 해제
pub fn unregister_control_panel() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("reg")
            .args(["delete", REG_UNINSTALL_KEY, "/f"])
            .creation_flags(0x08000000)
            .status();
    }
    Ok(())
}

/// 윈도우 부팅 시 자동 시작 등록 (레지스트리 Run 키에 등록)
pub fn enable_autostart() -> Result<()> {
    let current_exe = std::env::current_exe()?;
    let exe_path = current_exe.to_string_lossy();
    let reg_value = format!("\"{}\" start -d --no-browser", exe_path);

    #[cfg(target_os = "windows")]
    {
        let status = std::process::Command::new("reg")
            .args(["add", REG_RUN_KEY, "/v", APP_NAME, "/t", "REG_SZ", "/d", &reg_value, "/f"])
            .creation_flags(0x08000000)
            .status()
            .map_err(|e| GatewayError::ConfigError(format!("레지스트리 등록 실패: {}", e)))?;

        if !status.success() {
            return Err(GatewayError::ConfigError("레지스트리 자동 시작 등록에 실패했습니다.".to_string()));
        }
    }

    Ok(())
}

/// 윈도우 부팅 시 자동 시작 해제
pub fn disable_autostart() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let status = std::process::Command::new("reg")
            .args(["delete", REG_RUN_KEY, "/v", APP_NAME, "/f"])
            .creation_flags(0x08000000)
            .status()
            .map_err(|e| GatewayError::ConfigError(format!("레지스트리 삭제 실패: {}", e)))?;

        if !status.success() {
            return Err(GatewayError::ConfigError("자동 시작이 등록되어 있지 않거나 삭제에 실패했습니다.".to_string()));
        }
    }

    Ok(())
}

/// 현재 자동 시작 등록 상태 확인
pub fn is_autostart_enabled() -> bool {
    #[cfg(target_os = "windows")]
    {
        let output = std::process::Command::new("reg")
            .args(["query", REG_RUN_KEY, "/v", APP_NAME])
            .creation_flags(0x08000000)
            .output();

        if let Ok(out) = output {
            return out.status.success();
        }
    }
    false
}
