use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::net::{TcpListener, TcpStream};
use colored::*;

/// 활성화된 TCP L4 스트림 프록시 매니저 (게임 전용 No-Delay 고속 터널)
pub struct TcpProxyManager {
    handles: Vec<tokio::task::JoinHandle<()>>,
}

impl Default for TcpProxyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TcpProxyManager {
    pub fn new() -> Self {
        Self { handles: Vec::new() }
    }

    /// 설정된 모든 TCP 포트(예: 25565 마크, 3306 DB) 리스너 구동
    pub fn start_listeners(
        &mut self,
        routes_lock: Arc<RwLock<HashMap<u16, String>>>,
    ) {
        let routes = {
            let lock = routes_lock.read().unwrap();
            lock.clone()
        };

        for (listen_port, target_addr) in routes {
            let target = target_addr.clone();
            let handle = tokio::spawn(async move {
                let bind_addr = format!("0.0.0.0:{}", listen_port);
                let listener = match TcpListener::bind(&bind_addr).await {
                    Ok(l) => {
                        println!("{} TCP 게임/스트림 중계 가동 (NoDelay 적용): {} ➜ {}", "🎮 [TCP/L4]".magenta().bold(), bind_addr.cyan(), target.yellow());
                        l
                    }
                    Err(e) => {
                        println!("{} TCP 포트({}) 바인딩 실패: {}", "✖ [오류]".red().bold(), listen_port, e);
                        return;
                    }
                };

                while let Ok((mut client_stream, _)) = listener.accept().await {
                    // 클라이언트 소켓에 Nagle 해제 (0ms 즉시 전송) 적용
                    let _ = client_stream.set_nodelay(true);

                    let target_clone = target.clone();
                    tokio::spawn(async move {
                        let target_clean = target_clone.trim_start_matches("http://").trim_start_matches("https://");
                        let addr = if target_clean.contains(':') {
                            target_clean.to_string()
                        } else {
                            format!("127.0.0.1:{}", target_clean)
                        };

                        if let Ok(mut server_stream) = TcpStream::connect(&addr).await {
                            // 백엔드 서버 소켓에도 Nagle 해제 적용
                            let _ = server_stream.set_nodelay(true);
                            let _ = tokio::io::copy_bidirectional(&mut client_stream, &mut server_stream).await;
                        }
                    });
                }
            });
            self.handles.push(handle);
        }
    }
}
