use biquad::{Biquad, Coefficients, DirectForm1, ToHertz};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Sample;
use once_cell::sync::Lazy;
use ringbuf::{
    traits::{Consumer, Observer, Producer, Split},
    HeapRb,
};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Emitter;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

static IS_RUNNING: Lazy<Arc<AtomicBool>> = Lazy::new(|| Arc::new(AtomicBool::new(false)));
static LAST_CLIP_TIME: Lazy<AtomicI64> = Lazy::new(|| AtomicI64::new(0));
static STREAMS: Lazy<Mutex<Vec<cpal::Stream>>> = Lazy::new(|| Mutex::new(Vec::new()));
static WORKER_THREAD: Lazy<Mutex<Option<JoinHandle<()>>>> = Lazy::new(|| Mutex::new(None));
static ENGINE_TRANSITION: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
static STREAM_INFO: Lazy<Mutex<Option<StreamInfo>>> = Lazy::new(|| Mutex::new(None));
static INPUT_OVERRUNS: AtomicU64 = AtomicU64::new(0);
static OUTPUT_OVERRUNS: AtomicU64 = AtomicU64::new(0);
static OUTPUT_UNDERRUNS: AtomicU64 = AtomicU64::new(0);

pub static OUTPUT_MUTED: AtomicBool = AtomicBool::new(false);
pub static OUTPUT_EQ_PROFILE: Lazy<Arc<Mutex<EqProfile>>> =
    Lazy::new(|| Arc::new(Mutex::new(EqProfile::default())));
pub static EQ_PROFILE_CHANGED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EqBand {
    pub filter_type: String,
    pub frequency: f64,
    pub gain_db: f64,
    pub q: f64,
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct EqProfile {
    pub enabled: bool,
    pub preamp_gain: f64,
    pub bands: Vec<EqBand>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct DspFactoryPreset {
    pub name: String,
    pub description: String,
    pub target_sample_rate: Option<u32>,
    pub filter_type: String,
    pub headroom_db: f32,
    pub preamp_gain: f64,
}

pub fn get_native_factory_presets() -> Vec<DspFactoryPreset> {
    vec![
        DspFactoryPreset {
            name: "BioPhys Bit-Perfect Master".to_string(),
            description: "384kHz / 768kHz Ultra-Hi-Res Bit-Perfect Resampling with Phase Flash Filter".to_string(),
            target_sample_rate: Some(384000),
            filter_type: "biophys_phase_flash".to_string(),
            headroom_db: -3.0,
            preamp_gain: 0.0,
        },
        DspFactoryPreset {
            name: "Acoustic Fluid Natural".to_string(),
            description: "BioPhys 3-Mass Acoustic Smooth Minimum Phase Filtering".to_string(),
            target_sample_rate: Some(192000),
            filter_type: "biophys_acoustic_smooth".to_string(),
            headroom_db: -2.5,
            preamp_gain: 0.0,
        },
        DspFactoryPreset {
            name: "Audiophile Reference Clean".to_string(),
            description: "Zero-Latency 256-Tap Blackman-Harris Resampler".to_string(),
            target_sample_rate: None,
            filter_type: "linear_precise".to_string(),
            headroom_db: -3.0,
            preamp_gain: 0.0,
        },
    ]
}

#[derive(serde::Serialize)]
pub struct BitDepthOption {
    pub value: String,
    pub label: String,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct StreamInfo {
    pub source_sample_rate: u32,
    pub source_bit_depth: String,
    pub source_channels: usize,
    pub output_sample_rate: u32,
    pub output_bit_depth: String,
    pub output_channels: usize,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct EngineStatus {
    pub running: bool,
    pub input_overruns: u64,
    pub output_overruns: u64,
    pub output_underruns: u64,
}

#[derive(Clone, Debug, serde::Serialize)]
struct EngineError {
    stage: String,
    message: String,
}

pub fn get_current_stream_info() -> Option<StreamInfo> {
    STREAM_INFO.lock().ok().and_then(|info| info.clone())
}

pub fn get_engine_status() -> EngineStatus {
    EngineStatus {
        running: IS_RUNNING.load(Ordering::SeqCst),
        input_overruns: INPUT_OVERRUNS.load(Ordering::Relaxed),
        output_overruns: OUTPUT_OVERRUNS.load(Ordering::Relaxed),
        output_underruns: OUTPUT_UNDERRUNS.load(Ordering::Relaxed),
    }
}

fn report_stream_error(app: &tauri::AppHandle, stage: &str, error: impl std::fmt::Display) {
    IS_RUNNING.store(false, Ordering::SeqCst);
    let message = error.to_string();
    eprintln!("{stage} stream error: {message}");
    let _ = app.emit(
        "engine-error",
        EngineError {
            stage: stage.to_string(),
            message,
        },
    );
}

fn get_host(_is_asio: bool) -> cpal::Host {
    cpal::default_host()
}

fn is_virtual_cable_device(name: &str) -> bool {
    name.to_lowercase().contains("vb-audio")
}

pub fn get_source_devices(is_asio: bool) -> Vec<String> {
    let host = get_host(is_asio);
    let mut device_names = Vec::new();

    if let Ok(devices) = host.output_devices() {
        for device in devices {
            let name = device.to_string();
            if !device_names.contains(&name) {
                device_names.push(name);
            }
        }
    }

    device_names
}

pub fn get_output_devices(is_asio: bool) -> Vec<String> {
    let host = get_host(is_asio);
    host.output_devices()
        .map(|devices| devices.map(|device| device.to_string()).collect())
        .unwrap_or_default()
}

fn find_device(host: &cpal::Host, name: &str) -> Option<(cpal::Device, bool)> {
    if let Ok(devices) = host.input_devices() {
        for d in devices {
            let dev_name = d.to_string();
            if dev_name.contains(name) {
                return Some((d, true));
            }
        }
    }
    if let Ok(devices) = host.output_devices() {
        for d in devices {
            let dev_name = d.to_string();
            if dev_name.contains(name) {
                return Some((d, false));
            }
        }
    }
    None
}

fn find_input_device(host: &cpal::Host, name: &str) -> Option<cpal::Device> {
    host.input_devices()
        .ok()?
        .find(|device| device.to_string() == name)
}

fn find_source_device(host: &cpal::Host, name: &str) -> Option<(cpal::Device, bool)> {
    if let Some(device) = find_input_device(host, name) {
        return Some((device, true));
    }

    host.output_devices().ok()?.find_map(|device| {
        let device_name = device.to_string();
        (device_name == name && is_virtual_cable_device(&device_name)).then_some((device, false))
    })
}

fn find_output_device(host: &cpal::Host, name: &str) -> Option<cpal::Device> {
    host.output_devices()
        .ok()?
        .find(|device| device.to_string() == name)
}

fn find_best_input_config(device: &cpal::Device) -> Result<cpal::SupportedStreamConfig, String> {
    if let Ok(config) = device.default_input_config() {
        return Ok(config);
    }
    let mut configs = device
        .supported_input_configs()
        .map_err(|_| "No input config".to_string())?
        .collect::<Vec<_>>();
    configs.sort_by_key(|config| std::cmp::Reverse(config.max_sample_rate()));
    configs
        .into_iter()
        .next()
        .map(cpal::SupportedStreamConfigRange::with_max_sample_rate)
        .ok_or_else(|| "No input config".to_string())
}

fn find_input_config_with_target(
    device: &cpal::Device,
    target_sample_rate: Option<u32>,
) -> Result<cpal::SupportedStreamConfig, String> {
    let Some(target_sample_rate) = target_sample_rate else {
        return find_best_input_config(device);
    };

    let mut configs = device
        .supported_input_configs()
        .map_err(|_| "No input config".to_string())?
        .filter(|config| sample_format_value(config.sample_format()).is_some())
        .collect::<Vec<_>>();
    configs.sort_by_key(|config| {
        (
            if config.sample_format() == cpal::SampleFormat::I32 {
                0
            } else {
                1
            },
            std::cmp::Reverse(config.max_sample_rate()),
        )
    });

    configs
        .into_iter()
        .find(|config| {
            target_sample_rate >= config.min_sample_rate()
                && target_sample_rate <= config.max_sample_rate()
        })
        .map(|config| config.with_sample_rate(target_sample_rate))
        .ok_or_else(|| format!("Input device does not support {target_sample_rate} Hz"))
}

fn find_best_output_config(device: &cpal::Device) -> Result<cpal::SupportedStreamConfig, String> {
    if let Ok(config) = device.default_output_config() {
        if sample_format_value(config.sample_format()).is_some() {
            return Ok(config);
        }
    }
    let mut configs = device
        .supported_output_configs()
        .map_err(|_| "No output config".to_string())?
        .filter(|config| sample_format_value(config.sample_format()).is_some())
        .collect::<Vec<_>>();
    configs.sort_by_key(|config| std::cmp::Reverse(config.max_sample_rate()));
    configs
        .into_iter()
        .next()
        .map(cpal::SupportedStreamConfigRange::with_max_sample_rate)
        .ok_or_else(|| "No output config".to_string())
}

fn sample_format_value(format: cpal::SampleFormat) -> Option<&'static str> {
    match format {
        cpal::SampleFormat::I16 => Some("i16"),
        cpal::SampleFormat::I24 => Some("i24"),
        cpal::SampleFormat::I32 => Some("i32"),
        cpal::SampleFormat::U16 => Some("u16"),
        cpal::SampleFormat::F32 => Some("f32"),
        _ => None,
    }
}

fn sample_format_label(format: cpal::SampleFormat) -> Option<&'static str> {
    match format {
        cpal::SampleFormat::I16 | cpal::SampleFormat::U16 => Some("16-bit"),
        cpal::SampleFormat::I24 => Some("24-bit"),
        cpal::SampleFormat::I32 => Some("32-bit"),
        cpal::SampleFormat::F32 => Some("32-bit float"),
        _ => None,
    }
}

fn find_output_config_with_target(
    device: &cpal::Device,
    target_sample_rate: Option<u32>,
    output_sample_format: Option<&str>,
) -> Result<cpal::SupportedStreamConfig, String> {
    if target_sample_rate.is_some() || output_sample_format.is_some() {
        let mut configs = device
            .supported_output_configs()
            .map_err(|_| "No output config".to_string())?
            .filter(|config| {
                let Some(format) = sample_format_value(config.sample_format()) else {
                    return false;
                };
                output_sample_format
                    .map(|wanted| format == wanted)
                    .unwrap_or(true)
            })
            .collect::<Vec<_>>();
        configs.sort_by_key(|config| std::cmp::Reverse(config.max_sample_rate()));
        for config in configs {
            if let Some(target) = target_sample_rate {
                if target >= config.min_sample_rate() && target <= config.max_sample_rate() {
                    return Ok(config.with_sample_rate(target));
                }
            } else {
                return Ok(config.with_max_sample_rate());
            }
        }
        if output_sample_format.is_some() {
            return Err("Output device does not support the selected bit depth".to_string());
        }
    }
    find_best_output_config(device)
}

pub fn get_device_sample_rate(device_name: &str, is_asio: bool) -> Result<u32, String> {
    let host = get_host(is_asio);
    if let Some((device, true)) = find_device(&host, device_name) {
        return Ok(find_best_input_config(&device)?.sample_rate());
    }
    let (device, _) = find_device(&host, device_name).ok_or("Device not found")?;
    Ok(find_best_output_config(&device)?.sample_rate())
}

pub fn get_device_bit_depth(device_name: &str, is_asio: bool) -> Result<String, String> {
    let host = get_host(is_asio);
    let (device, is_input) = if let Some((dev, true)) = find_device(&host, device_name) {
        (dev, true)
    } else {
        let (dev, _) = find_device(&host, device_name).ok_or("Device not found")?;
        (dev, false)
    };

    let config = if is_input {
        find_best_input_config(&device)?
    } else {
        find_best_output_config(&device)?
    };

    let mut current_format = match config.sample_format() {
        cpal::SampleFormat::F32 => "32-bit float".to_string(),
        cpal::SampleFormat::F64 => "64-bit float".to_string(),
        cpal::SampleFormat::I16 | cpal::SampleFormat::U16 => "16-bit".to_string(),
        cpal::SampleFormat::I32 | cpal::SampleFormat::U32 => "32-bit".to_string(),
        cpal::SampleFormat::I8 | cpal::SampleFormat::U8 => "8-bit".to_string(),
        _ => "Unknown".to_string(),
    };

    // WASAPI 공유 모드가 항상 F32(32-bit float)로 가짜 포맷을 보고하는 것을 우회하는 로직
    if !is_asio && current_format == "32-bit float" {
        let mut max_int_bit = 0;

        let supported_configs: Vec<_> = if is_input {
            device
                .supported_input_configs()
                .map(|iter| iter.collect())
                .unwrap_or_default()
        } else {
            device
                .supported_output_configs()
                .map(|iter| iter.collect())
                .unwrap_or_default()
        };

        for sc in supported_configs {
            match sc.sample_format() {
                cpal::SampleFormat::I16 => {
                    if max_int_bit < 16 {
                        max_int_bit = 16;
                    }
                }
                cpal::SampleFormat::I32 => {
                    let bit = if device_name.to_lowercase().contains("fiio") {
                        32
                    } else {
                        24
                    };
                    if max_int_bit < bit {
                        max_int_bit = bit;
                    }
                }
                _ => {}
            }
        }

        if max_int_bit > 0 {
            current_format = format!("{}-bit", max_int_bit);
        }
    }

    Ok(current_format)
}

pub fn get_device_supported_bit_depths(
    device_name: &str,
    is_asio: bool,
) -> Result<Vec<BitDepthOption>, String> {
    let host = get_host(is_asio);
    let (device, _) = find_device(&host, device_name).ok_or("Output device not found")?;
    let mut options = Vec::new();
    for config in device
        .supported_output_configs()
        .map_err(|_| "No output config".to_string())?
    {
        let Some(value) = sample_format_value(config.sample_format()) else {
            continue;
        };
        let Some(label) = sample_format_label(config.sample_format()) else {
            continue;
        };
        if !options
            .iter()
            .any(|option: &BitDepthOption| option.value == value)
        {
            options.push(BitDepthOption {
                value: value.to_string(),
                label: label.to_string(),
            });
        }
    }
    Ok(options)
}

pub fn get_device_supported_sample_rates(
    device_name: &str,
    is_asio: bool,
) -> Result<Vec<u32>, String> {
    let host = get_host(is_asio);
    let (device, _) = find_device(&host, device_name).ok_or("Output device not found")?;
    let configs = device
        .supported_output_configs()
        .map_err(|_| "No output config".to_string())?;
    let standard_rates = [
        44_100, 48_000, 88_200, 96_000, 176_400, 192_000, 352_800, 384_000, 705_600, 768_000,
    ];
    let mut rates = Vec::new();
    for config in configs {
        for rate in standard_rates {
            if rate >= config.min_sample_rate()
                && rate <= config.max_sample_rate()
                && !rates.contains(&rate)
            {
                rates.push(rate);
            }
        }
    }
    rates.sort_unstable();
    Ok(rates)
}

pub fn set_output_mute(muted: bool) {
    OUTPUT_MUTED.store(muted, Ordering::SeqCst);
    set_system_mute(muted);
}

#[cfg(windows)]
pub fn set_system_mute(muted: bool) {
    unsafe {
        use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
        use windows::Win32::Media::Audio::{
            eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
            IAudioSessionManager2, ISimpleAudioVolume,
        };
        use windows::Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
        };
        use windows::core::Interface;

        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        if let Ok(enumerator) = CoCreateInstance::<_, IMMDeviceEnumerator>(
            &MMDeviceEnumerator,
            None,
            CLSCTX_ALL,
        ) {
            if let Ok(device) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
                // 1. 엔드포인트 레벨 음소거
                if let Ok(endpoint_volume) = device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                    let _ = endpoint_volume.SetMute(muted, std::ptr::null());
                }

                // 2. 🔇 [FiiO USB DAC 하드웨어 무시 음소거 원천 해결: 모든 활성 앱 오디오 세션 전수 소프트웨어 음소거]
                if let Ok(session_mgr) = device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) {
                    if let Ok(session_enum) = session_mgr.GetSessionEnumerator() {
                        if let Ok(count) = session_enum.GetCount() {
                            for i in 0..count {
                                if let Ok(session_ctrl) = session_enum.GetSession(i) {
                                    if let Ok(simple_vol) = session_ctrl.cast::<ISimpleAudioVolume>() {
                                        let _ = simple_vol.SetMute(muted, std::ptr::null());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(not(windows))]
pub fn set_system_mute(_muted: bool) {}

#[cfg(windows)]
pub fn get_system_mute() -> bool {
    unsafe {
        use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
        use windows::Win32::Media::Audio::{
            eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
        };
        use windows::Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
        };

        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        if let Ok(enumerator) = CoCreateInstance::<_, IMMDeviceEnumerator>(
            &MMDeviceEnumerator,
            None,
            CLSCTX_ALL,
        ) {
            if let Ok(device) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
                if let Ok(endpoint_volume) = device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                    if let Ok(is_muted) = endpoint_volume.GetMute() {
                        return is_muted.as_bool();
                    }
                }
            }
        }
    }
    OUTPUT_MUTED.load(Ordering::Relaxed)
}

#[cfg(windows)]
pub fn start_windows_volume_sync_daemon() {
    std::thread::Builder::new()
        .name("vesper_win_vol_sync".to_string())
        .spawn(|| {
            unsafe {
                use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
                use windows::Win32::Media::Audio::{
                    eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
                    IAudioSessionManager2, ISimpleAudioVolume,
                };
                use windows::Win32::System::Com::{
                    CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED,
                };
                use windows::core::Interface;

                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                let mut last_vol: f32 = -1.0;
                let mut last_mute: bool = false;

                loop {
                    std::thread::sleep(std::time::Duration::from_millis(20)); // 50 FPS 초저지연 실시간 동기화

                    if let Ok(enumerator) = CoCreateInstance::<_, IMMDeviceEnumerator>(
                        &MMDeviceEnumerator,
                        None,
                        CLSCTX_ALL,
                    ) {
                        if let Ok(device) = enumerator.GetDefaultAudioEndpoint(eRender, eConsole) {
                            if let Ok(endpoint_volume) = device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                                let mut current_vol = 1.0f32;
                                let mut current_mute = false;

                                if let Ok(vol) = endpoint_volume.GetMasterVolumeLevelScalar() {
                                    current_vol = vol;
                                }
                                if let Ok(mute) = endpoint_volume.GetMute() {
                                    current_mute = mute.as_bool();
                                }

                                if (current_vol - last_vol).abs() > 0.005 || current_mute != last_mute {
                                    last_vol = current_vol;
                                    last_mute = current_mute;

                                    if let Ok(session_mgr) = device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) {
                                        if let Ok(session_enum) = session_mgr.GetSessionEnumerator() {
                                            if let Ok(count) = session_enum.GetCount() {
                                                for i in 0..count {
                                                    if let Ok(session_ctrl) = session_enum.GetSession(i) {
                                                        if let Ok(simple_vol) = session_ctrl.cast::<ISimpleAudioVolume>() {
                                                            let _ = simple_vol.SetMasterVolume(current_vol, std::ptr::null());
                                                            let _ = simple_vol.SetMute(current_mute, std::ptr::null());
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        })
        .ok();
}

#[cfg(not(windows))]
pub fn start_windows_volume_sync_daemon() {}

#[cfg(windows)]
pub fn set_windows_default_playback_device(device_name: &str) -> Result<(), String> {
    let script = format!(
        r#"
Add-Type @"
using System;
using System.Runtime.InteropServices;
[ComImport]
[Guid("870af99c-171d-4f9e-af0d-e63df40c2bc9")]
public class PolicyConfigClient {{}}
[Guid("f8679f50-850a-41cf-9c72-430f290290c8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
public interface IPolicyConfig {{
    [PreserveSig] int GetMixFormat();
    [PreserveSig] int GetDeviceFormat();
    [PreserveSig] int ResetDeviceFormat();
    [PreserveSig] int SetDeviceFormat();
    [PreserveSig] int GetProcessingPeriod();
    [PreserveSig] int SetProcessingPeriod();
    [PreserveSig] int GetShareMode();
    [PreserveSig] int SetShareMode();
    [PreserveSig] int GetPropertyValue();
    [PreserveSig] int SetPropertyValue();
    [PreserveSig] int SetDefaultEndpoint([MarshalAs(UnmanagedType.LPWStr)] string wszDeviceId, int role);
    [PreserveSig] int SetEndpointVisibility();
}}
public class AudioSwitcher {{
    public static int SetDefault(string deviceId) {{
        try {{
            IPolicyConfig policyConfig = (IPolicyConfig)new PolicyConfigClient();
            int hr1 = policyConfig.SetDefaultEndpoint(deviceId, 0);
            int hr2 = policyConfig.SetDefaultEndpoint(deviceId, 1);
            int hr3 = policyConfig.SetDefaultEndpoint(deviceId, 2);
            return (hr1 == 0 && hr2 == 0 && hr3 == 0) ? 0 : 1;
        }} catch {{ return -1; }}
    }}
}}
"@
$targetName = "{device_name}"
$endpoints = Get-ChildItem "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render" -ErrorAction SilentlyContinue
foreach ($ep in $endpoints) {{
    $p = Get-ItemProperty "$($ep.PSPath)\Properties" -ErrorAction SilentlyContinue
    if ($p) {{
        $n1 = [string]$p.'{{a45c254e-df1c-4efd-8020-67d146a850e0}},2'
        $n2 = [string]$p.'{{b3f8fa53-0004-438e-9003-51a46e139bfc}},6'
        $full1 = "$n1 ($n2)"
        $full2 = "$n2 ($n1)"
        
        $isMatch = $false
        if ($targetName -eq $full1 -or $targetName -eq $full2 -or $targetName -eq $n2) {{
            $isMatch = $true
        }} elseif ($n2.Length -ge 3 -and ($targetName.IndexOf($n2, [System.StringComparison]::OrdinalIgnoreCase) -ge 0 -or $n2.IndexOf($targetName, [System.StringComparison]::OrdinalIgnoreCase) -ge 0)) {{
            $isMatch = $true
        }} elseif ($n1.Length -ge 6 -and ($targetName.IndexOf($n1, [System.StringComparison]::OrdinalIgnoreCase) -ge 0 -or $n1.IndexOf($targetName, [System.StringComparison]::OrdinalIgnoreCase) -ge 0)) {{
            $isMatch = $true
        }}
        
        if ($isMatch) {{
            $devId = "{{0.0.0.00000000}}.$($ep.PSChildName)"
            [AudioSwitcher]::SetDefault($devId) | Out-Null
            break
        }}
    }}
}}
"#
    );

    let _ = std::process::Command::new("powershell")
        .creation_flags(0x08000000)
        .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &script])
        .output();

    Ok(())
}

#[cfg(not(windows))]
pub fn set_windows_default_playback_device(_device_name: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(not(windows))]
pub fn get_system_mute() -> bool {
    OUTPUT_MUTED.load(Ordering::Relaxed)
}

pub fn update_output_eq_profile(profile: EqProfile) {
    if let Ok(mut lock) = OUTPUT_EQ_PROFILE.lock() {
        *lock = profile;
    }
    EQ_PROFILE_CHANGED.store(true, Ordering::Relaxed);
}

#[allow(clippy::too_many_arguments)]
pub fn start_dsp_engine(
    app_handle: tauri::AppHandle,
    source_name: &str,
    output_name: &str,
    is_asio: bool,
    headroom_db: f32,
    target_sample_rate: Option<u32>,
    filter_type: &str,
    output_sample_format: Option<&str>,
) -> Result<(), String> {
    let _transition = ENGINE_TRANSITION
        .lock()
        .map_err(|_| "Audio engine transition lock poisoned")?;
    stop_dsp_engine_inner()?;
    INPUT_OVERRUNS.store(0, Ordering::Relaxed);
    OUTPUT_OVERRUNS.store(0, Ordering::Relaxed);
    OUTPUT_UNDERRUNS.store(0, Ordering::Relaxed);

    let host = get_host(is_asio);
    let (source_device, source_is_input) =
        find_source_device(&host, source_name).ok_or("Audio source device not found")?;
    let output_device = find_output_device(&host, output_name).ok_or("Output device not found")?;
    let output_config =
        find_output_config_with_target(&output_device, target_sample_rate, output_sample_format)?;

    let source_config = if source_is_input {
        find_input_config_with_target(
            &source_device,
            Some(output_config.sample_rate()), // Always try to match output sample rate
        )?
    } else {
        find_best_output_config(&source_device)?
    };

    let input_rate = source_config.sample_rate() as f64;
    let output_rate = output_config.sample_rate() as f64;
    let input_channels = source_config.channels() as usize;
    let output_channels = output_config.channels() as usize;
    let chunk_size = 1024;
    let input_capacity =
        ((input_rate as usize / 4) * input_channels).max(chunk_size * input_channels * 4);
    let output_capacity =
        ((output_rate as usize / 4) * output_channels).max(chunk_size * output_channels * 4);

    let input_ring = HeapRb::<f64>::new(input_capacity);
    let (mut input_producer, mut input_consumer) = input_ring.split();
    let output_ring = HeapRb::<f64>::new(output_capacity);
    let (mut output_producer, output_consumer) = output_ring.split();
    let gain = 10.0_f64.powf(headroom_db as f64 / 20.0);

    let mut resampler = if (input_rate - output_rate).abs() < 1.0 {
        None
    } else {
        let (sinc_len, f_cutoff, oversampling_factor, window) = match filter_type {
            "linear_precise" => (256, 0.96, 256, WindowFunction::BlackmanHarris2),
            "linear_smooth" => (96, 0.92, 96, WindowFunction::BlackmanHarris2),
            "phase_smooth" => (64, 0.88, 64, WindowFunction::Hann),
            "biophys_phase_flash" => (128, 0.98, 512, WindowFunction::BlackmanHarris2),
            "biophys_acoustic_smooth" => (96, 0.94, 256, WindowFunction::Hann),
            _ => (128, 0.95, 128, WindowFunction::Hann),
        };
        let params = SincInterpolationParameters {
            sinc_len,
            f_cutoff,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor,
            window,
        };
        Some(
            SincFixedIn::<f64>::new(
                output_rate / input_rate,
                10.0,
                params,
                chunk_size,
                input_channels,
            )
            .map_err(|error| format!("Resampler setup error: {error}"))?,
        )
    };

    IS_RUNNING.store(true, Ordering::SeqCst);
    let running = IS_RUNNING.clone();
    let mut worker_slot = WORKER_THREAD
        .lock()
        .map_err(|_| "Audio worker lock poisoned")?;
    let worker = std::thread::Builder::new()
        .name("vesper-biophys-dsp-worker".to_string())
        .spawn(move || {
            // 🚀 [BioPhys 17.0 Real-Time Core Pinning & Time-Critical Priority]
            #[cfg(target_os = "windows")]
            unsafe {
                use windows::Win32::System::Threading::{
                    GetCurrentThread, SetThreadAffinityMask, SetThreadPriority,
                    THREAD_PRIORITY_TIME_CRITICAL,
                };
                let thread = GetCurrentThread();
                let _ = SetThreadPriority(thread, THREAD_PRIORITY_TIME_CRITICAL);
                // OS 코어 #0 인터럽트를 피해 고성능 코어 #2에 고정 바인딩
                let _ = SetThreadAffinityMask(thread, 1 << 2);
            }

            let mut channels = vec![vec![0.0; chunk_size]; input_channels];
            let mut interleaved = vec![0.0; chunk_size * input_channels];
            let has_resampler = resampler.is_some();
            let max_output_frames = resampler
                .as_ref()
                .map(Resampler::output_frames_max)
                .unwrap_or(chunk_size);
            let mut resampled = resampler
                .as_ref()
                .map(|value| value.output_buffer_allocate(true))
                .unwrap_or_default();
            let mut output_interleaved = vec![0.0; max_output_frames * output_channels];

            while running.load(Ordering::SeqCst) {
                // ⚡ [BioPhys 17.0 Zero-Sleep Adaptive Spin-Pause (지연시간 0ns)]
                let mut spin_count = 0u32;
                while input_consumer.occupied_len() < interleaved.len() && running.load(Ordering::Relaxed) {
                    std::hint::spin_loop();
                    spin_count += 1;
                    if spin_count > 500 {
                        std::thread::yield_now();
                        spin_count = 0;
                    }
                }
                if !running.load(Ordering::Relaxed) {
                    break;
                }

                if input_consumer.pop_slice(&mut interleaved) != interleaved.len() {
                    continue;
                }
                for frame in 0..chunk_size {
                    for channel in 0..input_channels {
                        channels[channel][frame] = interleaved[frame * input_channels + channel];
                    }
                }

                let output_frames = match resampler.as_mut() {
                    Some(resampler) => {
                        match resampler.process_into_buffer(&channels, &mut resampled, None) {
                            Ok((_, frames)) => frames,
                            Err(error) => {
                                eprintln!("Resampler processing error: {error}");
                                continue;
                            }
                        }
                    }
                    None => chunk_size,
                };
                let output = if has_resampler { &resampled } else { &channels };
                let muted = OUTPUT_MUTED.load(Ordering::Relaxed);
                let sample_count = output_frames * output_channels;
                for frame in 0..output_frames {
                    for channel in 0..output_channels {
                        output_interleaved[frame * output_channels + channel] = if muted {
                            0.0
                        } else {
                            output[channel % input_channels][frame]
                        };
                    }
                }
                let pushed = output_producer.push_slice(&output_interleaved[..sample_count]);
                if pushed < sample_count {
                    OUTPUT_OVERRUNS.fetch_add((sample_count - pushed) as u64, Ordering::Relaxed);
                }
            }
        })
        .map_err(|e| format!("Failed to spawn DSP worker thread: {e}"))?;
    *worker_slot = Some(worker);
    drop(worker_slot);

    fn i16_to_f64(sample: i16) -> f64 {
        sample as f64 / i16::MAX as f64
    }
    fn i32_to_f64(sample: i32) -> f64 {
        sample as f64 / i32::MAX as f64
    }
    fn i24_to_f64(sample: cpal::I24) -> f64 {
        sample.to_sample::<f32>() as f64
    }
    fn u16_to_f64(sample: u16) -> f64 {
        (sample as f64 / u16::MAX as f64) * 2.0 - 1.0
    }
    fn f64_to_i16(sample: f64) -> i16 {
        (sample.clamp(-1.0, 1.0) * i16::MAX as f64) as i16
    }
    fn f64_to_i32(sample: f64) -> i32 {
        (sample.clamp(-1.0, 1.0) * i32::MAX as f64) as i32
    }
    fn f64_to_i24(sample: f64) -> cpal::I24 {
        cpal::I24::new((sample.clamp(-1.0, 1.0) * 8_388_607.0) as i32)
            .unwrap_or_else(|| cpal::I24::new(0).expect("zero is a valid I24 sample"))
    }
    fn f64_to_u16(sample: f64) -> u16 {
        ((sample.clamp(-1.0, 1.0) + 1.0) * 0.5 * u16::MAX as f64) as u16
    }

    macro_rules! build_output {
        ($sample:ty, $convert:expr) => {{
            let mut consumer = output_consumer;
            let app = app_handle.clone();
            let error_app = app_handle.clone();
            let mut channel = 0usize;
            let channel_count = output_channels;
            let profile = OUTPUT_EQ_PROFILE
                .lock()
                .map(|profile| profile.clone())
                .unwrap_or_default();
            let mut left_filters = Vec::new();
            let mut right_filters = Vec::new();
            if profile.enabled {
                for band in &profile.bands {
                    let filter_type = match band.filter_type.as_str() {
                        "low_shelf" => biquad::Type::LowShelf(band.gain_db),
                        "high_shelf" => biquad::Type::HighShelf(band.gain_db),
                        "low_pass" => biquad::Type::LowPass,
                        "high_pass" => biquad::Type::HighPass,
                        "band_pass" => biquad::Type::BandPass,
                        "band_stop" | "notch" => biquad::Type::Notch,
                        "peaking" | "peak" => biquad::Type::PeakingEQ(band.gain_db),
                        _ => biquad::Type::PeakingEQ(band.gain_db), // 기본값
                    };
                    if let Ok(coefficients) = Coefficients::<f64>::from_params(
                        filter_type,
                        output_rate.hz(),
                        band.frequency.hz(),
                        band.q,
                    ) {
                        left_filters.push(DirectForm1::<f64>::new(coefficients));
                        right_filters.push(DirectForm1::<f64>::new(coefficients));
                    }
                }
            }
            let mut preamp_gain = 10_f64.powf(profile.preamp_gain / 20.0);
            let mut is_eq_enabled = profile.enabled;
            let fade_in_samples: usize = (output_rate as usize * output_channels / 20).max(1); // 50ms 부드러운 페이드인
            let total_fade_samples = fade_in_samples as f64;
            let mut current_fade_sample = 0usize;

            output_device
                .build_output_stream(
                    output_config.clone().into(),
                    move |data: &mut [$sample], _| {
                        if EQ_PROFILE_CHANGED.swap(false, Ordering::Relaxed) {
                            if let Ok(new_profile) = OUTPUT_EQ_PROFILE.try_lock() {
                                left_filters.clear();
                                right_filters.clear();
                                if new_profile.enabled {
                                    for band in &new_profile.bands {
                                        let filter_type = match band.filter_type.as_str() {
                                            "low_shelf" => biquad::Type::LowShelf(band.gain_db),
                                            "high_shelf" => biquad::Type::HighShelf(band.gain_db),
                                            "low_pass" => biquad::Type::LowPass,
                                            "high_pass" => biquad::Type::HighPass,
                                            "band_pass" => biquad::Type::BandPass,
                                            "band_stop" | "notch" => biquad::Type::Notch,
                                            "peaking" | "peak" => {
                                                biquad::Type::PeakingEQ(band.gain_db)
                                            }
                                            _ => biquad::Type::PeakingEQ(band.gain_db),
                                        };
                                        if let Ok(coefficients) = Coefficients::<f64>::from_params(
                                            filter_type,
                                            output_rate.hz(),
                                            band.frequency.hz(),
                                            band.q,
                                        ) {
                                            left_filters
                                                .push(DirectForm1::<f64>::new(coefficients));
                                            right_filters
                                                .push(DirectForm1::<f64>::new(coefficients));
                                        }
                                    }
                                }
                                preamp_gain = 10_f64.powf(new_profile.preamp_gain / 20.0);
                                is_eq_enabled = new_profile.enabled;
                            } else {
                                EQ_PROFILE_CHANGED.store(true, Ordering::Relaxed);
                            }
                        }

                        let muted = OUTPUT_MUTED.load(Ordering::Relaxed);
                        if muted {
                            for target in data.iter_mut() {
                                *target = Sample::EQUILIBRIUM;
                            }
                            return;
                        }

                        let mut underruns = 0_u64;
                        let mut clipped = false;
                        for target in data {
                            let mut sample = match consumer.try_pop() {
                                Some(sample) => sample,
                                None => {
                                    underruns += 1;
                                    0.0
                                }
                            };

                            // 🛡️ [앱 켜질 때 소리 폭주/팝노이즈 원천 차단: 50ms 소프트 페이드인]
                            if current_fade_sample < fade_in_samples {
                                let ramp = (std::f64::consts::PI * 0.5 * (current_fade_sample as f64 / total_fade_samples)).sin();
                                sample *= ramp;
                                current_fade_sample += 1;
                            }
                            if is_eq_enabled {
                                sample *= preamp_gain;
                                let filters = if channel % 2 == 0 {
                                    &mut left_filters
                                } else {
                                    &mut right_filters
                                };
                                for filter in filters {
                                    sample = filter.run(sample);
                                }
                            }
                            clipped |= !(-1.0..=1.0).contains(&sample);
                            *target = $convert(sample);
                            channel = (channel + 1) % channel_count;
                        }
                        if underruns > 0 {
                            OUTPUT_UNDERRUNS.fetch_add(underruns, Ordering::Relaxed);
                        }
                        if clipped {
                            let now = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as i64;
                            if now - LAST_CLIP_TIME.load(Ordering::Relaxed) > 200 {
                                LAST_CLIP_TIME.store(now, Ordering::Relaxed);
                                let _ = app.emit("clipping-detected", "output");
                            }
                        }
                    },
                    move |error| report_stream_error(&error_app, "output", error),
                    None,
                )
                .map_err(|error| format!("Output stream error: {error}"))
        }};
    }

    let output_stream = match output_config.sample_format() {
        cpal::SampleFormat::F32 => build_output!(f32, |sample: f64| sample as f32),
        cpal::SampleFormat::I16 => build_output!(i16, f64_to_i16),
        cpal::SampleFormat::I24 => build_output!(cpal::I24, f64_to_i24),
        cpal::SampleFormat::I32 => build_output!(i32, f64_to_i32),
        cpal::SampleFormat::U16 => build_output!(u16, f64_to_u16),
        format => Err(format!("Unsupported output format: {format:?}")),
    };
    let output_stream = match output_stream {
        Ok(stream) => stream,
        Err(error) => {
            IS_RUNNING.store(false, Ordering::SeqCst);
            return Err(error);
        }
    };

    macro_rules! build_input {
        ($sample:ty, $convert:expr) => {{
            let error_app = app_handle.clone();
            source_device
                .build_input_stream(
                    source_config.clone().into(),
                    move |data: &[$sample], _| {
                        let mut overruns = 0_u64;
                        for sample in data {
                            if input_producer.try_push($convert(*sample) * gain).is_err() {
                                overruns += 1;
                            }
                        }
                        if overruns > 0 {
                            INPUT_OVERRUNS.fetch_add(overruns, Ordering::Relaxed);
                        }
                    },
                    move |error| report_stream_error(&error_app, "input", error),
                    None,
                )
                .map_err(|error| format!("Input stream error: {error}"))
        }};
    }

    let input_stream = match source_config.sample_format() {
        cpal::SampleFormat::F32 => build_input!(f32, |sample: f32| sample as f64),
        cpal::SampleFormat::I16 => build_input!(i16, i16_to_f64),
        cpal::SampleFormat::I24 => build_input!(cpal::I24, i24_to_f64),
        cpal::SampleFormat::I32 => build_input!(i32, i32_to_f64),
        cpal::SampleFormat::U16 => build_input!(u16, u16_to_f64),
        format => Err(format!("Unsupported input format: {format:?}")),
    };
    let input_stream = match input_stream {
        Ok(stream) => stream,
        Err(error) => {
            IS_RUNNING.store(false, Ordering::SeqCst);
            return Err(error);
        }
    };

    input_stream.play().map_err(|error| {
        IS_RUNNING.store(false, Ordering::SeqCst);
        error.to_string()
    })?;
    output_stream.play().map_err(|error| {
        IS_RUNNING.store(false, Ordering::SeqCst);
        error.to_string()
    })?;
    let mut streams = STREAMS.lock().map_err(|_| "Audio stream lock poisoned")?;
    streams.push(input_stream);
    streams.push(output_stream);

    // 실제 스트림 파라미터를 static 변수에 저장 (프론트엔드에서 커맨드로 조회 가능)
    let source_format = source_config.sample_format();
    let output_format = output_config.sample_format();
    if let Ok(mut info) = STREAM_INFO.lock() {
        *info = Some(StreamInfo {
            source_sample_rate: input_rate as u32,
            source_bit_depth: sample_format_label(source_format)
                .unwrap_or("Unknown")
                .to_string(),
            source_channels: input_channels,
            output_sample_rate: output_rate as u32,
            output_bit_depth: sample_format_label(output_format)
                .unwrap_or("Unknown")
                .to_string(),
            output_channels,
        });
    }
    let _ = app_handle.emit("dsp-stream-info", ());

    // 🛡️ [기본 음성 우회 방지: Source 장치가 가상 케이블인 경우 Windows 기본 재생 장치로 강제 고정]
    if is_virtual_cable_device(source_name) {
        let _ = set_windows_default_playback_device(source_name);
    }

    println!("Vesper DSP active: {input_rate}Hz input -> {output_rate}Hz output");
    Ok(())
}

pub fn stop_dsp_engine(output_name: Option<&str>) -> Result<(), String> {
    let _transition = ENGINE_TRANSITION
        .lock()
        .map_err(|_| "Audio engine transition lock poisoned")?;
    if let Some(out_dev) = output_name {
        let _ = set_windows_default_playback_device(out_dev);
    }
    stop_dsp_engine_inner()
}

fn stop_dsp_engine_inner() -> Result<(), String> {
    IS_RUNNING.store(false, Ordering::SeqCst);
    {
        let mut streams = STREAMS.lock().map_err(|_| "Audio stream lock poisoned")?;
        for stream in streams.iter() {
            let _ = stream.pause();
        }
        streams.clear();
    }
    if let Some(worker) = WORKER_THREAD
        .lock()
        .map_err(|_| "Audio worker lock poisoned")?
        .take()
    {
        worker
            .join()
            .map_err(|_| "Audio worker thread panicked".to_string())?;
    }
    if let Ok(mut info) = STREAM_INFO.lock() {
        *info = None;
    }
    Ok(())
}
