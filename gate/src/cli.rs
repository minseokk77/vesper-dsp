use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "vgate")]
#[command(author = "Vesper Ecosystem")]
#[command(version = "0.1.0")]
#[command(about = "⚡ Vesper Gate: 초고속 Rust 기반 로컬 리버스 프록시 & API 게이트웨이", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 🚀 Vesper Gate 프록시 서버 시작
    Start {
        /// 리스닝할 포트 번호 (생략 시 config.toml의 포트 사용)
        #[arg(short, long)]
        port: Option<u16>,

        /// 바인딩할 호스트 주소 (기본값: 127.0.0.1)
        #[arg(long)]
        host: Option<String>,

        /// 🔇 콘솔 창 없이 백그라운드 데몬으로 실행
        #[arg(short, long)]
        daemon: bool,

        /// 브라우저 대시보드 자동 열기 비활성화
        #[arg(long)]
        no_browser: bool,
    },

    /// ⏹️ 백그라운드에서 실행 중인 게이트웨이 종료
    Stop,

    /// 🔄 백그라운드 게이트웨이 재시작
    Restart {
        /// 리스닝할 포트 번호
        #[arg(short, long)]
        port: Option<u16>,
    },

    /// ⚙️ 윈도우 부팅 시 자동 시작 등록 / 해제 (enable, disable, status)
    Autostart {
        /// enable, disable, 또는 status
        action: String,
    },

    /// 🗑️ 게이트웨이 완전 초기화 및 제거
    Uninstall,

    /// ➕ 새로운 도메인 또는 경로 라우팅 규칙 추가
    Add {
        /// 원본 도메인 또는 경로 (예: app.local 또는 /api)
        source: String,

        /// 대상 포트 또는 전체 URL (예: 5173 또는 http://127.0.0.1:8000)
        target: String,
    },

    /// ➖ 등록된 라우팅 규칙 또는 Mock 제거
    Remove {
        /// 삭제할 도메인 또는 경로 (예: app.local)
        source: String,
    },

    /// 📋 현재 등록된 모든 라우팅 및 Mock 엔드포인트 목록 확인
    List,

    /// 🎭 백엔드 없이 즉각 응답하는 Mock JSON API 엔드포인트 등록
    Mock {
        /// API 경로 (예: /api/user)
        path: String,

        /// 반환할 JSON 데이터 (예: '{"id": 1, "name": "홍길동"}')
        json: String,
    },

    /// 🌐 CORS 헤더 자동 주입 기능 켜기/끄기
    Cors {
        /// enable 또는 disable
        action: String,
    },

    /// 📊 실행 중인 게이트웨이의 실시간 상태 및 메트릭스 조회
    Status {
        /// 게이트웨이 주소 (기본값: http://127.0.0.1:8080)
        #[arg(short, long)]
        url: Option<String>,
    },
}
