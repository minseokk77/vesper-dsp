pub mod shared_state;
pub mod dsp_kernel;

use std::ffi::c_void;
use std::sync::Mutex;
use once_cell::sync::Lazy;
use windows::core::{GUID, HRESULT, BOOL};
use windows::Win32::Foundation::{HINSTANCE, S_OK, E_NOINTERFACE, E_POINTER};
use windows::Win32::System::Memory::{OpenFileMappingW, MapViewOfFile, FILE_MAP_READ};

use crate::shared_state::{VesperApoSharedState, SHARED_MEMORY_NAME};
use crate::dsp_kernel::VesperApoDspPipeline;

// Vesper BioPhys APO CLSID: {794D0219-58D6-4F71-B53B-987895A5497B}
pub const CLSID_VESPER_APO: GUID = GUID::from_u128(0x794D0219_58D6_4F71_B53B_987895A5497B);

struct SafeSharedPtr(usize);
unsafe impl Send for SafeSharedPtr {}
unsafe impl Sync for SafeSharedPtr {}

static GLOBAL_DSP: Lazy<Mutex<VesperApoDspPipeline>> = Lazy::new(|| Mutex::new(VesperApoDspPipeline::new(48000.0)));
static SHARED_MEMORY_PTR: Lazy<Option<SafeSharedPtr>> = Lazy::new(|| {
    unsafe {
        use std::os::windows::ffi::OsStrExt;
        let wide: Vec<u16> = std::ffi::OsStr::new(SHARED_MEMORY_NAME).encode_wide().chain(std::iter::once(0)).collect();
        let handle = OpenFileMappingW(FILE_MAP_READ.0, false, windows::core::PCWSTR(wide.as_ptr()));
        if let Ok(handle) = handle {
            if !handle.is_invalid() {
                let map = MapViewOfFile(handle, FILE_MAP_READ, 0, 0, std::mem::size_of::<VesperApoSharedState>());
                if !map.Value.is_null() {
                    return Some(SafeSharedPtr(map.Value as usize));
                }
            }
        }
    }
    None
});

#[no_mangle]
pub unsafe extern "system" fn DllMain(
    _hinst_dll: HINSTANCE,
    _fdw_reason: u32,
    _lpv_reserved: *mut c_void,
) -> BOOL {
    BOOL(1)
}

/// 🚀 [실시간 오디오 버퍼 인플레이스 DSP 처리 함수]
/// audiodg.exe 또는 호스트 프로세스에서 오디오 프레임이 전달될 때 호출됩니다.
#[no_mangle]
pub unsafe extern "C" fn VesperApoProcessBuffer(
    buffer: *mut f32,
    sample_count: usize,
    _sample_rate: u32,
) {
    if buffer.is_null() || sample_count == 0 {
        return;
    }

    if let Some(shared_ptr) = &*SHARED_MEMORY_PTR {
        let state = &*(shared_ptr.0 as *const VesperApoSharedState);
        
        // 🔇 [하드웨어 DAC 무시 음소거 원천 해결: APO 레벨에서 버퍼 0 완전 초기화]
        if state.is_muted.load(std::sync::atomic::Ordering::Relaxed) {
            std::ptr::write_bytes(buffer, 0, sample_count);
            return;
        }

        if !state.is_enabled.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        if let Ok(mut dsp) = GLOBAL_DSP.try_lock() {
            dsp.update_from_shared_state(state);
            let slice = std::slice::from_raw_parts_mut(buffer, sample_count);
            dsp.process_interleaved_stereo(slice);
        }
    }
}

/// COM Class Factory Entry
#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if ppv.is_null() || rclsid.is_null() || riid.is_null() {
        return E_POINTER;
    }
    *ppv = std::ptr::null_mut();

    if *rclsid == CLSID_VESPER_APO {
        return S_OK;
    }

    E_NOINTERFACE
}

/// 📦 [APO COM DLL 자체 레지스트리 등록]
#[no_mangle]
pub unsafe extern "system" fn DllRegisterServer() -> HRESULT {
    S_OK
}

/// 📦 [APO COM DLL 자체 레지스트리 해제]
#[no_mangle]
pub unsafe extern "system" fn DllUnregisterServer() -> HRESULT {
    S_OK
}
