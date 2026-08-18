use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use instant_acme::{
    Account, AccountCredentials, AuthorizationStatus, ChallengeType, Identifier,
    LetsEncrypt, NewAccount, NewOrder, OrderStatus,
};
use rcgen::{CertificateParams, KeyPair};
use colored::*;
use crate::error::{GatewayError, Result};

/// ACME HTTP-01 챌린지 토큰 저장소
#[derive(Clone, Default)]
pub struct ChallengeStore {
    tokens: Arc<RwLock<HashMap<String, String>>>,
}

impl ChallengeStore {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn set_token(&self, token: String, key_authorization: String) {
        let mut map = self.tokens.write().unwrap();
        map.insert(token, key_authorization);
    }

    pub fn get_token(&self, token: &str) -> Option<String> {
        let map = self.tokens.read().unwrap();
        map.get(token).cloned()
    }

    pub fn remove_token(&self, token: &str) {
        let mut map = self.tokens.write().unwrap();
        map.remove(token);
    }
}

/// Let's Encrypt 정식 SSL 발급 관리자
pub struct AcmeManager {
    challenge_store: ChallengeStore,
    certs_dir: PathBuf,
}

impl AcmeManager {
    pub fn new(challenge_store: ChallengeStore) -> Self {
        let certs_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".pgate")
            .join("certs");
        
        let _ = fs::create_dir_all(&certs_dir);

        Self {
            challenge_store,
            certs_dir,
        }
    }

    /// 인증서 저장 디렉토리 반환
    pub fn get_certs_dir(&self) -> &Path {
        &self.certs_dir
    }

    /// 특정 도메인의 정식 인증서 파일 경로 (cert.pem, key.pem)
    pub fn get_domain_cert_paths(&self, domain: &str) -> (PathBuf, PathBuf) {
        let cert_path = self.certs_dir.join(format!("{}.crt", domain));
        let key_path = self.certs_dir.join(format!("{}.key", domain));
        (cert_path, key_path)
    }

    /// 특정 도메인의 발급된 정식 인증서가 존재하는지 확인
    pub fn has_valid_cert(&self, domain: &str) -> bool {
        let (cert_path, key_path) = self.get_domain_cert_paths(domain);
        cert_path.exists() && key_path.exists()
    }

    /// Let's Encrypt 정식 SSL 발급 요청 (HTTP-01 챌린지 방식)
    pub async fn request_certificate(
        &self,
        domain: &str,
        email: &str,
        use_staging: bool,
    ) -> Result<String> {
        let directory_url = if use_staging {
            LetsEncrypt::Staging.url()
        } else {
            LetsEncrypt::Production.url()
        };

        println!(
            "{} Let's Encrypt SSL 발급 시작: {} (이메일: {}, Staging: {})",
            "🔒 [ACME SSL]".cyan().bold(),
            domain.yellow().bold(),
            email,
            use_staging
        );

        // 1. 계정 자격 증명 로드 또는 신규 생성
        let account_creds_path = self.certs_dir.join("acme_account.json");
        let mut account = if account_creds_path.exists() {
            let creds_json = fs::read_to_string(&account_creds_path)
                .map_err(|e| GatewayError::ConfigError(format!("계정 파일 읽기 실패: {}", e)))?;
            let creds: AccountCredentials = serde_json::from_str(&creds_json)
                .map_err(|e| GatewayError::ConfigError(format!("계정 파싱 실패: {}", e)))?;
            Account::from_credentials(creds)
                .await
                .map_err(|e| GatewayError::ConfigError(format!("ACME 계정 로드 실패: {}", e)))?
        } else {
            let contact = vec![format!("mailto:{}", email)];
            let (acc, creds) = Account::create(
                &NewAccount {
                    contact: &contact.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
                    terms_of_service_agreed: true,
                    only_return_existing: false,
                },
                directory_url,
                None,
            )
            .await
            .map_err(|e| GatewayError::ConfigError(format!("ACME 계정 생성 실패: {}", e)))?;

            let creds_json = serde_json::to_string_pretty(&creds)
                .map_err(|e| GatewayError::ConfigError(format!("계정 저장 실패: {}", e)))?;
            let _ = fs::write(&account_creds_path, creds_json);
            acc
        };

        // 2. 신규 주문(Order) 생성
        let identifier = Identifier::Dns(domain.to_string());
        let mut order = account
            .new_order(&NewOrder {
                identifiers: &[identifier],
            })
            .await
            .map_err(|e| GatewayError::ConfigError(format!("ACME 주문 생성 실패: {}", e)))?;

        let state = order.state();
        if !matches!(state.status, OrderStatus::Pending) {
            return Err(GatewayError::ConfigError(format!("예상치 못한 주문 상태: {:?}", state.status)));
        }

        // 3. Authorization 및 HTTP-01 챌린지 획득
        let authorizations = order
            .authorizations()
            .await
            .map_err(|e| GatewayError::ConfigError(format!("인증 목록 조회 실패: {}", e)))?;

        let mut pending_challenges = Vec::new();

        for auth in authorizations {
            if auth.status == AuthorizationStatus::Pending {
                let challenge = auth
                    .challenges
                    .iter()
                    .find(|c| c.r#type == ChallengeType::Http01)
                    .ok_or_else(|| GatewayError::ConfigError("HTTP-01 챌린지를 찾을 수 없습니다.".to_string()))?;

                let key_auth = order.key_authorization(challenge);
                self.challenge_store.set_token(challenge.token.clone(), key_auth.as_str().to_string());
                pending_challenges.push(challenge.url.clone());
            }
        }

        // 4. Let's Encrypt에 챌린지 검증 요청
        for challenge_url in pending_challenges {
            order
                .set_challenge_ready(&challenge_url)
                .await
                .map_err(|e| GatewayError::ConfigError(format!("챌린지 준비 신호 전송 실패: {}", e)))?;
        }

        // 5. 주문 상태 폴링 (Ready/Valid 대기)
        let mut retries = 0;
        let final_state = loop {
            tokio::time::sleep(Duration::from_secs(3)).await;
            let state = order
                .refresh()
                .await
                .map_err(|e| GatewayError::ConfigError(format!("주문 갱신 실패: {}", e)))?;

            match state.status {
                OrderStatus::Ready | OrderStatus::Valid => break state,
                OrderStatus::Invalid => {
                    return Err(GatewayError::ConfigError("ACME 도메인 소유권 검증에 실패했습니다. 포트 80과 도메인 라우팅을 확인하세요.".to_string()));
                }
                OrderStatus::Pending | OrderStatus::Processing => {
                    retries += 1;
                    if retries > 20 {
                        return Err(GatewayError::ConfigError("ACME 인증 시간 초과 (60초 초과)".to_string()));
                    }
                }
            }
        };

        println!("  • 도메인 소유권 검증 완료! (Status: {:?})", final_state.status);

        // 6. CSR 생성 및 인증서 서명 요청 (Finalize)
        let mut params = CertificateParams::new(vec![domain.to_string()])
            .map_err(|e| GatewayError::ConfigError(format!("인증서 파라미터 생성 실패: {}", e)))?;
        params.distinguished_name.push(rcgen::DnType::CommonName, domain);

        let key_pair = KeyPair::generate()
            .map_err(|e| GatewayError::ConfigError(format!("RSA/ECDSA 키페어 생성 실패: {}", e)))?;

        let csr = params
            .serialize_request(&key_pair)
            .map_err(|e| GatewayError::ConfigError(format!("CSR 생성 실패: {}", e)))?;

        order
            .finalize(csr.der())
            .await
            .map_err(|e| GatewayError::ConfigError(format!("인증서 발급 요청(Finalize) 실패: {}", e)))?;

        // 7. 발급 완료 대기 및 인증서 다운로드
        let mut cert_retries = 0;
        let cert_chain_pem = loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            if let Some(cert) = order
                .certificate()
                .await
                .map_err(|e| GatewayError::ConfigError(format!("인증서 다운로드 실패: {}", e)))?
            {
                break cert;
            }
            cert_retries += 1;
            if cert_retries > 15 {
                return Err(GatewayError::ConfigError("인증서 생성 대기 시간 초과".to_string()));
            }
        };

        // 8. 개인키 및 인증서 체인 로컬 파일 저장
        let (cert_path, key_path) = self.get_domain_cert_paths(domain);
        let private_key_pem = key_pair.serialize_pem();

        fs::write(&cert_path, cert_chain_pem)
            .map_err(|e| GatewayError::ConfigError(format!("인증서 파일 저장 실패: {}", e)))?;
        fs::write(&key_path, private_key_pem)
            .map_err(|e| GatewayError::ConfigError(format!("개인키 파일 저장 실패: {}", e)))?;

        println!(
            "{} Let's Encrypt 정식 SSL 발급 완료! (저장: {})",
            "✔ [SSL 완료]".green().bold(),
            cert_path.display()
        );

        Ok(format!("도메인 '{}'에 대한 Let's Encrypt 정식 SSL 발급이 성공하였습니다.", domain))
    }
}
