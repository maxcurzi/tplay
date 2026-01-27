//! Shared playback synchronization state between audio and video threads.
//! Audio acts as the master clock and updates the position, video reads it to sync frames.
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

pub struct PlaybackClock {
    position_nanos: AtomicU64,
    paused: AtomicBool,
    /// Playback speed stored as f32 bits (use f32::to_bits/from_bits).
    speed_bits: AtomicU32,
}

impl PlaybackClock {
    pub fn new() -> Self {
        Self {
            position_nanos: AtomicU64::new(0),
            paused: AtomicBool::new(true),
            speed_bits: AtomicU32::new(1.0_f32.to_bits()),
        }
    }

    pub fn set_position(&self, pos: Duration) {
        self.position_nanos.store(pos.as_nanos() as u64, Ordering::Release);
    }

    pub fn get_position(&self) -> Duration {
        Duration::from_nanos(self.position_nanos.load(Ordering::Acquire))
    }

    pub fn set_paused(&self, paused: bool) {
        self.paused.store(paused, Ordering::Release);
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Acquire)
    }

    /// Sets the playback speed multiplier.
    pub fn set_speed(&self, speed: f32) {
        self.speed_bits.store(speed.to_bits(), Ordering::Release);
    }

    /// Gets the current playback speed multiplier.
    #[allow(dead_code)]
    pub fn get_speed(&self) -> f32 {
        f32::from_bits(self.speed_bits.load(Ordering::Acquire))
    }
}

impl Default for PlaybackClock {
    fn default() -> Self {
        Self::new()
    }
}
