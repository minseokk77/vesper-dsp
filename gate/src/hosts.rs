use std::fs;
use std::path::Path;
use crate::error::{GatewayError, Result};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const HOSTS_PATH: &str = r"C:\Windows\System32\drivers\etc\hosts";
const SECTION_START: &str = "# >>> Vesper Gate Domains (Auto-Generated) >>>";
const SECTION_END: &str = "# <<< Vesper Gate Domains (Auto-Generated) <<<";

/// 윈도우 hosts 파일에 등록된 도메인 목록 동기화
pub fn sync_hosts_file(domains: &[String]) -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let hosts_path = Path::new(HOSTS_PATH);
        if !hosts_path.exists() {
            return Err(GatewayError::ConfigError("hosts 파일을 찾을 수 없습니다.".to_string()));
        }

        let current_content = fs::read_to_string(hosts_path).unwrap_or_default();
        let new_content = build_hosts_content(&current_content, domains);

        // 1. 일반 권한으로 직접 쓰기 시도
        if let Ok(()) = fs::write(hosts_path, &new_content) {
            return Ok(());
        }

        // 2. 권한 부족 시 임시 파일 생성 후 관리자 권한(UAC RunAs)으로 자동 덮어쓰기
        let temp_hosts = std::env::temp_dir().join("vesper_gate_hosts.tmp");
        fs::write(&temp_hosts, &new_content)?;

        let temp_path_str = temp_hosts.to_string_lossy().to_string();
        let cmd = format!(
            "Copy-Item -Path '{}' -Destination '{}' -Force; Remove-Item -Path '{}' -Force",
            temp_path_str, HOSTS_PATH, temp_path_str
        );

        let status = std::process::Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &format!("Start-Process powershell -ArgumentList '-NoProfile -Command {}' -Verb RunAs -WindowStyle Hidden -Wait", cmd)])
            .creation_flags(0x08000000)
            .status()
            .map_err(|e| GatewayError::ConfigError(format!("UAC 관리자 권한 실행 실패: {}", e)))?;

        if !status.success() {
            return Err(GatewayError::ConfigError("hosts 파일 수정 관리자 권한이 승인되지 않았습니다.".to_string()));
        }
    }

    Ok(())
}

fn build_hosts_content(original: &str, domains: &[String]) -> String {
    let mut cleaned_lines = Vec::new();
    let mut in_vesper_section = false;

    for line in original.lines() {
        if line.trim() == SECTION_START {
            in_vesper_section = true;
            continue;
        }
        if line.trim() == SECTION_END {
            in_vesper_section = false;
            continue;
        }
        if !in_vesper_section {
            cleaned_lines.push(line);
        }
    }

    let mut result = cleaned_lines.join("\r\n");
    if !result.ends_with("\r\n") && !result.is_empty() {
        result.push_str("\r\n");
    }

    if !domains.is_empty() {
        result.push_str(SECTION_START);
        result.push_str("\r\n");
        for domain in domains {
            if !domain.contains("127.0.0.1") && !domain.contains("localhost") {
                result.push_str(&format!("127.0.0.1 {}\r\n", domain));
            }
        }
        result.push_str(SECTION_END);
        result.push_str("\r\n");
    }

    result
}
