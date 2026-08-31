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

/// 🔍 [APO가 해당 장치에 정상 연결되어 있는지 검사]
pub fn check_device_apo_installed(device_name: &str) -> bool {
    let ps_script = format!(
        r#"
$targetName = "{device_name}"
$clsid = "{CLSID_VESPER_APO_STR}"
$endpoints = Get-ChildItem "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render" -ErrorAction SilentlyContinue

foreach ($ep in $endpoints) {{
    $props = Get-ItemProperty "$($ep.PSPath)\Properties" -ErrorAction SilentlyContinue
    if ($props) {{
        $name1 = $props.'{{a45c254e-df1c-4efd-8020-67d146a850e0}},2'
        $name2 = $props.'{{b3f8fa53-0004-438e-9003-51a46e139bfc}},6'
        if (($name1 -and $name1 -like "*$targetName*") -or ($name2 -and $name2 -like "*$targetName*") -or ($targetName -like "*$name1*")) {{
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
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
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
# 1. SeTakeOwnership & SeRestore P/Invoke 헬퍼 컴파일
`$definition = @"
using System;
using System.Runtime.InteropServices;
using System.Security.Principal;

public class RegSecurityHelper {{
    [DllImport("advapi32.dll", SetLastError = true)]
    public static extern bool OpenProcessToken(IntPtr ProcessHandle, uint DesiredAccess, out IntPtr TokenHandle);

    [DllImport("advapi32.dll", SetLastError = true, CharSet = CharSet.Auto)]
    public static extern bool LookupPrivilegeValue(string lpSystemName, string lpName, out long lpLuid);

    [DllImport("advapi32.dll", SetLastError = true)]
    public static extern bool AdjustTokenPrivileges(IntPtr TokenHandle, bool DisableAllPrivileges, ref TOKEN_PRIVILEGES NewState, uint BufferLength, IntPtr PreviousState, IntPtr ReturnLength);

    [StructLayout(LayoutKind.Sequential, Pack = 1)]
    public struct TOKEN_PRIVILEGES {{
        public uint PrivilegeCount;
        public long Luid;
        public uint Attributes;
    }}

    public const uint TOKEN_ADJUST_PRIVILEGES = 0x0020;
    public const uint TOKEN_QUERY = 0x0008;
    public const uint SE_PRIVILEGE_ENABLED = 0x00000002;
    public const string SE_TAKE_OWNERSHIP_NAME = "SeTakeOwnershipPrivilege";
    public const string SE_RESTORE_NAME = "SeRestorePrivilege";

    public static bool EnablePrivileges() {{
        IntPtr hToken;
        if (!OpenProcessToken(System.Diagnostics.Process.GetCurrentProcess().Handle, TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, out hToken)) {{
            return false;
        }}

        long luid;
        if (LookupPrivilegeValue(null, SE_TAKE_OWNERSHIP_NAME, out luid)) {{
            TOKEN_PRIVILEGES tp = new TOKEN_PRIVILEGES {{ PrivilegeCount = 1, Luid = luid, Attributes = SE_PRIVILEGE_ENABLED }};
            AdjustTokenPrivileges(hToken, false, ref tp, 0, IntPtr.Zero, IntPtr.Zero);
        }}

        if (LookupPrivilegeValue(null, SE_RESTORE_NAME, out luid)) {{
            TOKEN_PRIVILEGES tp = new TOKEN_PRIVILEGES {{ PrivilegeCount = 1, Luid = luid, Attributes = SE_PRIVILEGE_ENABLED }};
            AdjustTokenPrivileges(hToken, false, ref tp, 0, IntPtr.Zero, IntPtr.Zero);
        }}

        return true;
    }}
}}
"@

Add-Type -TypeDefinition `$definition
[RegSecurityHelper]::EnablePrivileges() | Out-Null

function Grant-RegAccess([string]`$subPath) {{
    try {{
        `$regKey = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey(`$subPath, [Microsoft.Win32.RegistryKeyPermissionCheck]::ReadWriteSubTree, [System.Security.AccessControl.RegistryRights]::TakeOwnership)
        if (`$regKey) {{
            `$acl = `$regKey.GetAccessControl()
            `$admin = New-Object System.Security.Principal.NTAccount("Administrators")
            `$acl.SetOwner(`$admin)
            `$regKey.SetAccessControl(`$acl)
            `$regKey.Close()
        }}

        `$regKey = [Microsoft.Win32.Registry]::LocalMachine.OpenSubKey(`$subPath, [Microsoft.Win32.RegistryKeyPermissionCheck]::ReadWriteSubTree, [System.Security.AccessControl.RegistryRights]::ChangePermissions)
        if (`$regKey) {{
            `$acl = `$regKey.GetAccessControl()
            `$rule = New-Object System.Security.AccessControl.RegistryAccessRule("Administrators", "FullControl", "ContainerInherit,ObjectInherit", "None", "Allow")
            `$acl.SetAccessRule(`$rule)
            `$regKey.SetAccessControl(`$acl)
            `$regKey.Close()
        }}
    }} catch {{}}
}}

`$targetName = "{device_name}"
`$dllPath = "{dll_path_str}"
`$clsid = "{CLSID_VESPER_APO_STR}"

# 2. COM CLSID 등록 (HKLM 및 HKCU)
`$clsidPaths = @(
    "HKLM:\SOFTWARE\Classes\CLSID\`$clsid",
    "HKCU:\Software\Classes\CLSID\`$clsid"
)
foreach (`$cp in `$clsidPaths) {{
    if (!(Test-Path `$cp)) {{ New-Item -Path `$cp -Force -ErrorAction SilentlyContinue | Out-Null }}
    Set-ItemProperty -Path `$cp -Name "(Default)" -Value "Vesper BioPhys APO" -ErrorAction SilentlyContinue
    `$inproc = "`$cp\InprocServer32"
    if (!(Test-Path `$inproc)) {{ New-Item -Path `$inproc -Force -ErrorAction SilentlyContinue | Out-Null }}
    Set-ItemProperty -Path `$inproc -Name "(Default)" -Value `$dllPath -ErrorAction SilentlyContinue
    Set-ItemProperty -Path `$inproc -Name "ThreadingModel" -Value "Both" -ErrorAction SilentlyContinue
}}

# 3. MMDevices Endpoint 검색 및 소유권 획득 후 FxProperties 등록
`$endpoints = Get-ChildItem "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render" -ErrorAction SilentlyContinue
foreach (`$ep in `$endpoints) {{
    `$props = Get-ItemProperty "`$(`$ep.PSPath)\Properties" -ErrorAction SilentlyContinue
    if (`$props) {{
        `$name1 = `$props.'{{a45c254e-df1c-4efd-8020-67d146a850e0}},2'
        `$name2 = `$props.'{{b3f8fa53-0004-438e-9003-51a46e139bfc}},6'
        if ((`$name1 -and `$name1 -like "*`$targetName*") -or (`$name2 -and `$name2 -like "*`$targetName*") -or (`$targetName -like "*`$name1*")) {{
            `$guid = `$ep.PSChildName
            `$relPath = "SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\`$guid\FxProperties"
            Grant-RegAccess `$relPath
            
            & reg.exe add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\`$guid\FxProperties" /v "{{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D}},5" /d `$clsid /f
            & reg.exe add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\`$guid\FxProperties" /v "{{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D}},6" /d `$clsid /f
            & reg.exe add "HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render\`$guid\FxProperties" /v "{{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D}},7" /d `$clsid /f
        }}
    }}
}}

# 4. 윈도우 오디오 엔진 재시작
net stop audiosrv /y
net start audiosrv
"#
    );

    let temp_file = std::env::temp_dir().join("vesper_bind_apo.ps1");
    std::fs::write(&temp_file, script_content).map_err(|e| format!("Failed to write temp script: {e}"))?;

    let ps_cmd = format!(
        "Start-Process powershell -ArgumentList '-NoProfile -ExecutionPolicy Bypass -File \"{}\"' -Verb RunAs -Wait",
        temp_file.to_str().unwrap().replace('/', "\\")
    );

    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &ps_cmd])
        .output();

    let _ = std::fs::remove_file(temp_file);
    Ok("SUCCESS".to_string())
}
