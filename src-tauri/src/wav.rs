/// wav.rs — convert a WAV file to float32 mono taps for the QSPI IR format.
/// Warns and trims if tap count > MAX_TAPS (matches firmware FirConvolver::kMaxTaps = 512).
use hound::{WavReader, SampleFormat};
use std::path::Path;

pub const MAX_TAPS: usize = 512;

pub struct IrTaps {
    pub taps:        Vec<f32>,
    pub sample_rate: u32,
    pub trimmed:     bool,   // true = original had more than MAX_TAPS samples
}

pub fn load_ir(path: &Path) -> Result<IrTaps, String> {
    let mut reader = WavReader::open(path)
        .map_err(|e| format!("Failed to open WAV: {e}"))?;

    let spec = reader.spec();
    let channels = spec.channels as usize;

    // Read all samples, mix down to mono.
    let mut mono: Vec<f32> = Vec::new();

    match (spec.sample_format, spec.bits_per_sample) {
        (SampleFormat::Float, 32) => {
            let samples: Vec<f32> = reader
                .samples::<f32>()
                .map(|s| s.map_err(|e| format!("Read error: {e}")))
                .collect::<Result<_, _>>()?;
            for frame in samples.chunks(channels) {
                let sum: f32 = frame.iter().sum();
                mono.push(sum / channels as f32);
            }
        }
        (SampleFormat::Int, 16) => {
            let samples: Vec<i16> = reader
                .samples::<i16>()
                .map(|s| s.map_err(|e| format!("Read error: {e}")))
                .collect::<Result<_, _>>()?;
            for frame in samples.chunks(channels) {
                let sum: f32 = frame.iter().map(|&s| s as f32 / 32768.0).sum();
                mono.push(sum / channels as f32);
            }
        }
        (SampleFormat::Int, 24) => {
            let samples: Vec<i32> = reader
                .samples::<i32>()
                .map(|s| s.map_err(|e| format!("Read error: {e}")))
                .collect::<Result<_, _>>()?;
            for frame in samples.chunks(channels) {
                let sum: f32 = frame.iter().map(|&s| s as f32 / 8_388_608.0).sum();
                mono.push(sum / channels as f32);
            }
        }
        (fmt, bits) => {
            return Err(format!(
                "Unsupported WAV format: {fmt:?} {bits}-bit. Use 16-bit, 24-bit int, or 32-bit float."
            ));
        }
    }

    let trimmed = mono.len() > MAX_TAPS;
    if trimmed {
        mono.truncate(MAX_TAPS);
    }

    Ok(IrTaps {
        taps: mono,
        sample_rate: spec.sample_rate,
        trimmed,
    })
}

/// Serialize float32 taps to raw little-endian bytes for the QSPI blob.
pub fn taps_to_bytes(taps: &[f32]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(taps.len() * 4);
    for &t in taps {
        buf.extend_from_slice(&t.to_le_bytes());
    }
    buf
}
