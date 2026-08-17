use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};

/// 탐지된 보안 위협 이벤트
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub id: usize,
    pub timestamp: String,
    pub client_ip: String,
    pub method: String,
    pub path: String,
    pub threat_type: String,
    pub severity: String, // "CRITICAL", "HIGH", "MEDIUM"
    pub action: String,   // "차단됨 (403 Forbidden)", "감시 기록됨"
}

/// 최근 100건의 보안 위협 로그를 메모리에 유지하는 링 버퍼
#[derive(Clone)]
pub struct SecurityLogBuffer {
    buffer: Arc<RwLock<VecDeque<SecurityEvent>>>,
    counter: Arc<RwLock<usize>>,
}

impl Default for SecurityLogBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl SecurityLogBuffer {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
            counter: Arc::new(RwLock::new(1)),
        }
    }

    pub fn record(&self, mut event: SecurityEvent) {
        let mut count = self.counter.write().unwrap();
        event.id = *count;
        *count += 1;

        let mut buf = self.buffer.write().unwrap();
        if buf.len() >= 100 {
            buf.pop_back();
        }
        buf.push_front(event);
    }

    pub fn get_recent_events(&self) -> Vec<SecurityEvent> {
        let buf = self.buffer.read().unwrap();
        buf.iter().cloned().collect()
    }

    pub fn clear(&self) {
        let mut buf = self.buffer.write().unwrap();
        buf.clear();
    }
}

/// HTTP 요청의 L7 패턴을 정밀 검사하여 수상한 접근 여부 판정
pub fn inspect_threat(
    method: &str,
    path: &str,
    query: &str,
    user_agent: &str,
    client_ip: &str,
) -> Option<SecurityEvent> {
    let lower_path = path.to_lowercase();
    let lower_query = query.to_lowercase();
    let lower_ua = user_agent.to_lowercase();

    // 1. 민감 파일 탈취 및 백도어 취약점 스캔 (CRITICAL)
    let sensitive_files = [
        "/.env", "/.git", "/.svn", "/.htaccess", "/.aws", "/config.json", "/wp-config.php",
        "/id_rsa", "/backup.sql", "/dump.sql", "/database.sql", "/credentials", "/web.config"
    ];
    for s in sensitive_files {
        if lower_path.contains(s) {
            return Some(create_event(
                client_ip, method, path,
                "🚨 민감 설정/비밀키 파일 탈취 스캔",
                "CRITICAL",
            ));
        }
    }

    // 2. 알려진 관리자 페이지 및 원격 실행(RCE) 스캐닝 (HIGH)
    let admin_scan = [
        "/wp-admin", "/wp-login", "/phpmyadmin", "/actuator", "/console/",
        "/shell", "/eval-stdin", "/solr/", "/jmx-console", "/autodiscover",
        "/telescope", "/telescope/requests"
    ];
    for s in admin_scan {
        if lower_path.contains(s) {
            return Some(create_event(
                client_ip, method, path,
                "⚠️ 비인가 관리자/쉘 접근 스캐닝",
                "HIGH",
            ));
        }
    }

    // 3. SQL Injection 공격 구문 (CRITICAL)
    let sqli_patterns = [
        "' or ", "\" or ", "' union select", "\" union select", " sleep(",
        " benchmarking(", "1=1", "1=2", "'--", "';--", "exec(", "waitfor delay"
    ];
    for s in sqli_patterns {
        if lower_path.contains(s) || lower_query.contains(s) {
            return Some(create_event(
                client_ip, method, path,
                "💉 SQL Injection 데이터베이스 공격 시도",
                "CRITICAL",
            ));
        }
    }

    // 4. 경로 조작 (Path Traversal) 공격 (CRITICAL)
    if lower_path.contains("../") || lower_path.contains("..\\") || lower_query.contains("../") || lower_query.contains("..%2f") || lower_query.contains("..%5c") {
        return Some(create_event(
            client_ip, method, path,
            "📂 상위 디렉토리(Path Traversal) 탈출 시도",
            "CRITICAL",
        ));
    }

    // 5. 악성 해킹 툴 User-Agent 탐지 (HIGH)
    let hacker_tools = [
        "sqlmap", "nikto", "masscan", "dirbuster", "gobuster", "zgrab", "nmap", "wprecon", "acunetix"
    ];
    for tool in hacker_tools {
        if lower_ua.contains(tool) {
            return Some(create_event(
                client_ip, method, path,
                &format!("🤖 자동화 해킹 스캐너 봇 ({})", tool),
                "HIGH",
            ));
        }
    }

    // 6. XSS (크로스 사이트 스크립팅) 공격 (MEDIUM)
    let xss_patterns = ["<script", "javascript:", "onerror=", "onload=", "document.cookie"];
    for s in xss_patterns {
        if lower_query.contains(s) || lower_path.contains(s) {
            return Some(create_event(
                client_ip, method, path,
                "⚡ Cross-Site Scripting (XSS) 공격 시도",
                "MEDIUM",
            ));
        }
    }

    None
}

fn create_event(
    client_ip: &str,
    method: &str,
    path: &str,
    threat_type: &str,
    severity: &str,
) -> SecurityEvent {
    let now = std::time::SystemTime::now();
    let datetime: chrono_free_timestamp = format_now();

    SecurityEvent {
        id: 0,
        timestamp: datetime,
        client_ip: client_ip.to_string(),
        method: method.to_string(),
        path: path.to_string(),
        threat_type: threat_type.to_string(),
        severity: severity.to_string(),
        action: "차단됨 (403 Forbidden)".to_string(),
    }
}

type chrono_free_timestamp = String;

fn format_now() -> String {
    // 외부 무거운 chrono 크레이트 없이 경량 네이티브 시간 포맷
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let secs = dur % 60;
    let mins = (dur / 60) % 60;
    let hours = ((dur / 3600) + 9) % 24; // KST (+9)
    format!("{:02}:{:02}:{:02} (KST)", hours, mins, secs)
}
