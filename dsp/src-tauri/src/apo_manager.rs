use std::sync::atomic::Ordering;
use once_cell::sync::Lazy;
use windows::core::{PCWSTR, w};
use windows::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::Memory::{
    CreateFileMappingW, MapViewOfFile, FILE_MAP_ALL_ACCESS, PAGE_READWRITE,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_HIDE;
use std::os::windows::ffi::OsStrExt;
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
    pub is_muted: std::sync::atomic::AtomicBool,
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
                        is_muted: std::sync::atomic::AtomicBool::new(false),
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

pub fn sync_apo_mute(muted: bool) {
    if let Some(shared) = &*SHARED_MEMORY {
        unsafe {
            let state = &mut *shared.ptr;
            state.is_muted.store(muted, Ordering::SeqCst);
        }
    }
}

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

/// 🛡️ [네이티브 ShellExecuteW("runas")를 통해 관리자 UAC 팝업을 띄워 APO 완벽 등록]
pub fn install_device_apo_elevated(device_name: &str) -> Result<String, String> {
    let exe_dir = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {e}"))?
        .parent()
        .ok_or("No parent dir")?
        .to_path_buf();
    let dll_path = exe_dir.join("vesper_apo.dll");
    let dll_path_str = dll_path.to_str().ok_or("Invalid DLL path")?.replace('/', "\\");

    // PowerShell로 엔드포인트 GUID를 먼저 정확히 찾아서 .ps1 생성
    let find_guid_script = format!(
        r#"
$targetName = "{device_name}"
$innerName = if ($targetName -match '\(([^)]+)\)') {{ $matches[1].Trim() }} else {{ $targetName.Trim() }}
$endpoints = Get-ChildItem "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render" -ErrorAction SilentlyContinue

foreach ($ep in $endpoints) {{
    $props = Get-ItemProperty "$($ep.PSPath)\Properties" -ErrorAction SilentlyContinue
    if ($props) {{
        $n1 = [string]$props.'{{a45c254e-df1c-4efd-8020-67d146a850e0}},2'
        $n2 = [string]$props.'{{b3f8fa53-0004-438e-9003-51a46e139bfc}},6'
        $combined = "$n1 $n2".Trim()
        
        if (($combined -like "*$innerName*") -or ($innerName -like "*$n2*")) {{
            Write-Output $ep.PSChildName
            exit
        }}
    }}
}}
"#
    );

    let output = std::process::Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &find_guid_script])
        .output()
        .map_err(|e| format!("Failed to run powershell: {e}"))?;

    let found_guid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if found_guid.is_empty() || !found_guid.starts_with('{') {
        return Err("Matching sound card endpoint not found".to_string());
    }

    let ps_content = format!(
        r#"
Add-Type @"
using System;
using System.Runtime.InteropServices;

public class NativeRegistry {{
    public static readonly IntPtr HKEY_LOCAL_MACHINE = new IntPtr(unchecked((int)0x80000002));
    public const int KEY_SET_VALUE = 0x0002;
    public const int KEY_QUERY_VALUE = 0x0001;
    public const uint REG_SZ = 1;

    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern int RegOpenKeyEx(IntPtr hKey, string lpSubKey, uint ulOptions, int samDesired, out IntPtr phkResult);

    [DllImport("advapi32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern int RegSetValueEx(IntPtr hKey, string lpValueName, int Reserved, uint dwType, string lpData, int cbData);

    [DllImport("advapi32.dll", SetLastError = true)]
    public static extern int RegCloseKey(IntPtr hKey);

    public static int WriteValue(string subKey, string valueName, string valueData) {{
        IntPtr hKey;
        int result = RegOpenKeyEx(HKEY_LOCAL_MACHINE, subKey, 0, KEY_SET_VALUE | KEY_QUERY_VALUE, out hKey);
        if (result != 0) {{
            result = RegOpenKeyEx(HKEY_LOCAL_MACHINE, subKey, 0, KEY_SET_VALUE, out hKey);
        }}
        if (result == 0) {{
            int byteLen = (valueData.Length + 1) * 2;
            result = RegSetValueEx(hKey, valueName, 0, REG_SZ, valueData, byteLen);
            RegCloseKey(hKey);
        }}
        return result;
    }}

    public static int WriteDword(string subKey, string valueName, int valueData) {{
        IntPtr hKey;
        int result = RegOpenKeyEx(HKEY_LOCAL_MACHINE, subKey, 0, KEY_SET_VALUE | KEY_QUERY_VALUE, out hKey);
        if (result != 0) {{
            result = RegOpenKeyEx(HKEY_LOCAL_MACHINE, subKey, 0, KEY_SET_VALUE, out hKey);
        }}
        if (result == 0) {{
            byte[] bytes = BitConverter.GetBytes(valueData);
            result = RegSetValueEx(hKey, valueName, 0, 4, bytes, 4);
            RegCloseKey(hKey);
        }}
        return result;
    }}
}}
"@

$clsid = "{CLSID_VESPER_APO_STR}"
$dll = "{dll_path_str}"
$guid = "{found_guid}"

# 1. HKLM CLSID 등록
if (!(Test-Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid")) {{ New-Item -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid" -Value "Vesper BioPhys APO" -Force | Out-Null }}
if (!(Test-Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid\InprocServer32")) {{ New-Item -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid\InprocServer32" -Value $dll -Force | Out-Null }}
Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid\InprocServer32" -Name "ThreadingModel" -Value "Both" -Force -ErrorAction SilentlyContinue

# 2. HKCU CLSID 등록
if (!(Test-Path "HKCU:\Software\Classes\CLSID\$clsid")) {{ New-Item -Path "HKCU:\Software\Classes\CLSID\$clsid" -Value "Vesper BioPhys APO" -Force | Out-Null }}
if (!(Test-Path "HKCU:\Software\Classes\CLSID\$clsid\InprocServer32")) {{ New-Item -Path "HKCU:\Software\Classes\CLSID\$clsid\InprocServer32" -Value $dll -Force | Out-Null }}
Set-ItemProperty -Path "HKCU:\Software\Classes\CLSID\$clsid\InprocServer32" -Name "ThreadingModel" -Value "Both" -Force -ErrorAction SilentlyContinue

# 3. FxProperties Win32 직접 쓰기
$subKey = "SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\$guid\FxProperties"
[NativeRegistry]::WriteValue($subKey, "{{d04e05a6-594b-4fb6-a80d-01af5eed7d1d}},5", $clsid)
[NativeRegistry]::WriteValue($subKey, "{{d04e05a6-594b-4fb6-a80d-01af5eed7d1d}},6", $clsid)
[NativeRegistry]::WriteValue($subKey, "{{d04e05a6-594b-4fb6-a80d-01af5eed7d1d}},7", $clsid)

# 4. 🎛️ [FiiO Thesycon UAC 2.0 384kHz 모드 Windows 마스터 볼륨/음소거 연동 활성화]
# Thesycon 드라이버가 하드웨어 볼륨 노드를 우회하고 Windows 32-bit 부동소수점 디지털 감쇄기를 직접 활성화하도록 설정
[NativeRegistry]::WriteDword("SYSTEM\CurrentControlSet\Services\fiio_usbaudio\ParametersDriver\Settings", "DisableKsVolMuteOut", 1)
[NativeRegistry]::WriteDword("SYSTEM\CurrentControlSet\Services\fiio_usbaudio\ParametersDriver\Settings", "IgnoreFeatureUnitOut", 1)
[NativeRegistry]::WriteDword("SYSTEM\CurrentControlSet\Services\fiio_usbaudioks\Parameters", "DisableKsVolMuteOut", 1)
[NativeRegistry]::WriteDword("SYSTEM\CurrentControlSet\Services\fiio_usbaudioks\Parameters", "IgnoreFeatureUnitOut", 1)
[NativeRegistry]::WriteDword("SYSTEM\CurrentControlSet\Services\fiio_usbaudio\Parameters", "DisableKsVolMuteOut", 1)
[NativeRegistry]::WriteDword("SYSTEM\CurrentControlSet\Services\fiio_usbaudioks\ParametersDriver\Settings", "DisableKsVolMuteOut", 1)

# 5. 오디오 서비스 안전 재시작
Restart-Service audiosrv -Force
"#
    );

    let temp_ps1 = std::env::temp_dir().join("vesper_elevated_install.ps1");
    std::fs::write(&temp_ps1, ps_content).map_err(|e| format!("Failed to write temp ps1: {e}"))?;

    let ps1_str = temp_ps1.to_str().unwrap().replace('/', "\\");
    let args_str = format!("-NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File \"{}\"", ps1_str);
    let args_wide: Vec<u16> = args_str.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        ShellExecuteW(
            None,
            w!("runas"),
            w!("powershell.exe"),
            PCWSTR(args_wide.as_ptr()),
            PCWSTR::null(),
            SW_HIDE,
        );
    }

    Ok("SUCCESS".to_string())
}
