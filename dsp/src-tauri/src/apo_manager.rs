use std::sync::atomic::Ordering;
use once_cell::sync::Lazy;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::Memory::{
    CreateFileMappingW, MapViewOfFile, FILE_MAP_ALL_ACCESS, PAGE_READWRITE,
};

pub const SHARED_MEMORY_NAME: &str = "Global\\VesperDspApoSharedMemory";
pub const MAX_EQ_BANDS: usize = 32;

// Vesper BioPhys APO CLSID: {794D0219-58D6-4F71-B53B-987895A5497B}
pub const CLSID_VESPER_APO_STR: &str = "{794D0219-58D6-4F71-B53B-987895A5497B}";

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ApoEqBand {
    pub filter_type: u32,
    pub frequency: f32,
    pub gain_db: f32,
    pub q: f32,
}

impl Default for ApoEqBand {
    fn default() -> Self {
        Self {
            filter_type: 0,
            frequency: 1000.0,
            gain_db: 0.0,
            q: 1.0,
        }
    }
}

#[repr(C)]
pub struct VesperApoSharedState {
    pub is_enabled: std::sync::atomic::AtomicBool,
    pub preamp_gain_db: f32,
    pub band_count: u32,
    pub bands: [ApoEqBand; MAX_EQ_BANDS],
    pub update_sequence: std::sync::atomic::AtomicU64,
}

struct SharedMemoryHandle {
    #[allow(dead_code)]
    handle: HANDLE,
    ptr: *mut VesperApoSharedState,
}
unsafe impl Send for SharedMemoryHandle {}
unsafe impl Sync for SharedMemoryHandle {}

static SHARED_MEMORY: Lazy<Option<SharedMemoryHandle>> = Lazy::new(|| {
    unsafe {
        use std::os::windows::ffi::OsStrExt;
        let wide: Vec<u16> = std::ffi::OsStr::new(SHARED_MEMORY_NAME).encode_wide().chain(std::iter::once(0)).collect();
        let handle = CreateFileMappingW(
            INVALID_HANDLE_VALUE,
            None,
            PAGE_READWRITE,
            0,
            std::mem::size_of::<VesperApoSharedState>() as u32,
            PCWSTR(wide.as_ptr()),
        );

        if let Ok(handle) = handle {
            if !handle.is_invalid() {
                let map = MapViewOfFile(handle, FILE_MAP_ALL_ACCESS, 0, 0, std::mem::size_of::<VesperApoSharedState>());
                if !map.Value.is_null() {
                    let ptr = map.Value as *mut VesperApoSharedState;
                    std::ptr::write(ptr, VesperApoSharedState {
                        is_enabled: std::sync::atomic::AtomicBool::new(true),
                        preamp_gain_db: 0.0,
                        band_count: 0,
                        bands: [ApoEqBand::default(); MAX_EQ_BANDS],
                        update_sequence: std::sync::atomic::AtomicU64::new(1),
                    });
                    return Some(SharedMemoryHandle { handle, ptr });
                }
            }
        }
    }
    None
});

pub fn sync_apo_eq_profile(enabled: bool, preamp_gain_db: f32, bands: &[crate::audio_engine::EqBand]) {
    if let Some(shared) = &*SHARED_MEMORY {
        unsafe {
            let state = &mut *shared.ptr;
            state.is_enabled.store(enabled, Ordering::Relaxed);
            state.preamp_gain_db = preamp_gain_db;
            state.band_count = (bands.len() as u32).min(MAX_EQ_BANDS as u32);

            for (i, band) in bands.iter().take(MAX_EQ_BANDS).enumerate() {
                let filter_type_code = match band.filter_type.as_str() {
                    "low_shelf" => 1,
                    "high_shelf" => 2,
                    "low_pass" => 3,
                    "high_pass" => 4,
                    "band_stop" | "notch" => 5,
                    _ => 0,
                };
                state.bands[i] = ApoEqBand {
                    filter_type: filter_type_code,
                    frequency: band.frequency as f32,
                    gain_db: band.gain_db as f32,
                    q: band.q as f32,
                };
            }

            state.update_sequence.fetch_add(1, Ordering::SeqCst);
        }
    }
}

pub fn is_apo_active() -> bool {
    SHARED_MEMORY.is_some()
}

/// 🔍 [장치 이름으로 윈도우 오디오 Endpoint GUID 자동 검색 및 레지스트리 자동 바인딩]
pub fn auto_bind_device_apo(device_name: &str) -> Result<String, String> {
    // 1. 현재 실행 파일 디렉토리 기준으로 vesper_apo.dll 경로 추출
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {e}"))?
        .parent()
        .ok_or("No parent dir")?
        .to_path_buf();
    let dll_path = exe_dir.join("vesper_apo.dll");
    let dll_path_str = dll_path.to_str().ok_or("Invalid DLL path")?.replace('/', "\\");

    // 2. PowerShell 스크립트로 Windows MMDevices Render 엔드포인트 자동 검색 및 레지스트리 원클릭 등록
    let ps_script = format!(
        r#"
$targetName = "{device_name}"
$dllPath = "{dll_path_str}"
$clsid = "{CLSID_VESPER_APO_STR}"

# COM CLSID 등록
$clsidPath = "HKCU:\Software\Classes\CLSID\$clsid"
if (!(Test-Path $clsidPath)) {{ New-Item -Path $clsidPath -Force | Out-Null }}
Set-ItemProperty -Path $clsidPath -Name "(Default)" -Value "Vesper BioPhys APO"
$inproc = "$clsidPath\InprocServer32"
if (!(Test-Path $inproc)) {{ New-Item -Path $inproc -Force | Out-Null }}
Set-ItemProperty -Path $inproc -Name "(Default)" -Value $dllPath
Set-ItemProperty -Path $inproc -Name "ThreadingModel" -Value "Both"

# MMDevices Endpoint 검색
$endpoints = Get-ChildItem "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render" -ErrorAction SilentlyContinue
$foundGuid = $null

foreach ($ep in $endpoints) {{
    $props = Get-ItemProperty "$($ep.PSPath)\Properties" -ErrorAction SilentlyContinue
    if ($props) {{
        $name1 = $props.'{{a45c254e-df1c-4efd-8020-67d146a850e0}},2'
        $name2 = $props.'{{b3f8fa53-0004-438e-9003-51a46e139bfc}},6'
        if (($name1 -and $name1 -like "*$targetName*") -or ($name2 -and $name2 -like "*$targetName*") -or ($targetName -like "*$name1*")) {{
            $foundGuid = $ep.PSChildName
            break
        }}
    }}
}}

if ($foundGuid) {{
    $fxPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\$foundGuid\FxProperties"
    if (!(Test-Path $fxPath)) {{ New-Item -Path $fxPath -Force | Out-Null }}
    Set-ItemProperty -Path $fxPath -Name "{{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D}},5" -Value $clsid -ErrorAction SilentlyContinue
    Set-ItemProperty -Path $fxPath -Name "{{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D}},6" -Value $clsid -ErrorAction SilentlyContinue
    Set-ItemProperty -Path $fxPath -Name "{{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D}},7" -Value $clsid -ErrorAction SilentlyContinue
    Write-Output "SUCCESS:$foundGuid"
}} else {{
    Write-Output "FALLBACK_GENERIC"
}}
"#
    );

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
        .output()
        .map_err(|e| format!("PowerShell execution failed: {e}"))?;

    let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(result)
}
