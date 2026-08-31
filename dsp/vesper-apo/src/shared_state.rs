use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;

pub const SHARED_MEMORY_NAME: &str = "Global\\VesperDspApoSharedMemory";
pub const MAX_EQ_BANDS: usize = 32;

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
    pub is_enabled: AtomicBool,
    pub preamp_gain_db: f32,
    pub band_count: u32,
    pub bands: [ApoEqBand; MAX_EQ_BANDS],
    pub update_sequence: AtomicU64,
}

impl Default for VesperApoSharedState {
    fn default() -> Self {
        Self {
            is_enabled: AtomicBool::new(true),
            preamp_gain_db: 0.0,
            band_count: 0,
            bands: [ApoEqBand::default(); MAX_EQ_BANDS],
            update_sequence: AtomicU64::new(0),
        }
    }
}
