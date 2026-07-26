use biquad::{Biquad, Coefficients, DirectForm1, ToHertz, Q_BUTTERWORTH_F64};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
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

pub static EARPHONE_MUTED: AtomicBool = AtomicBool::new(false);
pub static SPEAKER_MUTED: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EqBand {
    pub filter_type: String, // "low_shelf", "high_shelf", "peak_dip"
    pub frequency: f64,
    pub gain_db: f64,
    pub q: f64,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct EqProfile {
    pub enabled: bool,
    pub preamp_gain: f64,
    pub bands: Vec<EqBand>,
}

pub static EARPHONE_EQ_PROFILE: Lazy<Arc<Mutex<EqProfile>>> =
    Lazy::new(|| Arc::new(Mutex::new(EqProfile::default())));

pub fn set_earphone_mute(muted: bool) {
    EARPHONE_MUTED.store(muted, Ordering::SeqCst);
}

pub fn set_speaker_mute(muted: bool) {
    SPEAKER_MUTED.store(muted, Ordering::SeqCst);
}

pub fn update_earphone_eq_profile(profile: EqProfile) {
    if let Ok(mut lock) = EARPHONE_EQ_PROFILE.lock() {
        *lock = profile;
    }
}

fn get_host() -> cpal::Host {
    cpal::default_host()
}

pub fn get_available_devices() -> Vec<String> {
    let host = get_host();
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

fn find_best_output_config(device: &cpal::Device) -> Result<cpal::SupportedStreamConfig, String> {
    if let Ok(default_config) = device.default_output_config() {
        return Ok(default_config);
    }
    let mut configs = device
        .supported_output_configs()
        .map_err(|_| "No output config".to_string())?
        .collect::<Vec<_>>();
    configs.sort_by(|a, b| b.max_sample_rate().cmp(&a.max_sample_rate()));
    Ok(configs.into_iter().next().unwrap().with_max_sample_rate())
}

fn find_output_config_with_target(
    device: &cpal::Device,
    target_sr: Option<u32>,
) -> Result<cpal::SupportedStreamConfig, String> {
    if let Some(target) = target_sr {
        if let Ok(configs) = device.supported_output_configs() {
            for config in configs {
                let min = config.min_sample_rate();
                let max = config.max_sample_rate();
                if target >= min && target <= max {
                    return Ok(config.with_sample_rate(target as cpal::SampleRate));
                }
            }
        }
    }
    find_best_output_config(device)
}

fn find_best_input_config(device: &cpal::Device) -> Result<cpal::SupportedStreamConfig, String> {
    if let Ok(default_config) = device.default_input_config() {
        return Ok(default_config);
    }
    let mut configs = device
        .supported_input_configs()
        .map_err(|_| "No input config".to_string())?
        .collect::<Vec<_>>();
    configs.sort_by(|a, b| b.max_sample_rate().cmp(&a.max_sample_rate()));
    Ok(configs.into_iter().next().unwrap().with_max_sample_rate())
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

pub fn get_device_sample_rate(device_name: &str) -> Result<u32, String> {
    let host = get_host();
    let (dev, is_input) = find_device(&host, device_name).ok_or("Device not found")?;
    let config = if is_input {
        find_best_input_config(&dev)?
    } else {
        find_best_output_config(&dev)?
    };
    Ok(config.sample_rate())
}

pub fn get_device_bit_depth(device_name: &str) -> Result<String, String> {
    let host = get_host();
    let (dev, is_input) = find_device(&host, device_name).ok_or("Device not found")?;
    let config = if is_input {
        find_best_input_config(&dev)?
    } else {
        find_best_output_config(&dev)?
    };

    let mut current_format = match config.sample_format() {
        cpal::SampleFormat::F32 => "32bit Float".to_string(),
        cpal::SampleFormat::I16 => "16bit".to_string(),
        cpal::SampleFormat::I32 => "32bit".to_string(),
        cpal::SampleFormat::U16 => "16bit Unsigned".to_string(),
        cpal::SampleFormat::U32 => "32bit Unsigned".to_string(),
        cpal::SampleFormat::I8 => "8bit".to_string(),
        cpal::SampleFormat::U8 => "8bit Unsigned".to_string(),
        cpal::SampleFormat::F64 => "64bit Float".to_string(),
        _ => "Unknown Format".to_string(),
    };

    if current_format == "32bit Float" {
        let mut max_int_bit = 0;

        let supported_configs: Vec<_> = if is_input {
            dev.supported_input_configs()
                .map(|iter| iter.collect())
                .unwrap_or_default()
        } else {
            dev.supported_output_configs()
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
            current_format = format!("{}bit", max_int_bit);
        }
    }

    Ok(current_format)
}

pub fn get_device_supported_sample_rates(device_name: &str) -> Result<Vec<u32>, String> {
    let host = get_host();
    let (dev, is_input) = find_device(&host, device_name).ok_or("Device not found")?;

    let mut rates = Vec::new();
    let configs = if is_input {
        dev.supported_input_configs()
            .map_err(|_| "No input config")?
            .collect::<Vec<_>>()
    } else {
        dev.supported_output_configs()
            .map_err(|_| "No output config")?
            .collect::<Vec<_>>()
    };

    for config in configs {
        let min = config.min_sample_rate();
        let max = config.max_sample_rate();

        let std_rates = [
            44100, 48000, 88200, 96000, 176400, 192000, 352800, 384000, 705600, 768000,
        ];
        for &sr in &std_rates {
            if sr >= min && sr <= max && !rates.contains(&sr) {
                rates.push(sr);
            }
        }
    }
    rates.sort_unstable();
    Ok(rates)
}

pub fn start_audio_sync(
    app_handle: tauri::AppHandle,
    source_name: &str,
    earphone_name: &str,
    speaker_name: &str,
    delay_ms: u32,
    lpf_hz: f32,
    lpf_slope: u32,
    headroom_db: f32,
    earphone_target_sr: Option<u32>,
    speaker_target_sr: Option<u32>,
    earphone_filter: &str,
    speaker_filter: &str,
) -> Result<(), String> {
    if IS_RUNNING.load(Ordering::SeqCst) {
        let _ = stop_audio_sync();
    }

    IS_RUNNING.store(true, Ordering::SeqCst);

    let host = get_host();
    let (source_dev, is_source_input) = match find_device(&host, source_name) {
        Some((dev, is_input)) => (dev, is_input),
        None => {
            IS_RUNNING.store(false, Ordering::SeqCst);
            return Err("Source device not found".to_string());
        }
    };
    let (earphone_dev, _) = match find_device(&host, earphone_name) {
        Some((dev, _)) => (dev, false),
        None => {
            IS_RUNNING.store(false, Ordering::SeqCst);
            return Err("Earphone device not found".to_string());
        }
    };
    let (speaker_dev, _) = match find_device(&host, speaker_name) {
        Some((dev, _)) => (dev, false),
        None => {
            IS_RUNNING.store(false, Ordering::SeqCst);
            return Err("Speaker device not found".to_string());
        }
    };

    let source_config = if is_source_input {
        find_best_input_config(&source_dev)?
    } else {
        find_best_output_config(&source_dev)?
    };
    let earphone_config = find_output_config_with_target(&earphone_dev, earphone_target_sr)?;
    let speaker_config = find_output_config_with_target(&speaker_dev, speaker_target_sr)?;

    let in_rate = source_config.sample_rate() as f64;
    let in_channels = source_config.channels() as f64;
    let ear_rate = earphone_config.sample_rate() as f64;
    let ear_channels = earphone_config.channels() as f64;
    let spk_rate = speaker_config.sample_rate() as f64;
    let spk_channels = speaker_config.channels() as f64;

    println!(
        "Virtual Router Started: Source({}Hz) -> Earphone({}Hz) + Speaker({}Hz)",
        in_rate, ear_rate, spk_rate
    );

    let capacity_ear = (ear_rate * ear_channels * 10.0) as usize;
    let capacity_spk = (spk_rate * spk_channels * 10.0) as usize;

    let rb_ear = HeapRb::<f64>::new(capacity_ear);
    let (mut prod_ear, cons_ear) = rb_ear.split();

    let rb_spk = HeapRb::<f64>::new(capacity_spk);
    let (mut prod_spk, cons_spk) = rb_spk.split();

    let ear_delay_samples = (ear_rate * (delay_ms as f64 / 1000.0) * ear_channels) as usize;
    let ear_jitter = (ear_rate * 0.05 * ear_channels) as usize;
    for _ in 0..(ear_delay_samples + ear_jitter) {
        let _ = prod_ear.try_push(0.0f64);
    }

    let spk_jitter = (spk_rate * 0.05 * spk_channels) as usize;
    for _ in 0..spk_jitter {
        let _ = prod_spk.try_push(0.0f64);
    }

    macro_rules! build_output {
        ($dev:expr, $cfg:expr, $cons:expr, $type:ty, $cvt:expr, $is_woofer:expr, $app_handle:expr, $target_name:expr) => {{
            let err_fn = |err| eprintln!("Output error: {}", err);
            let mut cons = $cons;
            let config_into: cpal::StreamConfig = $cfg.clone().into();

            let fs = $cfg.sample_rate() as f64;
            let mut filter_l1: Option<DirectForm1<f64>> = None;
            let mut filter_l2: Option<DirectForm1<f64>> = None;
            let mut filter_r1: Option<DirectForm1<f64>> = None;
            let mut filter_r2: Option<DirectForm1<f64>> = None;

            if $is_woofer && lpf_slope > 0 && fs > 0.0 {
                let f0 = (lpf_hz as f64).hz();
                let f_s = fs.hz();
                if let Ok(coeffs) = Coefficients::<f64>::from_params(
                    biquad::Type::LowPass,
                    f_s,
                    f0,
                    Q_BUTTERWORTH_F64,
                ) {
                    filter_l1 = Some(DirectForm1::<f64>::new(coeffs));
                    filter_l2 = Some(DirectForm1::<f64>::new(coeffs));
                    filter_r1 = Some(DirectForm1::<f64>::new(coeffs));
                    filter_r2 = Some(DirectForm1::<f64>::new(coeffs));
                }
            }

            let mut channel = 0;
            let channels_count = $cfg.channels() as usize;

            let mut peq_filters_l: Vec<DirectForm1<f64>> = Vec::new();
            let mut peq_filters_r: Vec<DirectForm1<f64>> = Vec::new();
            let mut preamp_gain = 1.0;
            let mut peq_enabled = false;

            if !$is_woofer {
                if let Ok(profile) = EARPHONE_EQ_PROFILE.try_lock() {
                    peq_enabled = profile.enabled;
                    preamp_gain = 10_f64.powf(profile.preamp_gain / 20.0);

                    for band in &profile.bands {
                        let f0 = band.frequency.hz();
                        let q = band.q;
                        let filter_type = match band.filter_type.as_str() {
                            "low_shelf" => biquad::Type::LowShelf(band.gain_db),
                            "high_shelf" => biquad::Type::HighShelf(band.gain_db),
                            _ => biquad::Type::PeakingEQ(band.gain_db),
                        };

                        if let Ok(coeffs) =
                            Coefficients::<f64>::from_params(filter_type, fs.hz(), f0, q)
                        {
                            peq_filters_l.push(DirectForm1::<f64>::new(coeffs));
                            peq_filters_r.push(DirectForm1::<f64>::new(coeffs));
                        }
                    }
                }
            }

            let app = $app_handle.clone();
            let target = $target_name.to_string();

            $dev.build_output_stream(
                config_into,
                move |data: &mut [$type], _: &cpal::OutputCallbackInfo| {
                    for sample in data.iter_mut() {
                        let mut raw = cons.try_pop().unwrap_or(0.0f64);

                        if $is_woofer && lpf_slope > 0 {
                            let is_left = channel % 2 == 0;
                            if is_left {
                                if let Some(ref mut f) = filter_l1 {
                                    raw = f.run(raw);
                                }
                                if lpf_slope == 24 {
                                    if let Some(ref mut f) = filter_l2 {
                                        raw = f.run(raw);
                                    }
                                }
                            } else {
                                if let Some(ref mut f) = filter_r1 {
                                    raw = f.run(raw);
                                }
                                if lpf_slope == 24 {
                                    if let Some(ref mut f) = filter_r2 {
                                        raw = f.run(raw);
                                    }
                                }
                            }
                        }

                        if !$is_woofer && peq_enabled {
                            raw *= preamp_gain;
                            let is_left = channel % 2 == 0;
                            if is_left {
                                for f in &mut peq_filters_l {
                                    raw = f.run(raw);
                                }
                            } else {
                                for f in &mut peq_filters_r {
                                    raw = f.run(raw);
                                }
                            }
                        }

                        if raw > 1.0 || raw < -1.0 {
                            let now = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as i64;
                            let last = LAST_CLIP_TIME.load(Ordering::Relaxed);
                            if now - last > 200 {
                                LAST_CLIP_TIME.store(now, Ordering::Relaxed);
                                let _ = app.emit("clipping-detected", target.clone());
                            }
                        }

                        *sample = $cvt(raw);
                        channel = (channel + 1) % channels_count;
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| format!("Output stream error: {}", e))
        }};
    }

    fn i16_to_f64(s: i16) -> f64 {
        s as f64 / i16::MAX as f64
    }
    fn f64_to_i16(s: f64) -> i16 {
        (s.clamp(-1.0, 1.0) * i16::MAX as f64) as i16
    }

    fn i32_to_f64(s: i32) -> f64 {
        s as f64 / i32::MAX as f64
    }
    fn f64_to_i32(s: f64) -> i32 {
        (s.clamp(-1.0, 1.0) * i32::MAX as f64) as i32
    }

    fn u16_to_f64(s: u16) -> f64 {
        (s as f64 / u16::MAX as f64) * 2.0 - 1.0
    }
    fn f64_to_u16(s: f64) -> u16 {
        ((s.clamp(-1.0, 1.0) + 1.0) * 0.5 * u16::MAX as f64) as u16
    }

    fn f32_to_f64(s: f32) -> f64 {
        s as f64
    }
    fn f64_to_f32(s: f64) -> f32 {
        s as f32
    }

    let ear_stream = match earphone_config.sample_format() {
        cpal::SampleFormat::F32 => build_output!(
            earphone_dev,
            earphone_config,
            cons_ear,
            f32,
            f64_to_f32,
            false,
            app_handle.clone(),
            "earphone"
        ),
        cpal::SampleFormat::I16 => build_output!(
            earphone_dev,
            earphone_config,
            cons_ear,
            i16,
            f64_to_i16,
            false,
            app_handle.clone(),
            "earphone"
        ),
        cpal::SampleFormat::I32 => build_output!(
            earphone_dev,
            earphone_config,
            cons_ear,
            i32,
            f64_to_i32,
            false,
            app_handle.clone(),
            "earphone"
        ),
        cpal::SampleFormat::U16 => build_output!(
            earphone_dev,
            earphone_config,
            cons_ear,
            u16,
            f64_to_u16,
            false,
            app_handle.clone(),
            "earphone"
        ),
        _ => {
            return Err(format!(
                "Unsupported earphone format: {:?}",
                earphone_config.sample_format()
            ))
        }
    }?;

    let spk_stream = match speaker_config.sample_format() {
        cpal::SampleFormat::F32 => {
            build_output!(
                speaker_dev,
                speaker_config,
                cons_spk,
                f32,
                f64_to_f32,
                true,
                app_handle.clone(),
                "speaker"
            )
        }
        cpal::SampleFormat::I16 => {
            build_output!(
                speaker_dev,
                speaker_config,
                cons_spk,
                i16,
                f64_to_i16,
                true,
                app_handle.clone(),
                "speaker"
            )
        }
        cpal::SampleFormat::I32 => {
            build_output!(
                speaker_dev,
                speaker_config,
                cons_spk,
                i32,
                f64_to_i32,
                true,
                app_handle.clone(),
                "speaker"
            )
        }
        cpal::SampleFormat::U16 => {
            build_output!(
                speaker_dev,
                speaker_config,
                cons_spk,
                u16,
                f64_to_u16,
                true,
                app_handle.clone(),
                "speaker"
            )
        }
        _ => {
            return Err(format!(
                "Unsupported speaker format: {:?}",
                speaker_config.sample_format()
            ))
        }
    }?;

    let capacity_in = (in_rate * in_channels * 10.0) as usize;
    let rb_in = HeapRb::<f64>::new(capacity_in);
    let (mut prod_in, mut cons_in) = rb_in.split();

    let headroom_gain = 10.0_f64.powf(headroom_db as f64 / 20.0);

    let in_channels_usize = in_channels as usize;
    let ear_channels_usize = ear_channels as usize;
    let spk_channels_usize = spk_channels as usize;
    let chunk_size = 1024;

    let create_resampler =
        |target_rate: f64, filter_type: &str, channels: usize| -> Option<SincFixedIn<f64>> {
            if (in_rate - target_rate).abs() < 1.0 {
                return None;
            }
            let window = if filter_type.contains("선형 위상") {
                WindowFunction::BlackmanHarris2
            } else {
                WindowFunction::Hann
            };
            let params = SincInterpolationParameters {
                sinc_len: 128,
                f_cutoff: 0.95,
                interpolation: SincInterpolationType::Linear,
                oversampling_factor: 128,
                window,
            };
            SincFixedIn::<f64>::new(target_rate / in_rate, 10.0, params, chunk_size, channels).ok()
        };

    let mut resampler_ear = create_resampler(ear_rate, earphone_filter, in_channels_usize);
    let mut resampler_spk = create_resampler(spk_rate, speaker_filter, in_channels_usize);

    let is_running_clone = IS_RUNNING.clone();

    std::thread::spawn(move || {
        let mut in_buffer = vec![vec![0.0f64; chunk_size]; in_channels_usize];
        let mut interleaved_in = vec![0.0f64; chunk_size * in_channels_usize];

        while is_running_clone.load(Ordering::SeqCst) {
            if cons_in.occupied_len() >= chunk_size * in_channels_usize {
                for i in 0..(chunk_size * in_channels_usize) {
                    interleaved_in[i] = cons_in.try_pop().unwrap_or(0.0f64);
                }

                for ch in 0..in_channels_usize {
                    for i in 0..chunk_size {
                        in_buffer[ch][i] = interleaved_in[i * in_channels_usize + ch];
                    }
                }

                if let Some(ref mut rs) = resampler_ear {
                    if let Ok(out) = rs.process(&in_buffer, None) {
                        let out_frames = out[0].len();
                        for i in 0..out_frames {
                            for ch in 0..ear_channels_usize {
                                let in_ch = ch % in_channels_usize;
                                let val = if EARPHONE_MUTED.load(Ordering::Relaxed) {
                                    0.0
                                } else {
                                    out[in_ch][i]
                                };
                                let _ = prod_ear.try_push(val);
                            }
                        }
                    }
                } else {
                    for i in 0..chunk_size {
                        for ch in 0..ear_channels_usize {
                            let in_ch = ch % in_channels_usize;
                            let val = if EARPHONE_MUTED.load(Ordering::Relaxed) {
                                0.0
                            } else {
                                in_buffer[in_ch][i]
                            };
                            let _ = prod_ear.try_push(val);
                        }
                    }
                }

                if let Some(ref mut rs) = resampler_spk {
                    if let Ok(out) = rs.process(&in_buffer, None) {
                        let out_frames = out[0].len();
                        for i in 0..out_frames {
                            for ch in 0..spk_channels_usize {
                                let in_ch = ch % in_channels_usize;
                                let val = if SPEAKER_MUTED.load(Ordering::Relaxed) {
                                    0.0
                                } else {
                                    out[in_ch][i]
                                };
                                let _ = prod_spk.try_push(val);
                            }
                        }
                    }
                } else {
                    for i in 0..chunk_size {
                        for ch in 0..spk_channels_usize {
                            let in_ch = ch % in_channels_usize;
                            let val = if SPEAKER_MUTED.load(Ordering::Relaxed) {
                                0.0
                            } else {
                                in_buffer[in_ch][i]
                            };
                            let _ = prod_spk.try_push(val);
                        }
                    }
                }
            } else {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    });

    macro_rules! build_capture {
        ($type:ty, $cvt:expr) => {{
            let err_fn = |err| eprintln!("capture error: {}", err);
            let config_into: cpal::StreamConfig = source_config.clone().into();

            source_dev
                .build_input_stream(
                    config_into,
                    move |data: &[$type], _: &cpal::InputCallbackInfo| {
                        for &s in data {
                            let _ = prod_in.try_push($cvt(s) * headroom_gain);
                        }
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("Capture stream error: {}", e))
        }};
    }

    let source_stream = match source_config.sample_format() {
        cpal::SampleFormat::F32 => build_capture!(f32, f32_to_f64),
        cpal::SampleFormat::I16 => build_capture!(i16, i16_to_f64),
        cpal::SampleFormat::I32 => build_capture!(i32, i32_to_f64),
        cpal::SampleFormat::U16 => build_capture!(u16, u16_to_f64),
        _ => {
            return Err(format!(
                "Unsupported source format: {:?}",
                source_config.sample_format()
            ))
        }
    }?;

    source_stream.play().map_err(|e| {
        IS_RUNNING.store(false, Ordering::SeqCst);
        e.to_string()
    })?;
    ear_stream.play().map_err(|e| {
        IS_RUNNING.store(false, Ordering::SeqCst);
        e.to_string()
    })?;
    spk_stream.play().map_err(|e| {
        IS_RUNNING.store(false, Ordering::SeqCst);
        e.to_string()
    })?;

    let mut streams = STREAMS.lock().unwrap();
    streams.push(source_stream);
    streams.push(ear_stream);
    streams.push(spk_stream);

    Ok(())
}

pub fn stop_audio_sync() -> Result<(), String> {
    if IS_RUNNING.load(Ordering::SeqCst) {
        let mut streams = STREAMS.lock().unwrap();
        streams.clear();
        IS_RUNNING.store(false, Ordering::SeqCst);
        println!("Audio sync stopped.");
    }
    Ok(())
}
