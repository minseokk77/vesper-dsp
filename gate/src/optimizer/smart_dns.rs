use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use crate::error::{GatewayError, Result};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const UPSTREAM_CLOUDFLARE: &str = "1.1.1.1:53";
const UPSTREAM_KOREA_KT: &str = "168.126.63.1:53";

/// DNS 응답 램 캐시 엔트리 (TTL 유지)
struct DnsCacheEntry {
    response: Vec<u8>,
    expires_at: Instant,
}

/// 스마트 분기 DNS 서버
pub struct SmartDnsServer {
    cache: Arc<RwLock<HashMap<Vec<u8>, DnsCacheEntry>>>,
}

impl Default for SmartDnsServer {
    fn default() -> Self {
        Self::new()
    }
}

impl SmartDnsServer {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 로컬 127.0.0.1:53 포트에서 DNS 리스너 구동
    pub async fn start(&self) -> Result<()> {
        let socket = match UdpSocket::bind("127.0.0.1:53").await {
            Ok(s) => s,
            Err(e) => {
                return Err(GatewayError::ConfigError(format!("DNS 포트 53 바인딩 실패 (관리자 권한 필요 또는 다른 DNS 서비스 충돌): {}", e)));
            }
        };

        println!("  • 스마트 분기 DNS: 127.0.0.1:53 가동 (1.1.1.1 + 한국 공공/금융 자동 분기)");

        let socket = Arc::new(socket);
        let cache = Arc::clone(&self.cache);

        tokio::spawn(async move {
            let mut buf = vec![0u8; 1024];

            loop {
                match socket.recv_from(&mut buf).await {
                    Ok((len, src_addr)) => {
                        let query_data = buf[..len].to_vec();
                        let socket_clone = Arc::clone(&socket);
                        let cache_clone = Arc::clone(&cache);

                        tokio::spawn(async move {
                            if let Some(resp) = handle_dns_query(query_data, cache_clone).await {
                                let _ = socket_clone.send_to(&resp, src_addr).await;
                            }
                        });
                    }
                    Err(_) => break,
                }
            }
        });

        Ok(())
    }
}

async fn handle_dns_query(
    query: Vec<u8>,
    cache: Arc<RwLock<HashMap<Vec<u8>, DnsCacheEntry>>>,
) -> Option<Vec<u8>> {
    if query.len() < 12 {
        return None;
    }

    let domain = extract_domain_name(&query).unwrap_or_default();
    let lower_domain = domain.to_lowercase();

    // 1. 램 캐시 확인
    {
        let cache_read = cache.read().unwrap();
        if let Some(entry) = cache_read.get(&query[12..]) {
            if Instant::now() < entry.expires_at {
                let mut resp = entry.response.clone();
                // DNS 트랜잭션 ID 복사
                resp[0] = query[0];
                resp[1] = query[1];
                return Some(resp);
            }
        }
    }

    // 2. 스마트 분기 라우팅
    let upstream_addr = if is_korean_public_or_banking(&lower_domain) {
        UPSTREAM_KOREA_KT
    } else {
        UPSTREAM_CLOUDFLARE
    };

    // 3. 업스트림 DNS에 비동기 전달 및 응답 수신
    if let Ok(upstream_socket) = UdpSocket::bind("0.0.0.0:0").await {
        let upstream_target: SocketAddr = upstream_addr.parse().ok()?;
        if upstream_socket.send_to(&query, upstream_target).await.is_ok() {
            let mut resp_buf = vec![0u8; 1024];
            let recv_fut = upstream_socket.recv_from(&mut resp_buf);
            if let Ok(Ok((resp_len, _))) = tokio::time::timeout(Duration::from_millis(1500), recv_fut).await {
                let resp_data = resp_buf[..resp_len].to_vec();

                // 캐시에 60초간 보관
                let mut cache_write = cache.write().unwrap();
                cache_write.insert(
                    query[12..].to_vec(),
                    DnsCacheEntry {
                        response: resp_data.clone(),
                        expires_at: Instant::now() + Duration::from_secs(60),
                    },
                );

                return Some(resp_data);
            }
        }
    }

    None
}

/// 한국 공공기관, 장학재단, 금융, 은행 사이트 판별
fn is_korean_public_or_banking(domain: &str) -> bool {
    let patterns = [
        ".go.kr", ".gov.kr", "kosaf.go.kr", "hometax.go.kr",
        "kbstar.com", "shinhan.com", "wooribank.com", "hanafn.com",
        "ibk.co.kr", "nhbank.com", "kakaobank.com", "tossbank.com",
        "pass.co.kr", "kisa.or.kr", "police.go.kr", ".ac.kr", ".mil.kr"
    ];

    for p in patterns {
        if domain.ends_with(p) || domain.contains(p) {
            return true;
        }
    }
    false
}

/// DNS 패킷에서 질의 도메인 이름 추출
fn extract_domain_name(packet: &[u8]) -> Option<String> {
    let mut pos = 12; // Header는 12바이트
    let mut parts = Vec::new();

    while pos < packet.len() {
        let len = packet[pos] as usize;
        if len == 0 {
            break;
        }
        pos += 1;
        if pos + len > packet.len() {
            return None;
        }
        let part = std::str::from_utf8(&packet[pos..pos + len]).ok()?;
        parts.push(part);
        pos += len;
    }

    Some(parts.join("."))
}

/// 윈도우 시스템 DNS를 127.0.0.1 (스마트 분기 DNS)로 설정
pub fn set_system_dns_smart() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let ps_script = r#"
Get-NetAdapter | Where-Object { $_.Status -eq 'Up' } | ForEach-Object {
    Set-DnsClientServerAddress -InterfaceIndex $_.ifIndex -ServerAddresses ('127.0.0.1', '1.1.1.1') -ErrorAction SilentlyContinue
}
"#;
        run_elevated_ps(ps_script)?;
    }
    Ok(())
}

/// 윈도우 시스템 DNS를 원래의 통신사 자동(DHCP)으로 복원
pub fn reset_system_dns() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        let ps_script = r#"
Get-NetAdapter | Where-Object { $_.Status -eq 'Up' } | ForEach-Object {
    Set-DnsClientServerAddress -InterfaceIndex $_.ifIndex -ResetServerAddresses -ErrorAction SilentlyContinue
}
"#;
        run_elevated_ps(ps_script)?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn run_elevated_ps(script: &str) -> Result<()> {
    let temp_script = std::env::temp_dir().join("vgate_dns_opt.ps1");
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
        return Err(GatewayError::ConfigError("DNS 설정을 위한 관리자 권한이 승인되지 않았습니다.".to_string()));
    }

    Ok(())
}
