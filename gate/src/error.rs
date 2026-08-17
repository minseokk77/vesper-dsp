use thiserror::Error;

/// pgate 게이트웨이 전용 에러 정의 (친숙한 한국어 에러 메시지)
#[derive(Error, Debug)]
pub enum GatewayError {
    #[error("설정 파일 접근/파싱 실패: {0}")]
    ConfigError(String),

    #[error("네트워크 바인딩 또는 리스닝 실패: {0}")]
    NetworkError(String),

    #[error("백엔드 대상 서버에 연결할 수 없습니다 ({0}). 서버가 켜져 있는지 확인하세요.")]
    BackendUnreachable(String),

    #[error("유효하지 않은 요청 주소 또는 도메인입니다: {0}")]
    InvalidTarget(String),

    #[error("I/O 작업 실패: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, GatewayError>;
