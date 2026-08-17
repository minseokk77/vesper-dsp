#![windows_subsystem = "windows"]

mod autostart;
mod cli;
mod config;
mod daemon;
mod error;
mod hosts;
mod proxy;
mod stats;
mod tcp_proxy;
mod tls;
mod tray;

use clap::Parser;
use colored::*;
use std::sync::{Arc, RwLock};
use tokio::net::TcpListener;
use hyper_util::rt::TokioIo;
use hyper_util::server::conn::auto;

use autostart::{enable_autostart, disable_autostart, is_autostart_enabled, register_control_panel, unregister_control_panel};
use cli::{Cli, Commands};
use config::GatewayConfig;
use stats::GatewayStats;
use proxy::handle_request;
use daemon::{spawn_daemon, stop_daemon, get_saved_pid, save_pid, remove_pid_file};
use tray::run_tray_thread;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 윈도우 환경: 터미널에서 실행된 경우 부모 콘솔에 연결하여 CLI 출력 지원
    #[cfg(target_os = "windows")]
    unsafe {
        use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};
        let _ = AttachConsole(ATTACH_PARENT_PROCESS);
    }

    // 윈도우 제어판 '프로그램 추가/제거' 목록에 자동 등록 (일반인 친화적 UX)
    let _ = register_control_panel();

    let cli = Cli::parse();
    let mut config = GatewayConfig::load()?;

    let command = cli.command.unwrap_or(Commands::Start {
        port: None,
        host: None,
        daemon: false,
        no_browser: true,
    });

    match command {
        Commands::Start { port, host, daemon, no_browser: _ } => {
            if daemon {
                let pid = spawn_daemon(port, host, true)?;
                println!("{} pgate 게이트웨이가 백그라운드에서 실행되었습니다!", "✔ [성공]".green().bold());
                println!("   PID: {}", pid.to_string().cyan().bold());
                println!("   웹 대시보드: http://{}:{}/pgate/ui", config.host, port.unwrap_or(config.port));
                println!("   종료하려면: {} 명령어를 입력하세요.", "pgate stop".yellow().bold());
                return Ok(());
            }

            if let Some(p) = port {
                config.port = p;
            }
            if let Some(h) = host {
                config.host = h;
            }

            // 현재 PID 기록
            let current_pid = std::process::id();
            let _ = save_pid(current_pid);

            start_server(config).await?;
            remove_pid_file();
        }

        Commands::Stop => {
            if stop_daemon()? {
                println!("{} 백그라운드 게이트웨이가 성공적으로 종료되었습니다.", "✔ [성공]".green().bold());
            } else {
                println!("{} 실행 중인 백그라운드 게이트웨이를 찾을 수 없습니다.", "ℹ [안내]".blue().bold());
            }
        }

        Commands::Restart { port } => {
            let _ = stop_daemon();
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            let pid = spawn_daemon(port, None, true)?;
            println!("{} pgate 게이트웨이가 백그라운드에서 재시작되었습니다!", "✔ [성공]".green().bold());
            println!("   PID: {}", pid.to_string().cyan().bold());
        }

        Commands::Autostart { action } => {
            match action.to_lowercase().as_str() {
                "enable" | "on" | "add" => {
                    enable_autostart()?;
                    println!("{} 윈도우 부팅 시 pgate 게이트웨이가 자동 실행되도록 등록되었습니다!", "✔ [설정 완료]".green().bold());
                    println!("   (컴퓨터를 켤 때마다 창 없이 백그라운드에서 조용히 자동 실행됩니다)");
                }
                "disable" | "off" | "remove" => {
                    disable_autostart()?;
                    println!("{} 윈도우 부팅 시 자동 실행이 해제되었습니다.", "✔ [해제 완료]".green().bold());
                }
                "status" | "check" => {
                    let enabled = is_autostart_enabled();
                    println!("{} 윈도우 부팅 시 자동 시작 상태: {}", "🔍".bold(), if enabled { "등록됨 (ON)".green().bold() } else { "등록 안 됨 (OFF)".yellow() });
                }
                _ => {
                    println!("{} 유효하지 않은 동작입니다. 'enable', 'disable', 또는 'status'를 입력하세요.", "✖ [오류]".red().bold());
                }
            }
        }

        Commands::Uninstall => {
            let _ = stop_daemon();
            let _ = disable_autostart();
            let _ = unregister_control_panel();
            if let Some(home_dir) = dirs::home_dir() {
                let pgate_dir = home_dir.join(".pgate");
                if pgate_dir.exists() {
                    let _ = std::fs::remove_dir_all(pgate_dir);
                }
            }
            println!("{} pgate 관련 모든 설정, 제어판 등록 및 프로세스가 완벽하게 정리되었습니다.", "✔ [정리 완료]".green().bold());
            println!("   이제 pgate.exe 실행 파일만 휴지통에 버리시면 100% 흔적 없이 삭제됩니다.");
        }

        Commands::Add { source, target } => {
            config.add_route(&source, &target)?;
            println!("{} 라우팅 규칙이 성공적으로 추가되었습니다:", "✔ [성공]".green().bold());
            println!("   {} ➜ {}", source.cyan().bold(), target.yellow());
            println!("   설정 파일: {}", GatewayConfig::config_path()?.display().to_string().dimmed());
        }

        Commands::Remove { source } => {
            let removed = config.remove_route(&source)?;
            if removed {
                println!("{} '{}' 라우팅 규칙이 삭제되었습니다.", "✔ [성공]".green().bold(), source.cyan());
            } else {
                println!("{} '{}' 규칙을 찾을 수 없습니다.", "✖ [경고]".yellow().bold(), source);
            }
        }

        Commands::List => {
            print_route_list(&config)?;
        }

        Commands::Mock { path, json } => {
            config.add_mock(&path, &json)?;
            println!("{} 가짜 API (Mock)가 등록되었습니다:", "✔ [성공]".green().bold());
            println!("   경로: {}", path.cyan().bold());
            println!("   응답: {}", json.dimmed());
        }

        Commands::Cors { action } => {
            let enable = match action.to_lowercase().as_str() {
                "enable" | "on" | "true" | "1" => true,
                "disable" | "off" | "false" | "0" => false,
                _ => {
                    println!("{} 유효하지 않은 동작입니다. 'enable' 또는 'disable'을 입력하세요.", "✖ [오류]".red().bold());
                    return Ok(());
                }
            };
            config.enable_cors = enable;
            config.save()?;
            println!("{} CORS 자동 해결 기능: {}", "✔ [설정 완료]".green().bold(), if enable { "활성화 (ON)".green().bold() } else { "비활성화 (OFF)".red() });
        }

        Commands::Status { url } => {
            let target_url = url.unwrap_or_else(|| format!("http://{}:{}/pgate/status", config.host, config.port));
            let pid_info = get_saved_pid()?.map(|p| format!("(PID: {})", p)).unwrap_or_else(|| "(직접 실행 중)".to_string());
            println!("{} 게이트웨이 상태 확인 중: {} {}", "🔍".bold(), target_url.cyan(), pid_info.dimmed());
            
            match fetch_status(&target_url).await {
                Ok(status_json) => {
                    println!("{}\n{}", "✔ 게이트웨이가 정상 가동 중입니다:".green().bold(), status_json);
                }
                Err(e) => {
                    println!("{} 게이트웨이에 연결할 수 없습니다 (서버가 꺼져 있을 수 있습니다): {}", "✖ [오류]".red().bold(), e);
                }
            }
        }
    }

    Ok(())
}

fn print_route_list(config: &GatewayConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "=================================================".bright_blue());
    println!("  🚀 {} {}", "pgate".bold().cyan(), "로컬 게이트웨이 라우팅 테이블".bold());
    println!("{}", "=================================================".bright_blue());
    println!(" • 바인딩 주소: {}:{}", config.host.yellow(), config.port.to_string().yellow().bold());
    println!(" • CORS 자동 해결: {}", if config.enable_cors { "켜짐 (ON)".green().bold() } else { "꺼짐 (OFF)".red() });
    println!(" • 메모리 캐싱: {}", if config.enable_cache { "켜짐 (ON)".green().bold() } else { "꺼짐 (OFF)".red() });
    println!(" • 부팅 자동 시작: {}", if is_autostart_enabled() { "등록됨 (ON)".green().bold() } else { "등록 안 됨 (OFF)".yellow() });
    println!();

    println!("{}", "[도메인 라우팅 (Host Routes)]".bold().underline());
    if config.host_routes.is_empty() {
        println!("  (등록된 도메인 라우트 없음)");
    } else {
        for (host, target) in &config.host_routes {
            println!("  http://{:<20} ➜ {}", host.cyan().bold(), target.yellow());
        }
    }
    println!();

    println!("{}", "[경로 라우팅 (Path Routes)]".bold().underline());
    if config.path_routes.is_empty() {
        println!("  (등록된 경로 라우트 없음)");
    } else {
        for (path, target) in &config.path_routes {
            println!("  {:<27} ➜ {}", path.cyan().bold(), target.yellow());
        }
    }
    println!();

    println!("{}", "[가짜 API 엔드포인트 (Mock API)]".bold().underline());
    if config.mock_endpoints.is_empty() {
        println!("  (등록된 Mock 엔드포인트 없음)");
    } else {
        for (path, json) in &config.mock_endpoints {
            println!("  {:<27} ➜ {}", path.green().bold(), json.dimmed());
        }
    }
    println!("{}", "=================================================".bright_blue());
    println!("  설정 파일: {}", GatewayConfig::config_path()?.display().to_string().dimmed());
    Ok(())
}

async fn start_server(config: GatewayConfig) -> Result<(), Box<dyn std::error::Error>> {
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await?;
    let dashboard_url = format!("http://{}:{}/pgate/ui", config.host, config.port);

    println!("{}", "=================================================".green());
    println!("  🚀 {} {}", "pgate 게이트웨이가 가동되었습니다!".bold().green(), format!("({}:{})", config.host, config.port).yellow());
    println!("{}", "=================================================".green());
    println!("  • 웹 대시보드 : {}", dashboard_url.cyan().bold());
    println!("  • 시스템 트레이 : 윈도우 우측 하단 숨겨진 아이콘에 상주 중");
    println!("  • 종료하려면 Ctrl + C 또는 트레이 아이콘에서 [종료]를 누르세요.");
    println!("{}", "-------------------------------------------------".dimmed());

    // 윈도우 순수 Win32 시스템 트레이 메시지 펌프 스레드 실행
    run_tray_thread(dashboard_url);

    let config_lock = Arc::new(RwLock::new(config.clone()));
    let stats_arc = Arc::new(GatewayStats::new());
    let stream_cache = Arc::new(crate::proxy::stream_booster::StreamBufferCache::new());
    let security_buffer = Arc::new(crate::proxy::security::SecurityLogBuffer::new());

    // 1. 마인크래프트 & 일반 TCP L4 포트 중계 매니저 구동
    let tcp_routes_lock = Arc::new(RwLock::new(config.tcp_routes.clone()));
    let mut tcp_mgr = crate::tcp_proxy::TcpProxyManager::new();
    tcp_mgr.start_listeners(tcp_routes_lock);

    // 2. 로컬 HTTPS (포트 8443) 백그라운드 서버 가동
    if let Ok(tls_acceptor) = crate::tls::load_or_generate_tls_config() {
        let https_addr = format!("{}:{}", config.host, config.https_port);
        if let Ok(https_listener) = TcpListener::bind(&https_addr).await {
            println!("  • 로컬 HTTPS   : {}", format!("https://{}:{}", config.host, config.https_port).cyan().bold());
            let cfg_https = Arc::clone(&config_lock);
            let stats_https = Arc::clone(&stats_arc);
            let sc_https = Arc::clone(&stream_cache);
            let sec_https = Arc::clone(&security_buffer);

            tokio::spawn(async move {
                while let Ok((stream, remote_addr)) = https_listener.accept().await {
                    let client_ip = remote_addr.ip().to_string();
                    let acceptor = tls_acceptor.clone();
                    let cfg_clone = Arc::clone(&cfg_https);
                    let stats_clone = Arc::clone(&stats_https);
                    let sc_clone = Arc::clone(&sc_https);
                    let sec_clone = Arc::clone(&sec_https);

                    tokio::spawn(async move {
                        if let Ok(tls_stream) = acceptor.accept(stream).await {
                            let io = TokioIo::new(tls_stream);
                            let service = hyper::service::service_fn(move |req| {
                                let cfg = Arc::clone(&cfg_clone);
                                let st = Arc::clone(&stats_clone);
                                let sc = Arc::clone(&sc_clone);
                                let sec = Arc::clone(&sec_clone);
                                let ip = client_ip.clone();
                                async move { handle_request(req, cfg, st, sc, sec, ip).await }
                            });

                            let _ = auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                                .serve_connection(io, service)
                                .await;
                        }
                    });
                }
            });
        }
    }

    loop {
        let (stream, remote_addr) = listener.accept().await?;
        let client_ip = remote_addr.ip().to_string();
        let io = TokioIo::new(stream);
        let cfg_clone = Arc::clone(&config_lock);
        let stats_clone = Arc::clone(&stats_arc);
        let stream_cache_clone = Arc::clone(&stream_cache);
        let sec_clone = Arc::clone(&security_buffer);

        tokio::spawn(async move {
            let service = hyper::service::service_fn(move |req| {
                let cfg = Arc::clone(&cfg_clone);
                let st = Arc::clone(&stats_clone);
                let sc = Arc::clone(&stream_cache_clone);
                let sec = Arc::clone(&sec_clone);
                let ip = client_ip.clone();
                async move { handle_request(req, cfg, st, sc, sec, ip).await }
            });

            if let Err(_err) = auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                .serve_connection(io, service)
                .await
            {
                // 클라이언트 연결 종료
            }
        });
    }
}

async fn fetch_status(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let uri: hyper::Uri = url.parse()?;
    let host = uri.host().unwrap_or("127.0.0.1");
    let port = uri.port_u16().unwrap_or(8080);

    let stream = tokio::net::TcpStream::connect((host, port)).await?;
    let io = TokioIo::new(stream);

    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;
    tokio::spawn(async move {
        let _ = conn.await;
    });

    let req = hyper::Request::builder()
        .uri(uri.path())
        .header(hyper::header::HOST, host)
        .body(http_body_util::Empty::<bytes::Bytes>::new())?;

    let res = sender.send_request(req).await?;
    let body = http_body_util::BodyExt::collect(res.into_body()).await?.to_bytes();
    Ok(String::from_utf8_lossy(&body).to_string())
}
