use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;
use colored::*;
use crate::error::{GatewayError, Result};

/// 로컬 HTTPS/TLS용 인증서 및 서버 설정 로드/생성
/// 공인 도메인의 정식 Let's Encrypt 인증서가 존재하면 우선 로드, 없을 시 자체 서명 인증서 사용
pub fn load_or_generate_tls_config(preferred_domain: Option<&str>) -> Result<TlsAcceptor> {
    let certs_dir = get_certs_dir()?;

    // 1. 도메인별 정식 Let's Encrypt 인증서 확인
    let (cert_path, key_path) = if let Some(domain) = preferred_domain {
        let domain_cert = certs_dir.join(format!("{}.crt", domain));
        let domain_key = certs_dir.join(format!("{}.key", domain));
        if domain_cert.exists() && domain_key.exists() {
            println!("  • {} 정식 Let's Encrypt SSL 인증서 로드: {}", "🔒 [SSL]".green().bold(), domain.yellow());
            (domain_cert, domain_key)
        } else {
            get_or_create_self_signed(&certs_dir)?
        }
    } else {
        // 기본 디렉토리 내에 발급된 .crt 파일이 있는지 탐색
        let mut found_custom = None;
        if let Ok(entries) = fs::read_dir(&certs_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|s| s.to_str()) == Some("crt") {
                    let file_stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                    let key_p = certs_dir.join(format!("{}.key", file_stem));
                    if key_p.exists() {
                        found_custom = Some((p, key_p, file_stem));
                        break;
                    }
                }
            }
        }

        if let Some((c_path, k_path, stem)) = found_custom {
            println!("  • {} 정식 Let's Encrypt SSL 인증서 로드: {}", "🔒 [SSL]".green().bold(), stem.yellow());
            (c_path, k_path)
        } else {
            get_or_create_self_signed(&certs_dir)?
        }
    };

    let cert_file = fs::File::open(&cert_path)?;
    let mut cert_reader = std::io::BufReader::new(cert_file);
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| GatewayError::ConfigError(format!("인증서 읽기 실패: {}", e)))?;

    let key_file = fs::File::open(&key_path)?;
    let mut key_reader = std::io::BufReader::new(key_file);
    let key: PrivateKeyDer<'static> = rustls_pemfile::private_key(&mut key_reader)
        .map_err(|e| GatewayError::ConfigError(format!("개인키 파싱 실패: {}", e)))?
        .ok_or_else(|| GatewayError::ConfigError("유효한 개인키를 찾을 수 없습니다.".to_string()))?;

    let mut server_config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| GatewayError::ConfigError(format!("TLS 서버 설정 실패: {}", e)))?;

    server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(TlsAcceptor::from(Arc::new(server_config)))
}

fn get_or_create_self_signed(certs_dir: &Path) -> Result<(PathBuf, PathBuf)> {
    let cert_path = certs_dir.join("cert.pem");
    let key_path = certs_dir.join("key.pem");

    if !cert_path.exists() || !key_path.exists() {
        generate_self_signed_cert(&cert_path, &key_path)?;
    }

    Ok((cert_path, key_path))
}

fn get_certs_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| {
        GatewayError::ConfigError("홈 디렉토리를 찾을 수 없습니다.".to_string())
    })?;
    let certs_dir = home.join(".pgate").join("certs");
    if !certs_dir.exists() {
        fs::create_dir_all(&certs_dir)?;
    }
    Ok(certs_dir)
}

fn generate_self_signed_cert(cert_path: &PathBuf, key_path: &PathBuf) -> Result<()> {
    let subject_alt_names = vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "*.local".to_string(),
        "app.local".to_string(),
        "api.local".to_string(),
        "*.minseok.online".to_string(),
    ];

    let certified_key = rcgen::generate_simple_self_signed(subject_alt_names)
        .map_err(|e| GatewayError::ConfigError(format!("자체 서명 인증서 생성 실패: {}", e)))?;

    let cert_pem = certified_key.cert.pem();
    let key_pem = certified_key.key_pair.serialize_pem();

    fs::write(cert_path, cert_pem)?;
    fs::write(key_path, key_pem)?;

    Ok(())
}
