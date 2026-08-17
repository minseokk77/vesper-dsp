use hyper::{Response, StatusCode};
use hyper::header::{HeaderValue, CONTENT_TYPE};
use http_body_util::Full;
use bytes::Bytes;
use crate::proxy::cors::inject_cors_headers;

/// Mock JSON 응답 생성
pub fn handle_mock_response(json_body: &str, enable_cors: bool) -> Response<Full<Bytes>> {
    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"))
        .body(Full::new(Bytes::from(json_body.to_string())))
        .unwrap();

    if enable_cors {
        inject_cors_headers(response.headers_mut());
    }

    response
}
