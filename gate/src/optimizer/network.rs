use crate::error::{GatewayError, Result};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// 윈도우 TCP/IP 네트워크 스택 No-Lag 최적화 적용 (정부/은행 사이트 DNS는 건드리지 않음)
pub fn apply_tcp_no_lag() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let ps_script = r#"
# 1. 윈도우 네트워크 대역폭 쓰로틀링 해제 및 반응성 극대화
$sysProfile = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile'
Set-ItemProperty -Path $sysProfile -Name 'NetworkThrottlingIndex' -Value 0xFFFFFFFF -Type DWord -Force
Set-ItemProperty -Path $sysProfile -Name 'SystemResponsiveness' -Value 0 -Type DWord -Force

# 2. 모든 활성 네트워크 인터페이스에 Nagle 알고리즘 해제 & 즉시 응답(TcpAckFrequency) 적용
$interfacesPath = 'HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces'
Get-ChildItem $interfacesPath | ForEach-Object {
    $path = $_.PSPath
    Set-ItemProperty -Path $path -Name 'TcpAckFrequency' -Value 1 -Type DWord -Force -ErrorAction SilentlyContinue
    Set-ItemProperty -Path $path -Name 'TCPNoDelay' -Value 1 -Type DWord -Force -ErrorAction SilentlyContinue
    Set-ItemProperty -Path $path -Name 'TcpDelAckTicks' -Value 0 -Type DWord -Force -ErrorAction SilentlyContinue
}

# 3. TCP 수신 버퍼 윈도우 자동 튜닝
netsh int tcp set global autotuninglevel=normal | Out-Null
"#;

        run_elevated_ps(ps_script)?;
    }
    Ok(())
}

/// 윈도우 기본 네트워크 설정으로 복원
pub fn revert_tcp_no_lag() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let ps_script = r#"
$sysProfile = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Multimedia\SystemProfile'
Set-ItemProperty -Path $sysProfile -Name 'NetworkThrottlingIndex' -Value 10 -Type DWord -Force
Set-ItemProperty -Path $sysProfile -Name 'SystemResponsiveness' -Value 20 -Type DWord -Force

$interfacesPath = 'HKLM:\SYSTEM\CurrentControlSet\Services\Tcpip\Parameters\Interfaces'
Get-ChildItem $interfacesPath | ForEach-Object {
    $path = $_.PSPath
    Remove-ItemProperty -Path $path -Name 'TcpAckFrequency' -Force -ErrorAction SilentlyContinue
    Remove-ItemProperty -Path $path -Name 'TCPNoDelay' -Force -ErrorAction SilentlyContinue
    Remove-ItemProperty -Path $path -Name 'TcpDelAckTicks' -Force -ErrorAction SilentlyContinue
}
"#;

        run_elevated_ps(ps_script)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn run_elevated_ps(script: &str) -> Result<()> {
    let temp_script = std::env::temp_dir().join("vgate_net_opt.ps1");
    std::fs::write(&temp_script, script)?;

    let script_path = temp_script.to_string_lossy().to_string();
    let cmd = format!(
        "Start-Process powershell -ArgumentList '-NoProfile -ExecutionPolicy Bypass -File \"{}\"' -Verb RunAs -WindowStyle Hidden -Wait; Remove-Item -Path \"{}\" -Force",
        script_path, script_path
    );

    let status = std::process::Command::new("powershell")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &cmd])
        .creation_flags(0x08000000)
        .status()
        .map_err(|e| GatewayError::ConfigError(format!("관리자 권한 실행 실패: {}", e)))?;

    if !status.success() {
        return Err(GatewayError::ConfigError("네트워크 최적화 적용을 위한 관리자 권한이 승인되지 않았습니다.".to_string()));
    }

    Ok(())
}
