use bytes::Bytes;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// HLS 영상 세그먼트 캐시 (청크 데이터 + 저장 시점)
#[derive(Clone)]
pub struct StreamBufferCache {
    cache: Arc<RwLock<HashMap<String, (Bytes, Instant)>>>,
}

impl Default for StreamBufferCache {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamBufferCache {
    pub fn new() -> Self {
        let instance = Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // 30초 이상 지난 오래된 영상 청크는 메모리 절약을 위해 자동 청소 (LRU/TTL)
        let cleanup_cache = Arc::clone(&instance.cache);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(15)).await;
                let mut lock = cleanup_cache.write().unwrap();
                let now = Instant::now();
                lock.retain(|_, (_, time)| now.duration_since(*time) < Duration::from_secs(45));
            }
        });

        instance
    }

    /// 프리페치되거나 캐싱된 영상 조각 가져오기
    pub fn get(&self, uri: &str) -> Option<Bytes> {
        let lock = self.cache.read().unwrap();
        lock.get(uri).map(|(data, _)| data.clone())
    }

    /// 영상 조각 메모리에 저장
    pub fn insert(&self, uri: String, data: Bytes) {
        let mut lock = self.cache.write().unwrap();
        lock.insert(uri, (data, Instant::now()));
    }

    /// m3u8 플레이리스트 내용에서 다음 영상 조각(.ts / .m4s) URL을 파싱하여 백그라운드 프리페치 실행
    pub fn parse_and_prefetch(&self, base_uri: &str, m3u8_content: &str) {
        let mut chunk_uris = Vec::new();
        for line in m3u8_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if trimmed.ends_with(".ts") || trimmed.ends_with(".m4s") || trimmed.contains(".ts?") || trimmed.contains(".m4s?") {
                chunk_uris.push(trimmed.to_string());
            }
        }

        // 가장 최신의 다음 세그먼트 최대 3개를 추출하여 백그라운드 프리페치
        let target_chunks: Vec<String> = chunk_uris.into_iter().rev().take(3).collect();
        let cache_clone = self.clone();
        let base = base_uri.to_string();

        tokio::spawn(async move {
            for chunk_path in target_chunks {
                let full_url = if chunk_path.starts_with("http://") || chunk_path.starts_with("https://") {
                    chunk_path.clone()
                } else if let Some(last_slash) = base.rfind('/') {
                    format!("{}/{}", &base[..last_slash], chunk_path)
                } else {
                    chunk_path.clone()
                };

                // 이미 캐시에 있는지 확인
                if cache_clone.get(&full_url).is_some() {
                    continue;
                }

                // 백그라운드 초고속 프리페치 다운로드
                if let Ok(data) = download_chunk(&full_url).await {
                    cache_clone.insert(full_url, data);
                }
            }
        });
    }
}

/// 백그라운드에서 CDN 세그먼트 다운로드
async fn download_chunk(url: &str) -> Result<Bytes, Box<dyn std::error::Error + Send + Sync>> {
    let uri: hyper::Uri = url.parse()?;
    let host = uri.host().ok_or("호스트 누락")?;
    let port = uri.port_u16().unwrap_or_else(|| {
        if uri.scheme_str() == Some("https") { 443 } else { 80 }
    });

    let addr = format!("{}:{}", host, port);
    let stream = tokio::net::TcpStream::connect(addr).await?;
    let io = hyper_util::rt::TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
    tokio::spawn(async move {
        let _ = conn.await;
    });

    let req = hyper::Request::builder()
        .uri(uri.path_and_query().map(|pq| pq.as_str()).unwrap_or("/"))
        .header(hyper::header::HOST, host)
        .header(hyper::header::USER_AGENT, "VesperGate-StreamBooster/0.1.0")
        .body(http_body_util::Empty::<Bytes>::new())?;

    let res = sender.send_request(req).await?;
    let body = http_body_util::BodyExt::collect(res.into_body()).await?.to_bytes();
    Ok(body)
}
