use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use socket2::SockRef;
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tauri::Manager;
use tauri::{AppHandle, Emitter, State};
#[cfg(target_os = "android")]
use tauri_plugin_content_access::ContentAccessExt;
use tauri_plugin_notification::NotificationExt;
use tokio::{
    fs::{self, File},
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufWriter},
    net::{TcpListener, TcpStream, UdpSocket},
    sync::{oneshot, Mutex, RwLock, Semaphore},
    time::timeout,
};

const MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
const BROADCAST_ADDR: Ipv4Addr = Ipv4Addr::new(255, 255, 255, 255);
const DISCOVERY_PORT: u16 = 48_888;
const TRANSFER_PORT: u16 = 48_889;
const CHUNK_SIZE: usize = 1024 * 1024;
const TCP_BUFFER_SIZE: usize = 4 * 1024 * 1024;
const MAGIC: &[u8; 8] = b"CDROP002";
const MAX_HEADER_SIZE: usize = 16 * 1024;
const DEFAULT_MAX_FILE_SIZE: u64 = 1024 * 1024 * 1024 * 100;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(8);
const APPROVAL_TIMEOUT: Duration = Duration::from_secs(30);
const IO_IDLE_TIMEOUT: Duration = Duration::from_secs(30);
const STATUS_REJECTED: u8 = 0;
const STATUS_ACCEPTED: u8 = 1;
const STATUS_BUSY: u8 = 2;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub port: u16,
    pub os: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TrustedDevice {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PersistedSettings {
    device_id: String,
    device_name: String,
    trusted_devices: Vec<TrustedDevice>,
    receive_directory: Option<String>,
    max_file_size: u64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PublicSettings {
    device_id: String,
    device_name: String,
    pair_code: String,
    trusted_devices: Vec<TrustedDevice>,
    receive_directory: Option<String>,
    max_file_size: u64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PairCandidate {
    id: String,
    name: String,
    ip: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PairingPayload {
    pub uri: String,
    pub code: String,
    pub ip: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IncomingRequest {
    pub transfer_id: String,
    pub device_id: String,
    pub device_name: String,
    pub file_name: String,
    pub file_size: u64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgress {
    pub transfer_id: String,
    pub direction: TransferDirection,
    pub file_name: String,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub current_mibps: f64,
    pub average_mibps: f64,
    pub peak_mibps: f64,
    pub progress_percent: f64,
    pub eta_seconds: u64,
    pub item_index: usize,
    pub item_count: usize,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum TransferDirection {
    Send,
    Receive,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TransferCompleted {
    pub transfer_id: String,
    pub direction: TransferDirection,
    pub file_name: String,
    pub saved_path: Option<String>,
    pub total_bytes: u64,
    pub average_mibps: f64,
    pub sha256: String,
    pub item_index: usize,
    pub item_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ProtocolRequest {
    Pair {
        sender_id: String,
        sender_name: String,
        code: String,
    },
    File {
        transfer_id: String,
        sender_id: String,
        sender_name: String,
        file_name: String,
        file_size: u64,
        item_index: usize,
        item_count: usize,
    },
}

struct PendingApproval {
    sender: TrustedDevice,
    response: oneshot::Sender<bool>,
}

pub struct NetworkState {
    settings_path: PathBuf,
    settings: RwLock<PersistedSettings>,
    pair_code: String,
    send_gate: Arc<Semaphore>,
    receive_gate: Arc<Semaphore>,
    send_cancel: Mutex<Option<Arc<AtomicBool>>>,
    receive_cancel: Mutex<Option<Arc<AtomicBool>>>,
    pending_approvals: Mutex<HashMap<String, PendingApproval>>,
}

impl NetworkState {
    pub fn load(app: &AppHandle, default_name: String) -> Result<Arc<Self>, String> {
        let config_dir = app
            .path()
            .app_config_dir()
            .map_err(|error| format!("설정 폴더를 찾을 수 없습니다: {error}"))?;
        std::fs::create_dir_all(&config_dir)
            .map_err(|error| format!("설정 폴더를 만들 수 없습니다: {error}"))?;
        let settings_path = config_dir.join("settings.json");
        let settings = std::fs::read(&settings_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<PersistedSettings>(&bytes).ok())
            .unwrap_or_else(|| PersistedSettings {
                device_id: uuid::Uuid::new_v4().to_string(),
                device_name: default_name,
                trusted_devices: Vec::new(),
                receive_directory: None,
                max_file_size: DEFAULT_MAX_FILE_SIZE,
            });
        let code_seed = uuid::Uuid::new_v4().as_u128() as u32 % 1_000_000;
        let snapshot = settings.clone();
        let state = Arc::new(Self {
            settings_path,
            settings: RwLock::new(settings),
            pair_code: format!("{code_seed:06}"),
            send_gate: Arc::new(Semaphore::new(1)),
            receive_gate: Arc::new(Semaphore::new(1)),
            send_cancel: Mutex::new(None),
            receive_cancel: Mutex::new(None),
            pending_approvals: Mutex::new(HashMap::new()),
        });
        std::fs::write(
            &state.settings_path,
            serde_json::to_vec_pretty(&snapshot)
                .map_err(|error| format!("설정을 만들 수 없습니다: {error}"))?,
        )
        .map_err(|error| format!("설정을 저장할 수 없습니다: {error}"))?;
        Ok(state)
    }

    async fn save(&self) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(&*self.settings.read().await)
            .map_err(|error| format!("설정을 만들 수 없습니다: {error}"))?;
        fs::write(&self.settings_path, bytes)
            .await
            .map_err(|error| format!("설정을 저장할 수 없습니다: {error}"))
    }

    async fn identity(&self) -> (String, String) {
        let settings = self.settings.read().await;
        (settings.device_id.clone(), settings.device_name.clone())
    }

    async fn is_trusted(&self, device_id: &str) -> bool {
        self.settings
            .read()
            .await
            .trusted_devices
            .iter()
            .any(|device| device.id == device_id)
    }

    async fn trust(&self, device: TrustedDevice) -> Result<(), String> {
        let mut settings = self.settings.write().await;
        settings.trusted_devices.retain(|item| item.id != device.id);
        settings.trusted_devices.push(device);
        drop(settings);
        self.save().await
    }
}

pub async fn start_discovery_broadcaster(state: Arc<NetworkState>) -> Result<(), String> {
    let socket = UdpSocket::bind("0.0.0.0:0")
        .await
        .map_err(|error| format!("기기 알림 소켓을 열 수 없습니다: {error}"))?;
    socket
        .set_multicast_ttl_v4(1)
        .map_err(|error| format!("멀티캐스트 범위를 설정할 수 없습니다: {error}"))?;
    socket
        .set_broadcast(true)
        .map_err(|error| format!("로컬 브로드캐스트를 설정할 수 없습니다: {error}"))?;
    let destinations = [
        SocketAddr::new(IpAddr::V4(MULTICAST_ADDR), DISCOVERY_PORT),
        SocketAddr::new(IpAddr::V4(BROADCAST_ADDR), DISCOVERY_PORT),
    ];
    loop {
        let (id, name) = state.identity().await;
        let device = DeviceInfo {
            id,
            name,
            ip: "0.0.0.0".to_owned(),
            port: TRANSFER_PORT,
            os: std::env::consts::OS.to_owned(),
        };
        let message = serde_json::to_vec(&device)
            .map_err(|error| format!("기기 정보를 만들 수 없습니다: {error}"))?;
        for destination in destinations {
            if let Err(error) = socket.send_to(&message, destination).await {
                log::warn!("기기 알림 전송 실패: {error}");
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

pub async fn start_discovery_listener(
    app: AppHandle,
    state: Arc<NetworkState>,
) -> Result<(), String> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, DISCOVERY_PORT))
        .await
        .map_err(|error| format!("기기 검색 소켓을 열 수 없습니다: {error}"))?;
    socket
        .join_multicast_v4(MULTICAST_ADDR, Ipv4Addr::UNSPECIFIED)
        .map_err(|error| format!("로컬 기기 검색 그룹에 참가할 수 없습니다: {error}"))?;
    let mut buffer = [0_u8; 2048];
    loop {
        let (length, source) = socket
            .recv_from(&mut buffer)
            .await
            .map_err(|error| format!("기기 검색 데이터를 받을 수 없습니다: {error}"))?;
        let Ok(mut device) = serde_json::from_slice::<DeviceInfo>(&buffer[..length]) else {
            continue;
        };
        if device.id == state.settings.read().await.device_id {
            continue;
        }
        device.ip = source.ip().to_string();
        if let Err(error) = app.emit("device-discovered", device) {
            log::warn!("기기 검색 이벤트 전송 실패: {error}");
        }
    }
}

pub async fn start_tcp_server(app: AppHandle, state: Arc<NetworkState>) -> Result<(), String> {
    let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, TRANSFER_PORT))
        .await
        .map_err(|error| format!("파일 수신 포트를 열 수 없습니다: {error}"))?;
    loop {
        let (socket, _) = listener
            .accept()
            .await
            .map_err(|error| format!("파일 연결을 받을 수 없습니다: {error}"))?;
        configure_receiver(&socket)?;
        let app_handle = app.clone();
        let network_state = state.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = handle_connection(app_handle.clone(), network_state, socket).await {
                let _ = app_handle.emit("transfer-error", error);
            }
        });
    }
}

#[tauri::command]
pub async fn get_app_settings(
    state: State<'_, Arc<NetworkState>>,
) -> Result<PublicSettings, String> {
    let settings = state.settings.read().await.clone();
    Ok(PublicSettings {
        device_id: settings.device_id,
        device_name: settings.device_name,
        pair_code: state.pair_code.clone(),
        trusted_devices: settings.trusted_devices,
        receive_directory: settings.receive_directory,
        max_file_size: settings.max_file_size,
    })
}

#[tauri::command]
pub async fn set_device_name(
    state: State<'_, Arc<NetworkState>>,
    name: String,
) -> Result<(), String> {
    let name = name.trim().chars().take(48).collect::<String>();
    if name.is_empty() {
        return Err("기기 이름을 입력해 주세요.".to_owned());
    }
    state.settings.write().await.device_name = name;
    state.save().await
}

#[tauri::command]
pub async fn set_receive_directory(
    state: State<'_, Arc<NetworkState>>,
    path: Option<String>,
) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        let _ = (state, path);
        return Err("Android 수신 위치는 Download/Vesper Drop으로 고정됩니다.".to_owned());
    }
    #[cfg(not(target_os = "android"))]
    {
        if let Some(ref value) = path {
            let candidate = PathBuf::from(value);
            fs::create_dir_all(&candidate)
                .await
                .map_err(transfer_error)?;
            if !fs::metadata(&candidate)
                .await
                .map_err(transfer_error)?
                .is_dir()
            {
                return Err("수신 위치로 폴더를 선택해 주세요.".to_owned());
            }
        }
        state.settings.write().await.receive_directory = path;
        state.save().await
    }
}

#[tauri::command]
pub async fn forget_device(
    state: State<'_, Arc<NetworkState>>,
    device_id: String,
) -> Result<(), String> {
    state
        .settings
        .write()
        .await
        .trusted_devices
        .retain(|device| device.id != device_id);
    state.save().await
}

#[tauri::command]
pub async fn respond_to_incoming(
    state: State<'_, Arc<NetworkState>>,
    transfer_id: String,
    accept: bool,
    remember: bool,
) -> Result<(), String> {
    let pending = state
        .pending_approvals
        .lock()
        .await
        .remove(&transfer_id)
        .ok_or_else(|| "이미 만료된 수신 요청입니다.".to_owned())?;
    if accept && remember {
        state.trust(pending.sender.clone()).await?;
    }
    let _ = pending.response.send(accept);
    Ok(())
}

#[tauri::command]
pub async fn cancel_transfer(state: State<'_, Arc<NetworkState>>) -> Result<(), String> {
    if let Some(cancel) = state.send_cancel.lock().await.as_ref() {
        cancel.store(true, Ordering::Relaxed);
    }
    if let Some(cancel) = state.receive_cancel.lock().await.as_ref() {
        cancel.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[tauri::command]
pub async fn send_files(
    app: AppHandle,
    state: State<'_, Arc<NetworkState>>,
    target_ip: String,
    file_paths: Vec<String>,
) -> Result<(), String> {
    if file_paths.is_empty() {
        return Err("보낼 파일을 선택해 주세요.".to_owned());
    }
    let network = state.inner().clone();
    let _permit = network
        .send_gate
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| "전송 대기열을 사용할 수 없습니다.".to_owned())?;
    let cancel = Arc::new(AtomicBool::new(false));
    *network.send_cancel.lock().await = Some(cancel.clone());
    let count = file_paths.len();
    let result = async {
        for (index, path) in file_paths.into_iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                return Err("전송을 취소했습니다.".to_owned());
            }
            let source = open_source_file(&app, path).await?;
            send_source(
                &app,
                &network,
                &target_ip,
                source,
                cancel.clone(),
                index + 1,
                count,
            )
            .await?;
        }
        Ok(())
    }
    .await;
    *network.send_cancel.lock().await = None;
    result
}

#[tauri::command]
pub async fn pair_by_code(
    app: AppHandle,
    state: State<'_, Arc<NetworkState>>,
    code: String,
    candidates: Vec<PairCandidate>,
) -> Result<TrustedDevice, String> {
    let code = normalize_pair_code(&code)?;
    for candidate in candidates {
        if pair_with_candidate(&app, state.inner(), &candidate, &code)
            .await
            .is_ok()
        {
            let trusted = TrustedDevice {
                id: candidate.id,
                name: candidate.name,
            };
            state.trust(trusted.clone()).await?;
            return Ok(trusted);
        }
    }
    Err("이 LAN에서 해당 페어링 코드의 기기를 찾지 못했습니다.".to_owned())
}

#[tauri::command]
pub async fn pair_from_qr(
    app: AppHandle,
    state: State<'_, Arc<NetworkState>>,
    candidate: PairCandidate,
    code: String,
) -> Result<TrustedDevice, String> {
    let code = normalize_pair_code(&code)?;
    pair_with_candidate(&app, state.inner(), &candidate, &code).await?;
    let trusted = TrustedDevice {
        id: candidate.id,
        name: candidate.name,
    };
    state.trust(trusted.clone()).await?;
    Ok(trusted)
}

#[tauri::command]
pub async fn get_pairing_payload(
    state: State<'_, Arc<NetworkState>>,
) -> Result<PairingPayload, String> {
    let settings = state.settings.read().await;
    let ip = local_ipv4()?.to_string();
    let name = settings.device_name.replace(['&', '?', '='], "_");
    let uri = format!(
        "vesperdrop://lan-pair?deviceId={}&name={name}&ip={ip}&code={}",
        settings.device_id, state.pair_code
    );
    Ok(PairingPayload {
        uri,
        code: state.pair_code.clone(),
        ip,
    })
}

#[tauri::command]
pub async fn open_received_folder(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "android")]
    app.content_access()
        .open_received_folder()
        .map_err(|error| format!("수신 폴더를 열 수 없습니다: {error}"))?;
    #[cfg(target_os = "windows")]
    {
        let state = app.state::<Arc<NetworkState>>();
        let root = receive_root(&app, state.inner()).await?;
        fs::create_dir_all(&root).await.map_err(transfer_error)?;
        open::that_detached(&root)
            .map_err(|error| format!("수신 폴더를 열 수 없습니다: {error}"))?;
    }
    Ok(())
}

#[tauri::command]
pub fn request_local_network_access(app: AppHandle) -> Result<(), String> {
    #[cfg(target_os = "android")]
    app.content_access()
        .request_local_network_access()
        .map_err(|error| format!("로컬 네트워크 권한을 요청할 수 없습니다: {error}"))?;
    #[cfg(not(target_os = "android"))]
    let _ = app;
    Ok(())
}

#[tauri::command]
pub fn set_background_receive(app: AppHandle, enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        if enabled {
            app.content_access()
                .start_background_service()
                .map_err(|error| format!("백그라운드 수신을 시작할 수 없습니다: {error}"))?;
        } else {
            app.content_access()
                .stop_background_service()
                .map_err(|error| format!("백그라운드 수신을 중지할 수 없습니다: {error}"))?;
        }
    }
    #[cfg(not(target_os = "android"))]
    let _ = (app, enabled);
    Ok(())
}

#[tauri::command]
pub fn scan_pairing_qr(app: AppHandle) -> Result<String, String> {
    #[cfg(target_os = "android")]
    {
        return app
            .content_access()
            .scan_pairing_qr()
            .map_err(|error| format!("QR 코드를 스캔할 수 없습니다: {error}"));
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Err("QR 스캔은 Android에서 사용할 수 있습니다.".to_owned())
    }
}

#[tauri::command]
pub fn pick_folder(app: AppHandle) -> Result<String, String> {
    #[cfg(target_os = "android")]
    {
        return app
            .content_access()
            .pick_folder()
            .map_err(|error| format!("폴더 선택기를 열 수 없습니다: {error}"));
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Err("Android 전용 폴더 선택 명령입니다.".to_owned())
    }
}

struct SourceFile {
    file: File,
    file_name: String,
    file_size: u64,
    cleanup_path: Option<PathBuf>,
}

struct DestinationFile {
    file: File,
    saved_reference: String,
}

struct TransferStats {
    average_mibps: f64,
    sha256: [u8; 32],
}

async fn handle_connection(
    app: AppHandle,
    state: Arc<NetworkState>,
    mut socket: TcpStream,
) -> Result<(), String> {
    let request = read_request(&mut socket).await?;
    match request {
        ProtocolRequest::Pair {
            sender_id,
            sender_name,
            code,
        } => {
            if code != state.pair_code {
                write_status(&mut socket, STATUS_REJECTED).await?;
                return Err("페어링 코드가 일치하지 않습니다.".to_owned());
            }
            state
                .trust(TrustedDevice {
                    id: sender_id,
                    name: sender_name.clone(),
                })
                .await?;
            write_status(&mut socket, STATUS_ACCEPTED).await?;
            let _ = app.emit("device-paired", sender_name);
            Ok(())
        }
        ProtocolRequest::File {
            transfer_id,
            sender_id,
            sender_name,
            file_name,
            file_size,
            item_index,
            item_count,
        } => {
            let permit = match state.receive_gate.clone().try_acquire_owned() {
                Ok(permit) => permit,
                Err(_) => {
                    write_status(&mut socket, STATUS_BUSY).await?;
                    return Ok(());
                }
            };
            let max_size = state.settings.read().await.max_file_size;
            if file_size > max_size {
                write_status(&mut socket, STATUS_REJECTED).await?;
                return Err(format!(
                    "허용된 최대 파일 크기를 초과했습니다: {file_size} bytes"
                ));
            }
            let sender = TrustedDevice {
                id: sender_id.clone(),
                name: sender_name.clone(),
            };
            let accepted = if state.is_trusted(&sender_id).await {
                true
            } else {
                let (response, waiting) = oneshot::channel();
                state
                    .pending_approvals
                    .lock()
                    .await
                    .insert(transfer_id.clone(), PendingApproval { sender, response });
                app.emit(
                    "incoming-request",
                    IncomingRequest {
                        transfer_id: transfer_id.clone(),
                        device_id: sender_id,
                        device_name: sender_name,
                        file_name: sanitize_file_name(&file_name),
                        file_size,
                    },
                )
                .map_err(|error| format!("수신 요청을 표시할 수 없습니다: {error}"))?;
                let _ = app
                    .notification()
                    .builder()
                    .title("Vesper Drop 수신 요청")
                    .body(format!("{file_name} 수신을 확인해 주세요."))
                    .show();
                let result = timeout(APPROVAL_TIMEOUT, waiting)
                    .await
                    .ok()
                    .and_then(Result::ok)
                    .unwrap_or(false);
                state.pending_approvals.lock().await.remove(&transfer_id);
                result
            };
            if !accepted {
                write_status(&mut socket, STATUS_REJECTED).await?;
                return Ok(());
            }
            write_status(&mut socket, STATUS_ACCEPTED).await?;
            let cancel = Arc::new(AtomicBool::new(false));
            *state.receive_cancel.lock().await = Some(cancel.clone());
            let result = receive_file(
                &app,
                &state,
                &mut socket,
                transfer_id,
                sanitize_file_name(&file_name),
                file_size,
                cancel,
                item_index,
                item_count,
            )
            .await;
            *state.receive_cancel.lock().await = None;
            drop(permit);
            result
        }
    }
}

async fn send_source(
    app: &AppHandle,
    state: &Arc<NetworkState>,
    target_ip: &str,
    mut source: SourceFile,
    cancel: Arc<AtomicBool>,
    item_index: usize,
    item_count: usize,
) -> Result<(), String> {
    let result = async {
        let ip = target_ip
            .parse::<IpAddr>()
            .map_err(|_| "올바르지 않은 대상 IP 주소입니다.".to_owned())?;
        let mut socket = timeout(
            CONNECT_TIMEOUT,
            TcpStream::connect(SocketAddr::new(ip, TRANSFER_PORT)),
        )
        .await
        .map_err(|_| "대상 기기 연결 시간이 초과되었습니다.".to_owned())?
        .map_err(|error| format!("대상 기기에 연결할 수 없습니다: {error}"))?;
        configure_sender(&socket)?;
        let (sender_id, sender_name) = state.identity().await;
        let transfer_id = uuid::Uuid::new_v4().to_string();
        write_request(
            &mut socket,
            &ProtocolRequest::File {
                transfer_id: transfer_id.clone(),
                sender_id,
                sender_name,
                file_name: source.file_name.clone(),
                file_size: source.file_size,
                item_index,
                item_count,
            },
        )
        .await?;
        match read_status(&mut socket, APPROVAL_TIMEOUT).await? {
            STATUS_ACCEPTED => {}
            STATUS_BUSY => return Err("상대 기기가 다른 파일을 받고 있습니다.".to_owned()),
            _ => return Err("상대 기기에서 수신을 거절했거나 요청이 만료되었습니다.".to_owned()),
        }
        let stats = copy_with_progress(
            app,
            &mut source.file,
            &mut socket,
            &transfer_id,
            &source.file_name,
            source.file_size,
            TransferDirection::Send,
            cancel,
            item_index,
            item_count,
        )
        .await?;
        timed_write_all(&mut socket, &stats.sha256).await?;
        timed_flush(&mut socket).await?;
        if read_status(&mut socket, IO_IDLE_TIMEOUT).await? != STATUS_ACCEPTED {
            return Err("수신 측 파일 검증에 실패했습니다.".to_owned());
        }
        let hash = to_hex(&stats.sha256);
        app.emit(
            "transfer-completed",
            TransferCompleted {
                transfer_id,
                direction: TransferDirection::Send,
                file_name: source.file_name.clone(),
                saved_path: None,
                total_bytes: source.file_size,
                average_mibps: stats.average_mibps,
                sha256: hash,
                item_index,
                item_count,
            },
        )
        .map_err(|error| format!("완료 상태를 표시할 수 없습니다: {error}"))?;
        Ok(())
    }
    .await;
    cleanup_source(&source).await;
    result
}

async fn receive_file(
    app: &AppHandle,
    state: &Arc<NetworkState>,
    socket: &mut TcpStream,
    transfer_id: String,
    file_name: String,
    file_size: u64,
    cancel: Arc<AtomicBool>,
    item_index: usize,
    item_count: usize,
) -> Result<(), String> {
    let destination = open_destination_file(app, state, &file_name).await?;
    let saved_reference = destination.saved_reference;
    let mut writer = BufWriter::with_capacity(CHUNK_SIZE, destination.file);
    let result = async {
        let stats = copy_with_progress(
            app,
            socket,
            &mut writer,
            &transfer_id,
            &file_name,
            file_size,
            TransferDirection::Receive,
            cancel,
            item_index,
            item_count,
        )
        .await?;
        let mut expected_hash = [0_u8; 32];
        timed_read_exact(socket, &mut expected_hash).await?;
        if stats.sha256 != expected_hash {
            return Err("파일 체크섬이 일치하지 않습니다.".to_owned());
        }
        timed_flush(&mut writer).await?;
        writer.get_ref().sync_all().await.map_err(transfer_error)?;
        Ok(stats)
    }
    .await;
    drop(writer);
    let stats = match result {
        Ok(stats) => stats,
        Err(error) => {
            let _ = finish_destination_file(app, &saved_reference, false).await;
            let _ = write_status(socket, STATUS_REJECTED).await;
            return Err(error);
        }
    };
    finish_destination_file(app, &saved_reference, true).await?;
    write_status(socket, STATUS_ACCEPTED).await?;
    let hash = to_hex(&stats.sha256);
    app.emit(
        "transfer-completed",
        TransferCompleted {
            transfer_id,
            direction: TransferDirection::Receive,
            file_name: file_name.clone(),
            saved_path: Some(saved_reference),
            total_bytes: file_size,
            average_mibps: stats.average_mibps,
            sha256: hash,
            item_index,
            item_count,
        },
    )
    .map_err(|error| format!("완료 상태를 표시할 수 없습니다: {error}"))?;
    let _ = app
        .notification()
        .builder()
        .title("Vesper Drop 수신 완료")
        .body(format!("{file_name} 저장이 완료되었습니다."))
        .show();
    Ok(())
}

async fn copy_with_progress<R, W>(
    app: &AppHandle,
    reader: &mut R,
    writer: &mut W,
    transfer_id: &str,
    file_name: &str,
    total_bytes: u64,
    direction: TransferDirection,
    cancel: Arc<AtomicBool>,
    item_index: usize,
    item_count: usize,
) -> Result<TransferStats, String>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut buffer = vec![0_u8; CHUNK_SIZE];
    let started_at = Instant::now();
    let mut sample_at = Instant::now();
    let mut sample_bytes = 0_u64;
    let mut transferred = 0_u64;
    let mut peak = 0.0_f64;
    let mut hasher = Sha256::new();
    emit_progress(
        app,
        transfer_id,
        file_name,
        total_bytes,
        0,
        0.0,
        0.0,
        0.0,
        direction.clone(),
        item_index,
        item_count,
    )?;
    while transferred < total_bytes {
        if cancel.load(Ordering::Relaxed) {
            return Err("전송을 취소했습니다.".to_owned());
        }
        let read_limit = (total_bytes - transferred).min(CHUNK_SIZE as u64) as usize;
        let count = timeout(IO_IDLE_TIMEOUT, reader.read(&mut buffer[..read_limit]))
            .await
            .map_err(|_| "파일 전송이 30초 동안 응답하지 않았습니다.".to_owned())?
            .map_err(transfer_error)?;
        if count == 0 {
            return Err("파일 전송이 예정보다 일찍 종료되었습니다.".to_owned());
        }
        timeout(IO_IDLE_TIMEOUT, writer.write_all(&buffer[..count]))
            .await
            .map_err(|_| "파일 전송이 30초 동안 응답하지 않았습니다.".to_owned())?
            .map_err(transfer_error)?;
        hasher.update(&buffer[..count]);
        transferred += count as u64;
        if sample_at.elapsed() >= Duration::from_millis(100) || transferred == total_bytes {
            let sample_seconds = sample_at.elapsed().as_secs_f64().max(0.001);
            let current = (transferred - sample_bytes) as f64 / sample_seconds / 1_048_576.0;
            let elapsed = started_at.elapsed().as_secs_f64().max(0.001);
            let average = transferred as f64 / elapsed / 1_048_576.0;
            peak = peak.max(current);
            emit_progress(
                app,
                transfer_id,
                file_name,
                total_bytes,
                transferred,
                current,
                average,
                peak,
                direction.clone(),
                item_index,
                item_count,
            )?;
            sample_at = Instant::now();
            sample_bytes = transferred;
        }
    }
    let elapsed = started_at.elapsed().as_secs_f64().max(0.001);
    Ok(TransferStats {
        average_mibps: transferred as f64 / elapsed / 1_048_576.0,
        sha256: hasher.finalize().into(),
    })
}

#[allow(clippy::too_many_arguments)]
fn emit_progress(
    app: &AppHandle,
    transfer_id: &str,
    file_name: &str,
    total_bytes: u64,
    transferred: u64,
    current_mibps: f64,
    average_mibps: f64,
    peak_mibps: f64,
    direction: TransferDirection,
    item_index: usize,
    item_count: usize,
) -> Result<(), String> {
    let remaining = total_bytes.saturating_sub(transferred);
    let progress = if total_bytes == 0 {
        100.0
    } else {
        transferred as f64 / total_bytes as f64 * 100.0
    };
    let eta = if average_mibps > 0.0 {
        (remaining as f64 / (average_mibps * 1_048_576.0)).ceil() as u64
    } else {
        0
    };
    app.emit(
        "transfer-progress",
        TransferProgress {
            transfer_id: transfer_id.to_owned(),
            direction,
            file_name: file_name.to_owned(),
            bytes_transferred: transferred,
            total_bytes,
            current_mibps,
            average_mibps,
            peak_mibps,
            progress_percent: progress.min(100.0),
            eta_seconds: eta,
            item_index,
            item_count,
        },
    )
    .map_err(|error| format!("전송 상태를 표시할 수 없습니다: {error}"))
}

async fn write_request(socket: &mut TcpStream, request: &ProtocolRequest) -> Result<(), String> {
    let bytes = serde_json::to_vec(request)
        .map_err(|error| format!("연결 정보를 만들 수 없습니다: {error}"))?;
    if bytes.len() > MAX_HEADER_SIZE {
        return Err("연결 정보가 너무 큽니다.".to_owned());
    }
    timed_write_all(socket, MAGIC).await?;
    timed_write_all(socket, &(bytes.len() as u32).to_be_bytes()).await?;
    timed_write_all(socket, &bytes).await?;
    timed_flush(socket).await
}

async fn read_request(socket: &mut TcpStream) -> Result<ProtocolRequest, String> {
    let mut magic = [0_u8; MAGIC.len()];
    timed_read_exact(socket, &mut magic).await?;
    if &magic != MAGIC {
        return Err("Vesper Drop 파일 연결이 아닙니다.".to_owned());
    }
    let mut length = [0_u8; 4];
    timed_read_exact(socket, &mut length).await?;
    let length = u32::from_be_bytes(length) as usize;
    if length == 0 || length > MAX_HEADER_SIZE {
        return Err("연결 정보의 크기가 올바르지 않습니다.".to_owned());
    }
    let mut bytes = vec![0_u8; length];
    timed_read_exact(socket, &mut bytes).await?;
    serde_json::from_slice(&bytes).map_err(|_| "연결 정보가 손상되었습니다.".to_owned())
}

async fn pair_with_candidate(
    _app: &AppHandle,
    state: &Arc<NetworkState>,
    candidate: &PairCandidate,
    code: &str,
) -> Result<(), String> {
    let ip = candidate
        .ip
        .parse::<IpAddr>()
        .map_err(|_| "페어링 대상 IP가 올바르지 않습니다.".to_owned())?;
    let mut socket = timeout(
        CONNECT_TIMEOUT,
        TcpStream::connect(SocketAddr::new(ip, TRANSFER_PORT)),
    )
    .await
    .map_err(|_| "페어링 연결 시간이 초과되었습니다.".to_owned())?
    .map_err(transfer_error)?;
    let (sender_id, sender_name) = state.identity().await;
    write_request(
        &mut socket,
        &ProtocolRequest::Pair {
            sender_id,
            sender_name,
            code: code.to_owned(),
        },
    )
    .await?;
    if read_status(&mut socket, CONNECT_TIMEOUT).await? == STATUS_ACCEPTED {
        Ok(())
    } else {
        Err("페어링 코드가 일치하지 않습니다.".to_owned())
    }
}

async fn open_source_file(app: &AppHandle, file_path: String) -> Result<SourceFile, String> {
    #[cfg(target_os = "android")]
    if file_path.starts_with("content://") {
        use std::os::fd::{FromRawFd, OwnedFd};
        let content = app
            .content_access()
            .open_uri(file_path)
            .map_err(|error| format!("선택한 파일을 읽을 수 없습니다: {error}"))?;
        if content.fd < 0 {
            return Err("Android 파일 디스크립터가 올바르지 않습니다.".to_owned());
        }
        let owned_fd = unsafe { OwnedFd::from_raw_fd(content.fd) };
        return Ok(SourceFile {
            file: File::from_std(std::fs::File::from(owned_fd)),
            file_name: sanitize_file_name(&content.file_name),
            file_size: content.file_size,
            cleanup_path: None,
        });
    }
    let path = PathBuf::from(file_path);
    let metadata = fs::metadata(&path)
        .await
        .map_err(|error| format!("선택한 항목을 읽을 수 없습니다: {error}"))?;
    if metadata.is_dir() {
        return archive_directory(app, path).await;
    }
    if !metadata.is_file() {
        return Err("파일 또는 폴더를 선택해 주세요.".to_owned());
    }
    Ok(SourceFile {
        file: File::open(&path).await.map_err(transfer_error)?,
        file_name: safe_file_name(&path),
        file_size: metadata.len(),
        cleanup_path: None,
    })
}

#[cfg(not(target_os = "android"))]
async fn archive_directory(app: &AppHandle, source: PathBuf) -> Result<SourceFile, String> {
    use std::io::{Read, Write};
    use walkdir::WalkDir;
    use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};
    let cache = app
        .path()
        .app_cache_dir()
        .map_err(|error| format!("임시 폴더를 찾을 수 없습니다: {error}"))?;
    fs::create_dir_all(&cache).await.map_err(transfer_error)?;
    let archive = cache.join(format!("folder-{}.zip", uuid::Uuid::new_v4()));
    let archive_clone = archive.clone();
    let source_clone = source.clone();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let file = std::fs::File::create(&archive_clone).map_err(transfer_error)?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let prefix = source_clone.parent().unwrap_or(&source_clone);
        let mut buffer = vec![0_u8; 128 * 1024];
        for entry in WalkDir::new(&source_clone) {
            let entry = entry.map_err(|error| format!("폴더를 읽을 수 없습니다: {error}"))?;
            let relative = entry
                .path()
                .strip_prefix(prefix)
                .map_err(|error| format!("폴더 경로를 만들 수 없습니다: {error}"))?
                .to_string_lossy()
                .replace('\\', "/");
            if entry.file_type().is_dir() {
                zip.add_directory(format!("{relative}/"), options)
                    .map_err(|error| format!("폴더 압축에 실패했습니다: {error}"))?;
            } else {
                zip.start_file(relative, options)
                    .map_err(|error| format!("폴더 압축에 실패했습니다: {error}"))?;
                let mut input = std::fs::File::open(entry.path()).map_err(transfer_error)?;
                loop {
                    let count = input.read(&mut buffer).map_err(transfer_error)?;
                    if count == 0 {
                        break;
                    }
                    zip.write_all(&buffer[..count]).map_err(transfer_error)?;
                }
            }
        }
        zip.finish()
            .map_err(|error| format!("폴더 압축을 마무리할 수 없습니다: {error}"))?;
        Ok(())
    })
    .await
    .map_err(|error| format!("폴더 압축 작업이 중단되었습니다: {error}"))??;
    let size = fs::metadata(&archive).await.map_err(transfer_error)?.len();
    let folder_name = source
        .file_name()
        .and_then(|value| value.to_str())
        .map(sanitize_file_name)
        .unwrap_or_else(|| "folder".to_owned());
    Ok(SourceFile {
        file: File::open(&archive).await.map_err(transfer_error)?,
        file_name: format!("{folder_name}.zip"),
        file_size: size,
        cleanup_path: Some(archive),
    })
}

#[cfg(target_os = "android")]
async fn archive_directory(_app: &AppHandle, _source: PathBuf) -> Result<SourceFile, String> {
    Err("Android 폴더는 시스템 폴더 선택기를 사용해 주세요.".to_owned())
}

async fn cleanup_source(source: &SourceFile) {
    if let Some(path) = &source.cleanup_path {
        let _ = fs::remove_file(path).await;
    }
}

async fn open_destination_file(
    app: &AppHandle,
    state: &Arc<NetworkState>,
    file_name: &str,
) -> Result<DestinationFile, String> {
    #[cfg(target_os = "android")]
    {
        let _ = state;
        use std::os::fd::{FromRawFd, OwnedFd};
        let destination = app
            .content_access()
            .create_received_file(file_name.to_owned())
            .map_err(|error| format!("수신 파일을 만들 수 없습니다: {error}"))?;
        if destination.fd < 0 {
            return Err("Android 수신 파일 디스크립터가 올바르지 않습니다.".to_owned());
        }
        let owned_fd = unsafe { OwnedFd::from_raw_fd(destination.fd) };
        Ok(DestinationFile {
            file: File::from_std(std::fs::File::from(owned_fd)),
            saved_reference: destination.uri,
        })
    }
    #[cfg(not(target_os = "android"))]
    {
        let destination = available_download_path(app, state, file_name).await?;
        let file = File::options()
            .create_new(true)
            .write(true)
            .open(&destination)
            .await
            .map_err(transfer_error)?;
        Ok(DestinationFile {
            file,
            saved_reference: destination.to_string_lossy().into_owned(),
        })
    }
}

async fn finish_destination_file(
    app: &AppHandle,
    saved_reference: &str,
    success: bool,
) -> Result<(), String> {
    #[cfg(target_os = "android")]
    return app
        .content_access()
        .finish_received_file(saved_reference.to_owned(), success)
        .map_err(|error| format!("수신 파일을 마무리할 수 없습니다: {error}"));
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        if !success {
            let _ = fs::remove_file(saved_reference).await;
        }
        Ok(())
    }
}

#[cfg(not(target_os = "android"))]
async fn available_download_path(
    app: &AppHandle,
    state: &Arc<NetworkState>,
    file_name: &str,
) -> Result<PathBuf, String> {
    let root = receive_root(app, state).await?;
    fs::create_dir_all(&root).await.map_err(transfer_error)?;
    let path = Path::new(file_name);
    let stem = path.file_stem().and_then(|v| v.to_str()).unwrap_or("file");
    let extension = path.extension().and_then(|v| v.to_str());
    for index in 0..10_000 {
        let name = if index == 0 {
            file_name.to_owned()
        } else {
            match extension {
                Some(extension) => format!("{stem} ({index}).{extension}"),
                None => format!("{stem} ({index})"),
            }
        };
        let candidate = root.join(name);
        if !fs::try_exists(&candidate).await.map_err(transfer_error)? {
            return Ok(candidate);
        }
    }
    Err("같은 이름의 파일이 너무 많아 저장할 수 없습니다.".to_owned())
}

#[cfg(not(target_os = "android"))]
async fn receive_root(app: &AppHandle, state: &Arc<NetworkState>) -> Result<PathBuf, String> {
    if let Some(path) = state.settings.read().await.receive_directory.clone() {
        return Ok(PathBuf::from(path));
    }
    Ok(app
        .path()
        .download_dir()
        .map_err(|error| format!("다운로드 폴더를 찾을 수 없습니다: {error}"))?
        .join("Vesper Drop"))
}

fn safe_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(sanitize_file_name)
        .unwrap_or_else(|| "file".to_owned())
}

fn sanitize_file_name(file_name: &str) -> String {
    let sanitized: String = file_name
        .chars()
        .map(|character| match character {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '\0' => '_',
            value if value.is_control() => '_',
            value => value,
        })
        .collect();
    let trimmed = sanitized.trim().trim_matches(['.', ' ']);
    let candidate: String = if trimmed.is_empty() {
        "file".to_owned()
    } else {
        trimmed.chars().take(180).collect()
    };
    let stem = Path::new(&candidate)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_uppercase();
    let reserved = ["CON", "PRN", "AUX", "NUL", "CLOCK$"];
    if reserved.contains(&stem.as_str())
        || (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem[3..].parse::<u8>().is_ok_and(|number| number <= 9)
    {
        format!("_{candidate}")
    } else {
        candidate
    }
}

fn normalize_pair_code(code: &str) -> Result<String, String> {
    let code = code
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>();
    if code.len() != 6 {
        return Err("6자리 페어링 코드를 입력해 주세요.".to_owned());
    }
    Ok(code)
}

fn local_ipv4() -> Result<Ipv4Addr, String> {
    let socket = std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).map_err(transfer_error)?;
    socket
        .connect((Ipv4Addr::new(192, 0, 2, 1), 9))
        .map_err(transfer_error)?;
    match socket.local_addr().map_err(transfer_error)?.ip() {
        IpAddr::V4(ip) => Ok(ip),
        _ => Err("로컬 IPv4 주소를 찾지 못했습니다.".to_owned()),
    }
}

fn transfer_error(error: std::io::Error) -> String {
    format!("파일 전송 중 오류가 발생했습니다: {error}")
}

async fn timed_read_exact<R: AsyncRead + Unpin>(
    reader: &mut R,
    buffer: &mut [u8],
) -> Result<(), String> {
    timeout(IO_IDLE_TIMEOUT, reader.read_exact(buffer))
        .await
        .map_err(|_| "네트워크 응답 시간이 초과되었습니다.".to_owned())?
        .map(|_| ())
        .map_err(transfer_error)
}

async fn timed_write_all<W: AsyncWrite + Unpin>(
    writer: &mut W,
    bytes: &[u8],
) -> Result<(), String> {
    timeout(IO_IDLE_TIMEOUT, writer.write_all(bytes))
        .await
        .map_err(|_| "네트워크 전송 시간이 초과되었습니다.".to_owned())?
        .map_err(transfer_error)
}

async fn timed_flush<W: AsyncWrite + Unpin>(writer: &mut W) -> Result<(), String> {
    timeout(IO_IDLE_TIMEOUT, writer.flush())
        .await
        .map_err(|_| "네트워크 전송 시간이 초과되었습니다.".to_owned())?
        .map_err(transfer_error)
}

async fn read_status(socket: &mut TcpStream, duration: Duration) -> Result<u8, String> {
    let mut status = [0_u8; 1];
    timeout(duration, socket.read_exact(&mut status))
        .await
        .map_err(|_| "상대 기기 응답 시간이 초과되었습니다.".to_owned())?
        .map_err(transfer_error)?;
    Ok(status[0])
}

async fn write_status(socket: &mut TcpStream, status: u8) -> Result<(), String> {
    timed_write_all(socket, &[status]).await?;
    timed_flush(socket).await
}

fn configure_receiver(socket: &TcpStream) -> Result<(), String> {
    socket.set_nodelay(true).map_err(transfer_error)?;
    SockRef::from(socket)
        .set_recv_buffer_size(TCP_BUFFER_SIZE)
        .map_err(transfer_error)
}

fn configure_sender(socket: &TcpStream) -> Result<(), String> {
    socket.set_nodelay(true).map_err(transfer_error)?;
    SockRef::from(socket)
        .set_send_buffer_size(TCP_BUFFER_SIZE)
        .map_err(transfer_error)
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::{normalize_pair_code, sanitize_file_name};

    #[test]
    fn removes_path_traversal_and_windows_reserved_characters() {
        assert_eq!(
            sanitize_file_name("../../bad:name?.txt"),
            "_.._bad_name_.txt"
        );
        assert_eq!(sanitize_file_name("CON.txt"), "_CON.txt");
    }

    #[test]
    fn substitutes_empty_file_name() {
        assert_eq!(sanitize_file_name("..."), "file");
    }

    #[test]
    fn validates_pair_code() {
        assert_eq!(normalize_pair_code("123 456").unwrap(), "123456");
        assert!(normalize_pair_code("12345").is_err());
    }
}
