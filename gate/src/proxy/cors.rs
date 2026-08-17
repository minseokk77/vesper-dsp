use hyper::header::{HeaderMap, HeaderValue, ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_MAX_AGE};
use hyper::{Response, StatusCode};
use http_body_util::Full;
use bytes::Bytes;

/// CORS 헤더를 응답에 자동 주입
pub fn inject_cors_headers(headers: &mut HeaderMap) {
    headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    headers.insert(ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static("GET, POST, PUT, DELETE, PATCH, OPTIONS, HEAD"));
    headers.insert(ACCESS_CONTROL_ALLOW_HEADERS, HeaderValue::from_static("Authorization, Content-Type, Accept, Origin, User-Agent, X-Requested-With"));
    headers.insert(ACCESS_CONTROL_ALLOW_CREDENTIALS, HeaderValue::from_static("true"));
    headers.insert(ACCESS_CONTROL_MAX_AGE, HeaderValue::from_static("86400"));
}

/// OPTIONS 사전 요청(Preflight)에 대한 즉시 응답 생성
pub fn handle_preflight() -> Response<Full<Bytes>> {
    let mut response = Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(Full::new(Bytes::new()))
        .unwrap();

    inject_cors_headers(response.headers_mut());
    response
}
