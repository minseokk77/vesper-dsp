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
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::Emitter;

static IS_RUNNING: Lazy<Arc<AtomicBool>> = Lazy::new(|| Arc::new(AtomicBool::new(false)));
static LAST_CLIP_TIME: Lazy<AtomicI64> = Lazy::new(|| AtomicI64::new(0));
static STREAMS: Lazy<Mutex<Vec<cpal::Stream>>> = Lazy::new(|| Mutex::new(Vec::new()));

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

#[derive(serde::Serialize)]
pub struct BitDepthOption {
    pub value: String,
    pub label: String,
}

fn get_host(_is_asio: bool) -> cpal::Host {
    cpal::default_host()
}

pub fn get_available_devices(is_asio: bool) -> Vec<String> {
    let host = get_host(is_asio);
    let mut device_names = Vec::new();

    if let Ok(devices) = host.output_devices() {
        for device in devices {
            device_names.push(device.to_string());
        }
    }
    if let Ok(devices) = host.input_devices() {
        for device in devices {
            let name = device.to_string();
            if !device_names.contains(&name) {
                device_names.push(name);
            }
        }
    }
    device_names
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
                output_sample_format.map(|wanted| format == wanted).unwrap_or(true)
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
            device.supported_input_configs()
                .map(|iter| iter.collect())
                .unwrap_or_default()
        } else {
            device.supported_output_configs()
                .map(|iter| iter.collect())
                .unwrap_or_default()
        };

        for sc in supported_configs {
            match sc.sample_format() {
                cpal::SampleFormat::I16 => {
                    if max_int_bit < 16 { max_int_bit = 16; }
                }
                cpal::SampleFormat::I32 => {
                    let bit = if device_name.to_lowercase().contains("fiio") { 32 } else { 24 };
                    if max_int_bit < bit { max_int_bit = bit; }
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
}

pub fn update_output_eq_profile(profile: EqProfile) {
    if let Ok(mut lock) = OUTPUT_EQ_PROFILE.lock() {
        *lock = profile;
    }
    EQ_PROFILE_CHANGED.store(true, Ordering::Relaxed);
}

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
    stop_dsp_engine()?;
    IS_RUNNING.store(true, Ordering::SeqCst);

    let host = get_host(is_asio);
    let (source_device, source_is_input) =
        find_device(&host, source_name).ok_or("Audio source device not found")?;
    let (output_device, _) = find_device(&host, output_name).ok_or("Output device not found")?;
    let source_config = if source_is_input {
        find_best_input_config(&source_device)?
    } else {
        find_best_output_config(&source_device)?
    };
    let output_config =
        find_output_config_with_target(&output_device, target_sample_rate, output_sample_format)?;

    let input_rate = source_config.sample_rate() as f64;
    let output_rate = output_config.sample_rate() as f64;
    let input_channels = source_config.channels() as usize;
    let output_channels = output_config.channels() as usize;
    let chunk_size = 1024;
    let input_capacity = (input_rate * input_channels as f64 * 2.0) as usize;
    let output_capacity = (output_rate * output_channels as f64 * 2.0) as usize;

    let input_ring = HeapRb::<f64>::new(input_capacity);
    let (mut input_producer, mut input_consumer) = input_ring.split();
    let output_ring = HeapRb::<f64>::new(output_capacity);
    let (mut output_producer, output_consumer) = output_ring.split();
    let gain = 10.0_f64.powf(headroom_db as f64 / 20.0);

    let mut resampler = if (input_rate - output_rate).abs() < 1.0 {
        None
    } else {
        let (sinc_len, f_cutoff, oversampling_factor, window) = match filter_type {
            "정밀한, 선형 위상" => (256, 0.96, 256, WindowFunction::BlackmanHarris2),
            "정확한 최소 단계 (Minimum Phase)" => (128, 0.95, 128, WindowFunction::Hann),
            "부드러움, 리니어 위상" => (96, 0.92, 96, WindowFunction::BlackmanHarris2),
            _ => (64, 0.88, 64, WindowFunction::Hann),
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

    let running = IS_RUNNING.clone();
    std::thread::spawn(move || {
        let mut channels = vec![vec![0.0; chunk_size]; input_channels];
        let mut interleaved = vec![0.0; chunk_size * input_channels];
        while running.load(Ordering::SeqCst) {
            if input_consumer.occupied_len() < interleaved.len() {
                std::thread::sleep(std::time::Duration::from_millis(1));
                continue;
            }
            for sample in &mut interleaved {
                *sample = input_consumer.try_pop().unwrap_or_default();
            }
            for frame in 0..chunk_size {
                for channel in 0..input_channels {
                    channels[channel][frame] = interleaved[frame * input_channels + channel];
                }
            }

            let output = match resampler.as_mut() {
                Some(resampler) => match resampler.process(&channels, None) {
                    Ok(output) => output,
                    Err(error) => {
                        eprintln!("Resampler processing error: {error}");
                        continue;
                    }
                },
                None => channels.clone(),
            };
            for frame in 0..output[0].len() {
                for channel in 0..output_channels {
                    let sample = if OUTPUT_MUTED.load(Ordering::Relaxed) {
                        0.0
                    } else {
                        output[channel % input_channels][frame]
                    };
                    let _ = output_producer.try_push(sample);
                }
            }
        }
    });

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
                                            "peaking" | "peak" => biquad::Type::PeakingEQ(band.gain_db),
                                            _ => biquad::Type::PeakingEQ(band.gain_db),
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
                                preamp_gain = 10_f64.powf(new_profile.preamp_gain / 20.0);
                                is_eq_enabled = new_profile.enabled;
                            } else {
                                EQ_PROFILE_CHANGED.store(true, Ordering::Relaxed);
                            }
                        }

                        for target in data {
                            let mut sample = consumer.try_pop().unwrap_or_default();
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
                            if sample > 1.0 || sample < -1.0 {
                                let now = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as i64;
                                if now - LAST_CLIP_TIME.load(Ordering::Relaxed) > 200 {
                                    LAST_CLIP_TIME.store(now, Ordering::Relaxed);
                                    let _ = app.emit("clipping-detected", "output");
                                }
                            }
                            *target = $convert(sample);
                            channel = (channel + 1) % channel_count;
                        }
                    },
                    |error| eprintln!("Output stream error: {error}"),
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
        format => return Err(format!("Unsupported output format: {format:?}")),
    }?;

    macro_rules! build_input {
        ($sample:ty, $convert:expr) => {{
            source_device
                .build_input_stream(
                    source_config.clone().into(),
                    move |data: &[$sample], _| {
                        for sample in data {
                            let _ = input_producer.try_push($convert(*sample) * gain);
                        }
                    },
                    |error| eprintln!("Input stream error: {error}"),
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
        format => return Err(format!("Unsupported input format: {format:?}")),
    }?;

    input_stream.play().map_err(|error| error.to_string())?;
    output_stream.play().map_err(|error| error.to_string())?;
    let mut streams = STREAMS.lock().map_err(|_| "Audio stream lock poisoned")?;
    streams.push(input_stream);
    streams.push(output_stream);
    println!("Vesper DSP active: {input_rate}Hz input -> {output_rate}Hz output");
    Ok(())
}

pub fn stop_dsp_engine() -> Result<(), String> {
    IS_RUNNING.store(false, Ordering::SeqCst);
    let mut streams = STREAMS.lock().map_err(|_| "Audio stream lock poisoned")?;
    streams.clear();
    Ok(())
}
