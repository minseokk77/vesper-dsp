use std::sync::atomic::Ordering;
use once_cell::sync::Lazy;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::Memory::{
    CreateFileMappingW, MapViewOfFile, FILE_MAP_ALL_ACCESS, PAGE_READWRITE,
};
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

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

/// 🔍 [APO가 해당 장치에 정상 연결되어 있는지 검사]
pub fn check_device_apo_installed(device_name: &str) -> bool {
    let ps_script = format!(
        r#"
$targetName = "{device_name}"
$innerName = if ($targetName -match '\(([^)]+)\)') {{ $matches[1].Trim() }} else {{ $targetName.Trim() }}
$clsid = "{CLSID_VESPER_APO_STR}"
$endpoints = Get-ChildItem "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render" -ErrorAction SilentlyContinue

foreach ($ep in $endpoints) {{
    $props = Get-ItemProperty "$($ep.PSPath)\Properties" -ErrorAction SilentlyContinue
    if ($props) {{
        $n1 = [string]$props.'{{a45c254e-df1c-4efd-8020-67d146a850e0}},2'
        $n2 = [string]$props.'{{b3f8fa53-0004-438e-9003-51a46e139bfc}},6'
        $combined = "$n1 $n2".Trim()
        
        if (($combined -like "*$innerName*") -or ($innerName -like "*$n2*")) {{
            $fx = Get-ItemProperty "$($ep.PSPath)\FxProperties" -ErrorAction SilentlyContinue
            if ($fx -and $fx.'{{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D}},7' -eq $clsid) {{
                Write-Output "INSTALLED"
                exit
            }}
        }}
    }}
}}
Write-Output "NOT_INSTALLED"
"#
    );

    let output = std::process::Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &ps_script])
        .output();

    if let Ok(out) = output {
        let res = String::from_utf8_lossy(&out.stdout).trim().to_string();
        res == "INSTALLED"
    } else {
        false
    }
}

/// 🛡️ [관리자 권한 UAC 팝업을 띄워 Windows MMDevices 소유권 및 ACL 획득 후 APO 완벽 등록]
pub fn install_device_apo_elevated(device_name: &str) -> Result<String, String> {
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {e}"))?
        .parent()
        .ok_or("No parent dir")?
        .to_path_buf();
    let dll_path = exe_dir.join("vesper_apo.dll");
    let dll_path_str = dll_path.to_str().ok_or("Invalid DLL path")?.replace('/', "\\");

    let script_content = format!(
        r#"
$targetName = "{device_name}"
$innerName = if ($targetName -match '\(([^)]+)\)') {{ $matches[1].Trim() }} else {{ $targetName.Trim() }}
$dllPath = "{dll_path_str}"
$clsid = "{CLSID_VESPER_APO_STR}"

# 1. COM CLSID 등록 (HKLM 및 HKCU)
& reg.exe add "HKLM\SOFTWARE\Classes\CLSID\$clsid" /ve /d "Vesper BioPhys APO" /f
& reg.exe add "HKLM\SOFTWARE\Classes\CLSID\$clsid\InprocServer32" /ve /d "$dllPath" /f
& reg.exe add "HKLM\SOFTWARE\Classes\CLSID\$clsid\InprocServer32" /v "ThreadingModel" /d "Both" /f

& reg.exe add "HKCU\Software\Classes\CLSID\$clsid" /ve /d "Vesper BioPhys APO" /f
& reg.exe add "HKCU\Software\Classes\CLSID\$clsid\InprocServer32" /ve /d "$dllPath" /f
& reg.exe add "HKCU\Software\Classes\CLSID\$clsid\InprocServer32" /v "ThreadingModel" /d "Both" /f

# 2. MMDevices Endpoint 검색 및 소유권 획득 후 FxProperties 등록
$endpoints = Get-ChildItem "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render" -ErrorAction SilentlyContinue
foreach ($ep in $endpoints) {{
    $props = Get-ItemProperty "$($ep.PSPath)\Properties" -ErrorAction SilentlyContinue
    if ($props) {{
        $n1 = [string]$props.'{{a45c254e-df1c-4efd-8020-67d146a850e0}},2'
        $n2 = [string]$props.'{{b3f8fa53-0004-438e-9003-51a46e139bfc}},6'
        $combined = "$n1 $n2".Trim()
        
        if (($combined -like "*$innerName*") -or ($innerName -like "*$n2*")) {{
            $guid = $ep.PSChildName
            
            # ACL 권한 부여 (regini)
            $iniPath = "$env:TEMP\vesper_reg_$guid.ini"
            "\Registry\Machine\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\$guid\FxProperties [1 7 17]" | Set-Content -Path $iniPath -Encoding ASCII
            & regini.exe $iniPath
            Remove-Item -Path $iniPath -Force -ErrorAction SilentlyContinue
            
            # FxProperties 등록
            & reg.exe add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\$guid\FxProperties" /v "{{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D}},5" /d "$clsid" /f
            & reg.exe add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\$guid\FxProperties" /v "{{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D}},6" /d "$clsid" /f
            & reg.exe add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\$guid\FxProperties" /v "{{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D}},7" /d "$clsid" /f
        }}
    }}
}}

# 3. 윈도우 오디오 엔진 재시작
& net.exe stop audiosrv /y
& net.exe start audiosrv
"#
    );

    let temp_file = std::env::temp_dir().join("vesper_bind_apo.ps1");
    std::fs::write(&temp_file, script_content).map_err(|e| format!("Failed to write temp script: {e}"))?;

    let ps_cmd = format!(
        "Start-Process powershell -ArgumentList '-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File \"{}\"' -Verb RunAs -WindowStyle Hidden -Wait",
        temp_file.to_str().unwrap().replace('/', "\\")
    );

    let _ = std::process::Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &ps_cmd])
        .output();

    let _ = std::fs::remove_file(temp_file);
    Ok("SUCCESS".to_string())
}
