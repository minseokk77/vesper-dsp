pub mod cors;
pub mod mock;
pub mod stream_booster;
pub mod security;

use std::sync::{Arc, RwLock};
use hyper::{Method, Request, Response, StatusCode};
use hyper::body::Incoming;
use hyper::header::{HeaderValue, CONTENT_TYPE, HOST, USER_AGENT};
use http_body_util::{BodyExt, Full};
use bytes::Bytes;
use hyper_util::rt::TokioIo;
use tokio::net::TcpStream;
use serde::Deserialize;

use crate::config::GatewayConfig;
use crate::stats::GatewayStats;
use crate::proxy::cors::{handle_preflight, inject_cors_headers};
use crate::proxy::mock::handle_mock_response;
use crate::proxy::stream_booster::StreamBufferCache;
use crate::proxy::security::{SecurityLogBuffer, inspect_threat};
use crate::autostart::{enable_autostart, disable_autostart, is_autostart_enabled};

const DASHBOARD_HTML: &str = include_str!("../ui/dashboard.html");

#[derive(Deserialize)]
struct AddRouteReq {
    source: String,
    target: String,
}

#[derive(Deserialize)]
struct RemoveRouteReq {
    source: String,
}

#[derive(Deserialize)]
struct AddMockReq {
    path: String,
    json: String,
}

#[derive(Deserialize)]
struct AddTcpRouteReq {
    listen_port: u16,
    target: String,
}

#[derive(Deserialize)]
struct RemoveTcpRouteReq {
    listen_port: u16,
}

#[derive(Deserialize)]
struct SettingsReq {
    enable_cors: Option<bool>,
    enable_cache: Option<bool>,
    enable_stream_booster: Option<bool>,
    enable_security_shield: Option<bool>,
    enable_https: Option<bool>,
    enable_tcp_no_lag: Option<bool>,
    https_port: Option<u16>,
    autostart: Option<bool>,
}

/// 메인 HTTP 프록시 라우터 및 요청 처리기
pub async fn handle_request(
    req: Request<Incoming>,
    config_lock: Arc<RwLock<GatewayConfig>>,
    stats: Arc<GatewayStats>,
    stream_cache: Arc<StreamBufferCache>,
    security_buffer: Arc<SecurityLogBuffer>,
    client_ip: String,
) -> std::result::Result<Response<Full<Bytes>>, hyper::Error> {
    stats.record_request();

    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or("").to_string();
    let method = req.method().clone();
    let user_agent = req.headers()
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    // 0. 보안 쉴드(WAF): 수상한 접근 및 악성 공격 실시간 검사 & 차단
    {
        let cfg = config_lock.read().unwrap();
        if cfg.enable_security_shield && !path.starts_with("/pgate") {
            if let Some(threat) = inspect_threat(method.as_str(), &path, &query, &user_agent, &client_ip) {
                security_buffer.record(threat.clone());
                stats.record_threat_blocked();

                let block_html = format!(
                    r#"<!DOCTYPE html>
<html lang="ko">
<head>
    <meta charset="UTF-8">
    <title>403 Forbidden - Vesper Gate Security Shield</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #090d16; color: #f8fafc; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }}
        .card {{ background: rgba(26, 16, 28, 0.85); border: 1px solid rgba(244, 63, 94, 0.3); border-radius: 16px; padding: 32px; max-width: 520px; box-shadow: 0 8px 32px rgba(244, 63, 94, 0.2); text-align: center; }}
        h1 {{ color: #f43f5e; font-size: 24px; margin-bottom: 12px; }}
        p {{ color: #94a3b8; font-size: 14px; line-height: 1.6; }}
        .badge {{ background: rgba(244, 63, 94, 0.15); color: #f43f5e; padding: 6px 12px; border-radius: 8px; font-weight: bold; font-size: 13px; display: inline-block; margin: 12px 0; }}
    </style>
</head>
<body>
    <div class="card">
        <h1>🛡️ 403 Forbidden (접근 차단됨)</h1>
        <div class="badge">{}</div>
        <p>Vesper Gate 보안 쉴드가 비정상적이거나 악의적인 접근 시도를 감지하여 요청을 안전하게 차단했습니다.</p>
        <p style="font-size: 12px; color: #64748b; margin-top: 16px;">IP: {} | 시간: {}</p>
    </div>
</body>
</html>"#,
                    threat.threat_type, threat.client_ip, threat.timestamp
                );

                let mut resp = Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .header(CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"))
                    .body(Full::new(Bytes::from(block_html)))
                    .unwrap();
                if cfg.enable_cors {
                    inject_cors_headers(resp.headers_mut());
                }
                return Ok(resp);
            }
        }
    }

    // 1. CORS Preflight (OPTIONS) 요청 가로채기
    {
        let cfg = config_lock.read().unwrap();
        if cfg.enable_cors && method == Method::OPTIONS {
            return Ok(handle_preflight());
        }
    }

    // 2. 내장 웹 대시보드 서빙 (/ 또는 /pgate 또는 /pgate/ui)
    if path == "/" || path == "/pgate" || path == "/pgate/ui" {
        stats.record_cache_hit();
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"))
            .body(Full::new(Bytes::from(DASHBOARD_HTML)))
            .unwrap());
    }

    // 3. 게이트웨이 자체 상태 확인 API (/pgate/status)
    if path == "/pgate/status" || path == "/pingora/status" {
        let snapshot = stats.snapshot();
        let json_status = serde_json::to_string_pretty(&snapshot).unwrap_or_default();
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"))
            .body(Full::new(Bytes::from(json_status)))
            .unwrap());
    }

    // 4. 보안 위협 로그 조회/초기화 API (/pgate/api/security-logs)
    if path == "/pgate/api/security-logs" {
        match method {
            Method::GET => {
                let logs = security_buffer.get_recent_events();
                let json_val = serde_json::to_string(&logs).unwrap_or_default();
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"))
                    .body(Full::new(Bytes::from(json_val)))
                    .unwrap());
            }
            Method::DELETE => {
                security_buffer.clear();
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"))
                    .body(Full::new(Bytes::from("{\"status\":\"cleared\"}")))
                    .unwrap());
            }
            _ => {}
        }
    }

    // 5. 윈도우 hosts 파일 원클릭 자동 동기화 API (/pgate/api/hosts/sync)
    if path == "/pgate/api/hosts/sync" && method == Method::POST {
        let domains: Vec<String> = {
            let cfg = config_lock.read().unwrap();
            cfg.host_routes.keys().cloned().collect()
        };
        match crate::hosts::sync_hosts_file(&domains) {
            Ok(_) => {
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"))
                    .body(Full::new(Bytes::from("{\"status\":\"ok\"}")))
                    .unwrap());
            }
            Err(e) => {
                let err_msg = format!("{{\"error\":\"{}\"}}", e);
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"))
                    .body(Full::new(Bytes::from(err_msg)))
                    .unwrap());
            }
        }
    }

    // 6. TCP L4 포트 중계 제어 API (/pgate/api/tcp-routes)
    if path == "/pgate/api/tcp-routes" {
        match method {
            Method::GET => {
                let cfg = config_lock.read().unwrap();
                let json_val = serde_json::to_string(&cfg.tcp_routes).unwrap_or_default();
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"))
                    .body(Full::new(Bytes::from(json_val)))
                    .unwrap());
            }
            Method::POST => {
                let body_bytes = req.into_body().collect().await?.to_bytes();
                if let Ok(data) = serde_json::from_slice::<AddTcpRouteReq>(&body_bytes) {
                    let mut cfg = config_lock.write().unwrap();
                    let _ = cfg.add_tcp_route(data.listen_port, &data.target);
                }
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"))
                    .body(Full::new(Bytes::from("{\"status\":\"ok\"}")))
                    .unwrap());
            }
            Method::DELETE => {
                let body_bytes = req.into_body().collect().await?.to_bytes();
                if let Ok(data) = serde_json::from_slice::<RemoveTcpRouteReq>(&body_bytes) {
                    let mut cfg = config_lock.write().unwrap();
                    let _ = cfg.remove_tcp_route(data.listen_port);
                }
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"))
                    .body(Full::new(Bytes::from("{\"status\":\"ok\"}")))
                    .unwrap());
            }
            _ => {}
        }
    }

    // 7. 웹 UI 제어용 내부 REST API (/pgate/api/...)
    if path == "/pgate/api/routes" {
        match method {
            Method::GET => {
                let cfg = config_lock.read().unwrap();
                let mut json_val = serde_json::to_value(&*cfg).unwrap_or_default();
                if let serde_json::Value::Object(ref mut map) = json_val {
                    map.insert("autostart".to_string(), serde_json::Value::Bool(is_autostart_enabled()));
                }
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"))
                    .body(Full::new(Bytes::from(json_val.to_string())))
                    .unwrap());
            }
            Method::POST => {
                let body_bytes = req.into_body().collect().await?.to_bytes();
                if let Ok(data) = serde_json::from_slice::<AddRouteReq>(&body_bytes) {
                    let mut cfg = config_lock.write().unwrap();
                    let _ = cfg.add_route(&data.source, &data.target);
                }
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"))
                    .body(Full::new(Bytes::from("{\"status\":\"ok\"}")))
                    .unwrap());
            }
            Method::DELETE => {
                let body_bytes = req.into_body().collect().await?.to_bytes();
                if let Ok(data) = serde_json::from_slice::<RemoveRouteReq>(&body_bytes) {
                    let mut cfg = config_lock.write().unwrap();
                    let _ = cfg.remove_route(&data.source);
                }
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"))
                    .body(Full::new(Bytes::from("{\"status\":\"ok\"}")))
                    .unwrap());
            }
            _ => {}
        }
    }

    if path == "/pgate/api/mock" && method == Method::POST {
        let body_bytes = req.into_body().collect().await?.to_bytes();
        if let Ok(data) = serde_json::from_slice::<AddMockReq>(&body_bytes) {
            let mut cfg = config_lock.write().unwrap();
            if cfg.add_mock(&data.path, &data.json).is_ok() {
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"))
                    .body(Full::new(Bytes::from("{\"status\":\"ok\"}")))
                    .unwrap());
            }
        }
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"))
            .body(Full::new(Bytes::from("{\"error\":\"invalid json\"}")))
            .unwrap());
    }

    if path == "/pgate/api/settings" && method == Method::POST {
        let body_bytes = req.into_body().collect().await?.to_bytes();
        if let Ok(data) = serde_json::from_slice::<SettingsReq>(&body_bytes) {
            let mut cfg = config_lock.write().unwrap();
            if let Some(cors) = data.enable_cors {
                cfg.enable_cors = cors;
            }
            if let Some(cache) = data.enable_cache {
                cfg.enable_cache = cache;
            }
            if let Some(booster) = data.enable_stream_booster {
                cfg.enable_stream_booster = booster;
            }
            if let Some(sec) = data.enable_security_shield {
                cfg.enable_security_shield = sec;
            }
            if let Some(https) = data.enable_https {
                cfg.enable_https = https;
            }
            if let Some(no_lag) = data.enable_tcp_no_lag {
                cfg.enable_tcp_no_lag = no_lag;
                if no_lag {
                    let _ = crate::optimizer::network::apply_tcp_no_lag();
                } else {
                    let _ = crate::optimizer::network::revert_tcp_no_lag();
                }
            }
            if let Some(hport) = data.https_port {
                cfg.https_port = hport;
            }
            if let Some(auto) = data.autostart {
                if auto {
                    let _ = enable_autostart();
                } else {
                    let _ = disable_autostart();
                }
            }
            let _ = cfg.save();
        }
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"))
            .body(Full::new(Bytes::from("{\"status\":\"ok\"}")))
            .unwrap());
    }

    // 6. Mock API 엔드포인트 가로채기
    let (enable_cors, enable_stream_booster, mock_hit, target_backend) = {
        let cfg = config_lock.read().unwrap();
        let mock = cfg.mock_endpoints.get(&path).cloned();

        let host_header = req.headers()
            .get(HOST)
            .and_then(|h| h.to_str().ok())
            .map(|h| h.split(':').next().unwrap_or(h))
            .unwrap_or("");

        let target = if let Some(target) = cfg.host_routes.get(host_header) {
            Some(target.clone())
        } else if let Some((_, target)) = cfg.path_routes.iter().find(|(prefix, _)| path.starts_with(prefix.as_str())) {
            Some(target.clone())
        } else {
            None
        };

        (cfg.enable_cors, cfg.enable_stream_booster, mock, target)
    };

    if let Some(mock_json) = mock_hit {
        stats.record_cache_hit();
        return Ok(handle_mock_response(&mock_json, enable_cors));
    }

    // 7. 치지직/HLS 스트리밍 완충 캐시 확인 (.ts, .m4s)
    let full_req_uri = if query.is_empty() { path.clone() } else { format!("{}?{}", path, query) };
    if enable_stream_booster && (path.ends_with(".ts") || path.ends_with(".m4s") || path.ends_with(".mp4")) {
        if let Some(cached_chunk) = stream_cache.get(&full_req_uri) {
            stats.record_stream_buffer();
            let mut resp = Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, HeaderValue::from_static("video/mp4"))
                .body(Full::new(cached_chunk))
                .unwrap();
            if enable_cors {
                inject_cors_headers(resp.headers_mut());
            }
            return Ok(resp);
        }
    }

    let backend_url = match target_backend {
        Some(url) => url,
        None => {
            let host_header = req.headers()
                .get(HOST)
                .and_then(|h| h.to_str().ok())
                .unwrap_or("");

            let not_found_html = format!(
                r#"<!DOCTYPE html>
<html lang="ko">
<head>
    <meta charset="UTF-8">
    <title>Vesper Gate - 라우팅 경로를 찾을 수 없음</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #090d16; color: #f8fafc; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }}
        .card {{ background: rgba(18, 24, 38, 0.8); border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 16px; padding: 32px; max-width: 500px; box-shadow: 0 8px 32px rgba(0,0,0,0.4); text-align: center; }}
        h1 {{ color: #38bdf8; font-size: 22px; margin-bottom: 12px; }}
        p {{ color: #94a3b8; font-size: 14px; line-height: 1.6; }}
        code {{ background: #0f172a; color: #f43f5e; padding: 4px 8px; border-radius: 6px; font-family: monospace; }}
    </style>
</head>
<body>
    <div class="card">
        <h1>🚦 라우팅 대상을 찾을 수 없습니다</h1>
        <p>요청된 호스트(<code>{}</code>) 또는 경로(<code>{}</code>)에 연결된 백엔드가 없습니다.</p>
        <p><a href="/pgate/ui" style="color: #38bdf8; text-decoration: none; font-weight: 600;">👉 Vesper Gate 웹 대시보드 열기</a></p>
    </div>
</body>
</html>"#,
                host_header, path
            );

            let mut resp = Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"))
                .body(Full::new(Bytes::from(not_found_html)))
                .unwrap();
            if enable_cors {
                inject_cors_headers(resp.headers_mut());
            }
            return Ok(resp);
        }
    };

    // 8. 백엔드로 프록시 전달 및 m3u8 프리페치 파싱
    match forward_to_backend(req, &backend_url, enable_cors).await {
        Ok(resp) => {
            // m3u8 플레이리스트인 경우 백그라운드 프리페치 파싱
            if enable_stream_booster && (path.ends_with(".m3u8") || path.contains(".m3u8?")) {
                if let Some(ref bytes) = resp.body().clone().into_inner() {
                    if let Ok(text) = std::str::from_utf8(bytes) {
                        stream_cache.parse_and_prefetch(&full_req_uri, text);
                    }
                }
            }
            Ok(resp)
        }
        Err(_) => {
            stats.record_502();
            let mut resp = Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"))
                .body(Full::new(Bytes::from("{\"error\": \"502 Bad Gateway - 로컬 대상 서버가 꺼져 있습니다.\"}")))
                .unwrap();
            if enable_cors {
                inject_cors_headers(resp.headers_mut());
            }
            Ok(resp)
        }
    }
}

/// 백엔드로 요청을 포워딩하고 응답을 반환하는 핵심 프록시 함수
async fn forward_to_backend(
    req: Request<Incoming>,
    target_backend: &str,
    enable_cors: bool,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    let (target_host, target_port) = parse_target(target_backend)?;
    let addr = format!("{}:{}", target_host, target_port);

    let stream = TcpStream::connect(addr).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
    tokio::spawn(async move {
        let _ = conn.await;
    });

    let (parts, body) = req.into_parts();
    let body_bytes = body.collect().await?.to_bytes();

    let mut forward_req = hyper::Request::builder()
        .method(parts.method)
        .uri(parts.uri);

    for (header_name, header_value) in parts.headers.iter() {
        if header_name != hyper::header::HOST {
            forward_req = forward_req.header(header_name, header_value);
        }
    }
    forward_req = forward_req.header(hyper::header::HOST, format!("{}:{}", target_host, target_port));

    let outgoing_req = forward_req.body(Full::new(body_bytes))?;
    let response = sender.send_request(outgoing_req).await?;

    let (resp_parts, resp_body) = response.into_parts();
    let resp_bytes = resp_body.collect().await?.to_bytes();

    let mut final_resp = Response::builder().status(resp_parts.status);
    for (header_name, header_value) in resp_parts.headers.iter() {
        final_resp = final_resp.header(header_name, header_value);
    }

    let mut built_resp = final_resp.body(Full::new(resp_bytes))?;
    if enable_cors {
        inject_cors_headers(built_resp.headers_mut());
    }

    Ok(built_resp)
}

fn parse_target(target: &str) -> Result<(String, u16), Box<dyn std::error::Error + Send + Sync>> {
    if let Ok(port) = target.parse::<u16>() {
        return Ok(("127.0.0.1".to_string(), port));
    }

    let target_clean = target.trim_start_matches("http://").trim_start_matches("https://");
    let mut parts = target_clean.split(':');
    let host = parts.next().unwrap_or("127.0.0.1").to_string();
    let port = parts.next().and_then(|p| p.parse::<u16>().ok()).unwrap_or(80);

    Ok((host, port))
}
