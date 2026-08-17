pub mod cors;
pub mod mock;

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
    autostart: Option<bool>,
}

/// 메인 HTTP 프록시 라우터 및 요청 처리기
pub async fn handle_request(
    req: Request<Incoming>,
    config_lock: Arc<RwLock<GatewayConfig>>,
    stats: Arc<GatewayStats>,
) -> std::result::Result<Response<Full<Bytes>>, hyper::Error> {
    stats.record_request();

    let path = req.uri().path().to_string();
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
    let (enable_cors, mock_hit, target_backend) = {
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

        (cfg.enable_cors, mock, target)
    };

    if let Some(mock_json) = mock_hit {
        stats.record_cache_hit();
        return Ok(handle_mock_response(&mock_json, enable_cors));
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
    <title>pgate - 라우팅 경로를 찾을 수 없음</title>
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
        <p><a href="/pgate/ui" style="color: #38bdf8; text-decoration: none; font-weight: 600;">👉 pgate 웹 대시보드 열기</a></p>
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

    // 6. 백엔드로 프록시 전달
    match forward_to_backend(req, &backend_url, enable_cors).await {
        Ok(resp) => Ok(resp),
        Err(_) => {
            stats.record_502();
            let bad_gateway_html = format!(
                r#"<!DOCTYPE html>
<html lang="ko">
<head>
    <meta charset="UTF-8">
    <title>pgate - 502 Bad Gateway</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #090d16; color: #f8fafc; display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }}
        .card {{ background: rgba(18, 24, 38, 0.8); border: 1px solid rgba(244, 63, 94, 0.3); border-radius: 16px; padding: 32px; max-width: 500px; box-shadow: 0 8px 32px rgba(0,0,0,0.4); text-align: center; }}
        h1 {{ color: #f43f5e; font-size: 22px; margin-bottom: 12px; }}
        p {{ color: #94a3b8; font-size: 14px; line-height: 1.6; }}
        code {{ background: #0f172a; color: #38bdf8; padding: 4px 8px; border-radius: 6px; font-family: monospace; }}
    </style>
</head>
<body>
    <div class="card">
        <h1>⚠️ 백엔드 서버에 연결할 수 없습니다 (502)</h1>
        <p>대상 서버(<code>{}</code>)가 꺼져 있거나 응답하지 않습니다.</p>
        <p>로컬 개발 서버(Vite, FastAPI 등)가 켜져 있는지 확인해 주세요.</p>
    </div>
</body>
</html>"#,
                backend_url
            );

            let mut resp = Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .header(CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"))
                .body(Full::new(Bytes::from(bad_gateway_html)))
                .unwrap();
            if enable_cors {
                inject_cors_headers(resp.headers_mut());
            }
            Ok(resp)
        }
    }
}

async fn forward_to_backend(
    mut req: Request<Incoming>,
    backend_base: &str,
    enable_cors: bool,
) -> Result<Response<Full<Bytes>>, Box<dyn std::error::Error + Send + Sync>> {
    let backend_uri: hyper::Uri = backend_base.parse()?;
    let host = backend_uri.host().unwrap_or("127.0.0.1");
    let port = backend_uri.port_u16().unwrap_or(80);

    let stream = TcpStream::connect((host, port)).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
    tokio::spawn(async move {
        let _ = conn.await;
    });

    let original_path = req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("/");
    *req.uri_mut() = original_path.parse()?;

    let (parts, body) = req.into_parts();
    let body_bytes = body.collect().await?.to_bytes();
    let new_req = Request::from_parts(parts, Full::new(body_bytes));

    let backend_res = sender.send_request(new_req).await?;
    let (res_parts, res_body) = backend_res.into_parts();
    let res_bytes = res_body.collect().await?.to_bytes();

    let mut final_response = Response::from_parts(res_parts, Full::new(res_bytes));
    if enable_cors {
        inject_cors_headers(final_response.headers_mut());
    }

    Ok(final_response)
}
