pub mod cors;
pub mod mock;
pub mod stream_booster;

use std::sync::{Arc, RwLock};
use hyper::{Method, Request, Response, StatusCode};
use hyper::body::Incoming;
use hyper::header::{HeaderValue, CONTENT_TYPE, HOST};
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
struct SettingsReq {
    enable_cors: Option<bool>,
    enable_cache: Option<bool>,
    enable_stream_booster: Option<bool>,
    autostart: Option<bool>,
}

/// 메인 HTTP 프록시 라우터 및 요청 처리기
pub async fn handle_request(
    req: Request<Incoming>,
    config_lock: Arc<RwLock<GatewayConfig>>,
    stats: Arc<GatewayStats>,
    stream_cache: Arc<StreamBufferCache>,
) -> std::result::Result<Response<Full<Bytes>>, hyper::Error> {
    stats.record_request();

    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or("").to_string();
    let method = req.method().clone();

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

    // 4. 웹 UI 제어용 내부 REST API (/pgate/api/...)
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

    // 5. Mock API 엔드포인트 가로채기
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

    // 6. 치지직/HLS 스트리밍 완충 캐시 확인 (.ts, .m4s)
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

    // 7. 백엔드로 프록시 전달 및 m3u8 프리페치 파싱
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
