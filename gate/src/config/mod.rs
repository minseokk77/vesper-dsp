use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use crate::error::{GatewayError, Result};

/// 기본 게이트웨이 포트 (하드코딩 방지를 위한 기본 상수 정의)
pub const DEFAULT_GATEWAY_PORT: u16 = 8080;
pub const DEFAULT_HOST: &str = "127.0.0.1";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GatewayConfig {
    /// 게이트웨이가 대기할 호스트 IP (기본값: 127.0.0.1)
    #[serde(default = "default_host")]
    pub host: String,

    /// 게이트웨이 리스닝 포트 (기본값: 8080 또는 80)
    #[serde(default = "default_port")]
    pub port: u16,

    /// CORS 자동 해결 여부
    #[serde(default = "default_true")]
    pub enable_cors: bool,

    /// 램(RAM) 메모리 캐싱 활성화 여부
    #[serde(default = "default_true")]
    pub enable_cache: bool,

    /// 치지직 및 HLS 실시간 스트리밍 지터 완충 / 프리페치 부스터 활성화 여부
    #[serde(default = "default_true")]
    pub enable_stream_booster: bool,

    /// 악성 봇 및 수상한 접근 침입 탐지/차단(WAF) 활성화 여부
    #[serde(default = "default_true")]
    pub enable_security_shield: bool,

    /// 도메인(Host) 기반 라우팅 규칙 (예: "app.local" => "http://127.0.0.1:5173")
    #[serde(default)]
    pub host_routes: HashMap<String, String>,

    /// 경로(Path) 기반 라우팅 규칙 (예: "/api" => "http://127.0.0.1:8000")
    #[serde(default)]
    pub path_routes: HashMap<String, String>,

    /// Mock API 가짜 응답 규칙 (예: "/api/test" => "{\"status\":\"ok\"}")
    #[serde(default)]
    pub mock_endpoints: HashMap<String, String>,
}

fn default_host() -> String {
    DEFAULT_HOST.to_string()
}

fn default_port() -> u16 {
    DEFAULT_GATEWAY_PORT
}

fn default_true() -> bool {
    true
}

impl Default for GatewayConfig {
    fn default() -> Self {
        let mut host_routes = HashMap::new();
        host_routes.insert("app.local".to_string(), "http://127.0.0.1:5173".to_string());
        host_routes.insert("api.local".to_string(), "http://127.0.0.1:8000".to_string());

        let mut mock_endpoints = HashMap::new();
        mock_endpoints.insert(
            "/api/hello".to_string(),
            serde_json::json!({
                "message": "vgate 게이트웨이가 정상 작동 중입니다!",
                "timestamp": 2026
            }).to_string(),
        );

        Self {
            host: default_host(),
            port: default_port(),
            enable_cors: true,
            enable_cache: true,
            enable_stream_booster: true,
            enable_security_shield: true,
            host_routes,
            path_routes: HashMap::new(),
            mock_endpoints,
        }
    }
}

impl GatewayConfig {
    /// 설정 파일 경로 가져오기 (~/.pgate/config.toml)
    pub fn config_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().ok_or_else(|| {
            GatewayError::ConfigError("사용자 홈 디렉토리를 찾을 수 없습니다.".to_string())
        })?;
        let pgate_dir = home_dir.join(".pgate");
        if !pgate_dir.exists() {
            fs::create_dir_all(&pgate_dir)?;
        }
        Ok(pgate_dir.join("config.toml"))
    }

    /// 설정 파일 불러오기 (없으면 기본값 생성)
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            let default_config = Self::default();
            default_config.save()?;
            return Ok(default_config);
        }

        let content = fs::read_to_string(&path)?;
        let config: GatewayConfig = toml::from_str(&content)
            .map_err(|e| GatewayError::ConfigError(format!("설정 파일 파싱 오류: {}", e)))?;
        Ok(config)
    }

    /// 설정 파일 저장하기
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let content = toml::to_string_pretty(self)
            .map_err(|e| GatewayError::ConfigError(format!("설정 직렬화 오류: {}", e)))?;
        fs::write(path, content)?;
        Ok(())
    }

    /// 라우팅 추가 (도메인 또는 경로)
    pub fn add_route(&mut self, source: &str, target: &str) -> Result<()> {
        let formatted_target = if target.starts_with("http://") || target.starts_with("https://") {
            target.to_string()
        } else if let Ok(port) = target.parse::<u16>() {
            format!("http://127.0.0.1:{}", port)
        } else {
            format!("http://{}", target)
        };

        if source.starts_with('/') {
            self.path_routes.insert(source.to_string(), formatted_target);
        } else {
            self.host_routes.insert(source.to_string(), formatted_target);
        }
        self.save()?;
        Ok(())
    }

    /// 라우팅 제거
    pub fn remove_route(&mut self, source: &str) -> Result<bool> {
        let mut removed = false;
        if self.host_routes.remove(source).is_some() {
            removed = true;
        }
        if self.path_routes.remove(source).is_some() {
            removed = true;
        }
        if self.mock_endpoints.remove(source).is_some() {
            removed = true;
        }
        if removed {
            self.save()?;
        }
        Ok(removed)
    }

    /// Mock API 추가
    pub fn add_mock(&mut self, path: &str, json_content: &str) -> Result<()> {
        let path = if path.starts_with('/') { path.to_string() } else { format!("/{}", path) };
        // JSON 유효성 검사
        let parsed: serde_json::Value = serde_json::from_str(json_content)
            .map_err(|e| GatewayError::ConfigError(format!("잘못된 JSON 형식입니다: {}", e)))?;
        self.mock_endpoints.insert(path, parsed.to_string());
        self.save()?;
        Ok(())
    }
}
