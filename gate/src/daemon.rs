use std::fs;
use std::path::PathBuf;
use crate::error::{GatewayError, Result};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const PID_FILE_NAME: &str = "pgate.pid";

/// PID 파일 경로 가져오기 (~/.pgate/pgate.pid)
pub fn pid_file_path() -> Result<PathBuf> {
    let home_dir = dirs::home_dir().ok_or_else(|| {
        GatewayError::ConfigError("사용자 홈 디렉토리를 찾을 수 없습니다.".to_string())
    })?;
    let pgate_dir = home_dir.join(".pgate");
    if !pgate_dir.exists() {
        fs::create_dir_all(&pgate_dir)?;
    }
    Ok(pgate_dir.join(PID_FILE_NAME))
}

/// 현재 실행 중인 프로세스의 PID 저장
pub fn save_pid(pid: u32) -> Result<()> {
    let path = pid_file_path()?;
    fs::write(path, pid.to_string())?;
    Ok(())
}

/// 저장된 PID 읽기
pub fn get_saved_pid() -> Result<Option<u32>> {
    let path = pid_file_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)?;
    if let Ok(pid) = content.trim().parse::<u32>() {
        Ok(Some(pid))
    } else {
        Ok(None)
    }
}

/// PID 파일 제거
pub fn remove_pid_file() {
    if let Ok(path) = pid_file_path() {
        let _ = fs::remove_file(path);
    }
}

/// 백그라운드로 pgate 프로세스 분리 실행 (창 없이 실행)
pub fn spawn_daemon(port: Option<u16>, host: Option<String>, no_browser: bool) -> Result<u32> {
    let current_exe = std::env::current_exe()?;
    let mut cmd = std::process::Command::new(current_exe);
    
    cmd.arg("start");
    if let Some(p) = port {
        cmd.args(["--port", &p.to_string()]);
    }
    if let Some(h) = host {
        cmd.args(["--host", &h]);
    }
    if no_browser {
        cmd.arg("--no-browser");
    }

    // Windows 콘솔 창 완전 숨김 (CREATE_NO_WINDOW = 0x08000000)
    #[cfg(target_os = "windows")]
    {
        cmd.creation_flags(0x08000000);
    }

    let child = cmd.spawn().map_err(|e| {
        GatewayError::NetworkError(format!("백그라운드 프로세스 생성 실패: {}", e))
    })?;

    let pid = child.id();
    save_pid(pid)?;
    Ok(pid)
}

/// 백그라운드에서 실행 중인 pgate 프로세스 종료
pub fn stop_daemon() -> Result<bool> {
    if let Some(pid) = get_saved_pid()? {
        #[cfg(target_os = "windows")]
        {
            let status = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .creation_flags(0x08000000)
                .status();
            
            remove_pid_file();
            return Ok(status.map(|s| s.success()).unwrap_or(false));
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = std::process::Command::new("kill")
                .args(["-9", &pid.to_string()])
                .status();
            remove_pid_file();
            return Ok(true);
        }
    }
    Ok(false)
}
