//! Zero-allocation real-time audio DSP (512-point radix-2 Cooley-Tukey FFT, logarithmic band
//! spectrum, zero-crossing oscilloscope, and stereo VU metering) for the right-bar visualizer.

use std::f32::consts::TAU;
use std::sync::OnceLock;
use std::time::Instant;

use waywi::prelude::AudioCaptureFrame;

/// FFT window size in samples (power of 2; ~10.6 ms @ 48 kHz, ~93.75 Hz/bin).
pub const FFT_SIZE: usize = 512;
/// Number of logarithmically spaced frequency bands rendered by bar/wave modes.
pub const VIZ_BANDS: usize = 16;
/// Number of waveform vertices plotted by the oscilloscope mode.
pub const SCOPE_POINTS: usize = 64;

/// Visual presentation styles for the right-bar audio visualizer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VisualizerMode {
    #[default]
    Bars,
    Mirror,
    Oscilloscope,
    StereoVu,
    Wave,
}

impl VisualizerMode {
    #[cfg(test)]
    pub const ALL: [Self; 5] = [
        Self::Bars,
        Self::Mirror,
        Self::Oscilloscope,
        Self::StereoVu,
        Self::Wave,
    ];

    #[must_use]
    pub const fn id_u8(self) -> u8 {
        match self {
            Self::Bars => 0,
            Self::Mirror => 1,
            Self::Oscilloscope => 2,
            Self::StereoVu => 3,
            Self::Wave => 4,
        }
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bars => "bars",
            Self::Mirror => "mirror",
            Self::Oscilloscope => "oscilloscope",
            Self::StereoVu => "vu",
            Self::Wave => "wave",
        }
    }

    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "bars" | "spectrum" | "fft" => Some(Self::Bars),
            "mirror" | "symmetric" | "cava" => Some(Self::Mirror),
            "oscilloscope" | "osc" | "scope" => Some(Self::Oscilloscope),
            "vu" | "stereo" | "stereo_vu" | "meter" => Some(Self::StereoVu),
            "wave" | "area" | "curve" => Some(Self::Wave),
            _ => None,
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Bars => "Spectrum Bars",
            Self::Mirror => "Mirror Bars",
            Self::Oscilloscope => "Oscilloscope",
            Self::StereoVu => "Stereo VU",
            Self::Wave => "Wave Spectrum",
        }
    }

    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Bars => Self::Mirror,
            Self::Mirror => Self::Oscilloscope,
            Self::Oscilloscope => Self::StereoVu,
            Self::StereoVu => Self::Wave,
            Self::Wave => Self::Bars,
        }
    }

    #[must_use]
    pub const fn prev(self) -> Self {
        match self {
            Self::Bars => Self::Wave,
            Self::Mirror => Self::Bars,
            Self::Oscilloscope => Self::Mirror,
            Self::StereoVu => Self::Oscilloscope,
            Self::Wave => Self::StereoVu,
        }
    }

    /// Whether this mode requires raw interleaved PCM samples (VU mode runs on header-only levels).
    #[must_use]
    pub const fn needs_pcm(self) -> bool {
        !matches!(self, Self::StereoVu)
    }
}

/// Precomputed Hann window and radix-2 bit-reversal table (initialized once, read-only).
struct FftTables {
    hann: [f32; FFT_SIZE],
    bit_rev: [u16; FFT_SIZE],
    cos_twiddle: [f32; FFT_SIZE / 2],
    sin_twiddle: [f32; FFT_SIZE / 2],
}

fn fft_tables() -> &'static FftTables {
    static TABLES: OnceLock<FftTables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut hann = [0.0_f32; FFT_SIZE];
        let denom = (FFT_SIZE - 1) as f32;
        for (i, slot) in hann.iter_mut().enumerate() {
            *slot = 0.5 * (1.0 - (TAU * i as f32 / denom).cos());
        }

        let bits = FFT_SIZE.trailing_zeros();
        let mut bit_rev = [0_u16; FFT_SIZE];
        for (i, slot) in bit_rev.iter_mut().enumerate() {
            *slot = (i.reverse_bits() >> (usize::BITS - bits)) as u16;
        }

        let mut cos_twiddle = [0.0_f32; FFT_SIZE / 2];
        let mut sin_twiddle = [0.0_f32; FFT_SIZE / 2];
        for k in 0..(FFT_SIZE / 2) {
            let angle = -TAU * (k as f32) / (FFT_SIZE as f32);
            cos_twiddle[k] = angle.cos();
            sin_twiddle[k] = angle.sin();
        }

        FftTables {
            hann,
            bit_rev,
            cos_twiddle,
            sin_twiddle,
        }
    })
}

/// In-place 512-point radix-2 Cooley-Tukey FFT (zero heap allocations).
pub fn fft_radix2_512(re: &mut [f32; FFT_SIZE], im: &mut [f32; FFT_SIZE]) {
    let tables = fft_tables();
    for (i, &rev) in tables.bit_rev.iter().enumerate() {
        let j = rev as usize;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }

    let mut len = 2;
    while len <= FFT_SIZE {
        let half = len / 2;
        let step = FFT_SIZE / len;
        let mut base = 0;
        while base < FFT_SIZE {
            for k in 0..half {
                let tw = k * step;
                let wr = tables.cos_twiddle[tw];
                let wi = tables.sin_twiddle[tw];
                let i0 = base + k;
                let i1 = i0 + half;
                let tr = wr * re[i1] - wi * im[i1];
                let ti = wr * im[i1] + wi * re[i1];
                re[i1] = re[i0] - tr;
                im[i1] = im[i0] - ti;
                re[i0] += tr;
                im[i0] += ti;
            }
            base += len;
        }
        len <<= 1;
    }
}

/// Real-time smoothed spectrum, waveform, and stereo VU meter state.
pub struct VisualizerState {
    pub mode: VisualizerMode,
    /// Smoothed normalized magnitude per logarithmic frequency band (`0.0..=1.0`).
    pub bands: [f32; VIZ_BANDS],
    /// Falling peak-hold caps per band (`0.0..=1.0`).
    pub band_peaks: [f32; VIZ_BANDS],
    /// Zero-crossing-stabilized mono waveform samples (`-1.0..=1.0`).
    pub scope: [f32; SCOPE_POINTS],
    /// Smoothed stereo RMS and peak-hold levels (`0.0..=1.0`).
    pub vu_left: f32,
    pub vu_right: f32,
    pub vu_peak_left: f32,
    pub vu_peak_right: f32,
    /// True while recent non-silent audio is present.
    pub signal_active: bool,
    /// Smooth visual opacity/presence envelope (`0.0` = completely hidden, `1.0` = visible).
    pub presence: f32,
    /// Monotonic counter incremented on every visual change to drive `Canvas::cache_key`.
    pub frame_seq: u64,
    last_decay_tick: Option<Instant>,
    // Reusable scratch buffers for zero-allocation FFT.
    fft_re: [f32; FFT_SIZE],
    fft_im: [f32; FFT_SIZE],
    mono_buf: [f32; FFT_SIZE],
}

impl Default for VisualizerState {
    fn default() -> Self {
        Self {
            mode: VisualizerMode::Bars,
            bands: [0.0; VIZ_BANDS],
            band_peaks: [0.0; VIZ_BANDS],
            scope: [0.0; SCOPE_POINTS],
            vu_left: 0.0,
            vu_right: 0.0,
            vu_peak_left: 0.0,
            vu_peak_right: 0.0,
            signal_active: false,
            presence: 0.0,
            frame_seq: 0,
            last_decay_tick: None,
            fft_re: [0.0; FFT_SIZE],
            fft_im: [0.0; FFT_SIZE],
            mono_buf: [0.0; FFT_SIZE],
        }
    }
}

impl VisualizerState {
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.presence > 0.01
    }

    /// True only while fading out to zero after silence/stop (zero VSync wakeups once settled!).
    #[must_use]
    pub fn is_decaying(&self) -> bool {
        !self.signal_active && self.presence > 0.01
    }

    /// Immediately resets all visual state (used when user disables the visualizer).
    pub fn reset_immediate(&mut self) {
        self.bands = [0.0; VIZ_BANDS];
        self.band_peaks = [0.0; VIZ_BANDS];
        self.scope = [0.0; SCOPE_POINTS];
        self.vu_left = 0.0;
        self.vu_right = 0.0;
        self.vu_peak_left = 0.0;
        self.vu_peak_right = 0.0;
        self.signal_active = false;
        self.presence = 0.0;
        self.last_decay_tick = None;
        self.frame_seq = self.frame_seq.wrapping_add(1);
    }

    /// Marks audio as stopped/silent so bars decay smoothly to zero and unmount.
    pub fn mark_silent(&mut self) {
        if self.signal_active {
            self.signal_active = false;
            self.last_decay_tick = None;
        }
    }

    /// Ingests an [`AudioCaptureFrame`] from WayWi's `Audio::capture` subscription.
    pub fn ingest_frame(&mut self, frame: &AudioCaptureFrame) {
        if frame.silent {
            self.mark_silent();
            return;
        }

        self.signal_active = true;
        self.presence = (self.presence + 0.35).min(1.0);
        self.last_decay_tick = None;

        // Smooth stereo VU meter (logarithmic perceptual curve from -54 dBFS..0 dBFS).
        let target_l = amplitude_to_meter(frame.rms_left.max(frame.peak_left * 0.55));
        let target_r = amplitude_to_meter(frame.rms_right.max(frame.peak_right * 0.55));
        let peak_l = amplitude_to_meter(frame.peak_left);
        let peak_r = amplitude_to_meter(frame.peak_right);

        self.vu_left = smooth_level(self.vu_left, target_l, 0.65, 0.22);
        self.vu_right = smooth_level(self.vu_right, target_r, 0.65, 0.22);
        self.vu_peak_left = if peak_l >= self.vu_peak_left {
            peak_l
        } else {
            (self.vu_peak_left - 0.025).max(self.vu_left)
        };
        self.vu_peak_right = if peak_r >= self.vu_peak_right {
            peak_r
        } else {
            (self.vu_peak_right - 0.025).max(self.vu_right)
        };

        if !frame.pcm.is_empty() && self.mode.needs_pcm() {
            let channels = usize::from(frame.channels.max(1));
            let total_frames = frame.pcm.len() / channels;
            if total_frames >= 32 {
                self.extract_mono_window(&frame.pcm, channels, total_frames);
                match self.mode {
                    VisualizerMode::Oscilloscope => self.update_oscilloscope(total_frames),
                    VisualizerMode::Bars | VisualizerMode::Mirror | VisualizerMode::Wave => {
                        self.update_fft_bands(frame.sample_rate.max(8_000));
                    }
                    VisualizerMode::StereoVu => {}
                }
            }
        }

        self.frame_seq = self.frame_seq.wrapping_add(1);
    }

    /// Advances smooth fade-out decay when audio becomes silent; settles to exact 0.0.
    pub fn advance_decay(&mut self, now: Instant) {
        if !self.is_decaying() {
            self.last_decay_tick = None;
            return;
        }
        let dt = self.last_decay_tick.map_or(1.0 / 60.0, |prev| {
            now.saturating_duration_since(prev)
                .as_secs_f32()
                .clamp(1.0 / 240.0, 0.05)
        });
        self.last_decay_tick = Some(now);

        let factor = (-dt * 12.0).exp();
        for i in 0..VIZ_BANDS {
            self.bands[i] *= factor;
            self.band_peaks[i] = (self.band_peaks[i] - dt * 2.2).max(self.bands[i]);
        }
        for s in &mut self.scope {
            *s *= factor;
        }
        self.vu_left *= factor;
        self.vu_right *= factor;
        self.vu_peak_left = (self.vu_peak_left - dt * 2.4).max(self.vu_left);
        self.vu_peak_right = (self.vu_peak_right - dt * 2.4).max(self.vu_right);
        self.presence = (self.presence - dt * 3.8).max(0.0);

        if self.presence <= 0.01 {
            self.reset_immediate();
        } else {
            self.frame_seq = self.frame_seq.wrapping_add(1);
        }
    }

    fn extract_mono_window(&mut self, pcm: &[f32], channels: usize, total_frames: usize) {
        let take = total_frames.min(FFT_SIZE);
        let start_frame = total_frames - take;
        // Zero-pad leading slots if fewer than FFT_SIZE frames arrived.
        let pad = FFT_SIZE - take;
        self.mono_buf[..pad].fill(0.0);
        if channels == 1 {
            for i in 0..take {
                let s = pcm[start_frame + i];
                self.mono_buf[pad + i] = if s.is_finite() { s } else { 0.0 };
            }
        } else {
            for i in 0..take {
                let base = (start_frame + i) * channels;
                let l = pcm[base];
                let r = pcm[base + 1];
                let m = 0.5 * (l + r);
                self.mono_buf[pad + i] = if m.is_finite() { m } else { 0.0 };
            }
        }
    }

    fn update_oscilloscope(&mut self, valid_frames: usize) {
        let take = valid_frames.min(FFT_SIZE);
        let start = FFT_SIZE - take;
        let window = &self.mono_buf[start..];

        // Rising zero-crossing trigger in the first half for a rock-steady waveform trace.
        let search_end = (window.len().saturating_sub(SCOPE_POINTS * 2)).max(1);
        let mut trigger = 0usize;
        for i in 1..search_end {
            if window[i - 1] <= 0.0 && window[i] > 0.0 {
                trigger = i;
                break;
            }
        }
        let span = (window.len() - trigger).max(1);
        let step = (span as f32 / SCOPE_POINTS as f32).max(1.0);

        // Gentle auto-gain so quiet music remains legible while loud peaks stay bounded.
        let mut peak = 0.15_f32;
        for &s in window {
            peak = peak.max(s.abs());
        }
        let gain = (0.92 / peak).min(3.5);

        for (i, slot) in self.scope.iter_mut().enumerate() {
            let idx = (trigger + (i as f32 * step) as usize).min(window.len() - 1);
            let target = (window[idx] * gain).clamp(-1.0, 1.0);
            *slot = *slot * 0.25 + target * 0.75;
        }
    }

    fn update_fft_bands(&mut self, sample_rate: u32) {
        let tables = fft_tables();
        for i in 0..FFT_SIZE {
            self.fft_re[i] = self.mono_buf[i] * tables.hann[i];
            self.fft_im[i] = 0.0;
        }
        fft_radix2_512(&mut self.fft_re, &mut self.fft_im);

        let nyquist = (sample_rate as f32) * 0.5;
        let hz_per_bin = (sample_rate as f32) / (FFT_SIZE as f32);
        let min_hz = 55.0_f32.min(nyquist * 0.1);
        let max_hz = 14_500.0_f32.min(nyquist * 0.92).max(min_hz * 2.0);
        let log_ratio = (max_hz / min_hz).ln();
        let inv_scale = 2.0 / (FFT_SIZE as f32);

        for band in 0..VIZ_BANDS {
            let t0 = band as f32 / VIZ_BANDS as f32;
            let t1 = (band + 1) as f32 / VIZ_BANDS as f32;
            let f0 = min_hz * (t0 * log_ratio).exp();
            let f1 = min_hz * (t1 * log_ratio).exp();

            let bin0 = ((f0 / hz_per_bin).floor() as usize).clamp(1, FFT_SIZE / 2 - 1);
            let bin1 = ((f1 / hz_per_bin).ceil() as usize).clamp(bin0 + 1, FFT_SIZE / 2);

            let mut max_mag = 0.0_f32;
            for k in bin0..bin1 {
                let mag = (self.fft_re[k] * self.fft_re[k] + self.fft_im[k] * self.fft_im[k])
                    .sqrt()
                    * inv_scale;
                if mag > max_mag {
                    max_mag = mag;
                }
            }

            // Mild high-frequency tilt compensation so cymbals/vocals match bass energy visually.
            let tilt = 1.0 + 1.35 * t0;
            let level = amplitude_to_meter(max_mag * tilt * 2.2);

            self.bands[band] = smooth_level(self.bands[band], level, 0.68, 0.24);
            if self.bands[band] >= self.band_peaks[band] {
                self.band_peaks[band] = self.bands[band];
            } else {
                self.band_peaks[band] = (self.band_peaks[band] - 0.022).max(self.bands[band]);
            }
        }
    }
}

/// Maps linear amplitude (`0.0..=1.0+`) onto a perceptual logarithmic meter scale (`0.0..=1.0`).
#[inline]
fn amplitude_to_meter(amp: f32) -> f32 {
    if !amp.is_finite() || amp <= 1e-5 {
        return 0.0;
    }
    let db = 20.0 * amp.log10();
    // Map -56 dBFS..0 dBFS -> 0.0..1.0 with slight gentle curve.
    let norm = ((db + 56.0) / 56.0).clamp(0.0, 1.0);
    norm * norm.sqrt()
}

#[inline]
fn smooth_level(current: f32, target: f32, attack: f32, decay: f32) -> f32 {
    let alpha = if target >= current { attack } else { decay };
    (current + (target - current) * alpha).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use super::*;

    #[test]
    fn fft_detects_1khz_sine_in_matching_logarithmic_band() {
        let mut viz = VisualizerState::default();
        let sr = 48_000.0_f32;
        let freq = 1_000.0_f32;
        let mut pcm = Vec::with_capacity(FFT_SIZE * 2);
        for i in 0..FFT_SIZE {
            let s = (TAU * freq * (i as f32) / sr).sin() * 0.8;
            pcm.push(s);
            pcm.push(s);
        }
        let frame = AudioCaptureFrame {
            peak_left: 0.8,
            peak_right: 0.8,
            rms_left: 0.56,
            rms_right: 0.56,
            peak_db: -1.9,
            silent: false,
            sample_rate: 48_000,
            channels: 2,
            pcm: Arc::from(pcm),
        };
        for _ in 0..4 {
            viz.ingest_frame(&frame);
        }

        // 1 kHz sits cleanly in the middle bands (~band 7-9 of 16 spanning 55Hz..14.5kHz).
        let peak_band = viz
            .bands
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i)
            .unwrap();
        assert!(
            (6..=10).contains(&peak_band),
            "expected 1kHz peak around bands 6..=10, got band {peak_band} ({:?})",
            viz.bands
        );
        assert!(viz.bands[peak_band] > 0.5);
        assert!(viz.bands[0] < viz.bands[peak_band] * 0.35);
        assert!(viz.bands[VIZ_BANDS - 1] < viz.bands[peak_band] * 0.35);
    }

    #[test]
    fn silence_decays_visualizer_cleanly_to_zero_and_unmounts() {
        let mut viz = VisualizerState::default();
        let active = AudioCaptureFrame {
            peak_left: 0.7,
            peak_right: 0.5,
            rms_left: 0.4,
            rms_right: 0.3,
            peak_db: -3.1,
            silent: false,
            sample_rate: 48_000,
            channels: 2,
            pcm: Arc::from(vec![0.0_f32; 0]),
        };
        viz.mode = VisualizerMode::StereoVu;
        viz.ingest_frame(&active);
        assert!(viz.is_visible());
        assert!(!viz.is_decaying());

        let silent = AudioCaptureFrame {
            silent: true,
            peak_left: 0.0,
            peak_right: 0.0,
            rms_left: 0.0,
            rms_right: 0.0,
            peak_db: -96.0,
            ..active
        };
        viz.ingest_frame(&silent);
        assert!(viz.is_decaying());

        let mut t = Instant::now();
        for _ in 0..40 {
            t += Duration::from_millis(20);
            viz.advance_decay(t);
        }
        assert!(!viz.is_visible());
        assert!(!viz.is_decaying());
        assert_eq!(viz.vu_left, 0.0);
        assert_eq!(viz.vu_right, 0.0);
    }

    #[test]
    fn mode_cycles_forward_and_backward_cleanly() {
        let mut m = VisualizerMode::Bars;
        for _ in 0..VisualizerMode::ALL.len() {
            m = m.next();
        }
        assert_eq!(m, VisualizerMode::Bars);
        assert_eq!(m.prev(), VisualizerMode::Wave);
        assert!(!VisualizerMode::StereoVu.needs_pcm());
        assert!(VisualizerMode::Oscilloscope.needs_pcm());
    }
}
