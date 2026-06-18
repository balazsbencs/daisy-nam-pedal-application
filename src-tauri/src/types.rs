use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id:                String,
    pub name:              String,
    pub size_bytes:        u64,
    pub imported_at:       String,
    pub tone3000_id:       Option<String>,
    #[serde(default)]
    pub tone3000_model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrInfo {
    pub id:          String,
    pub name:        String,
    pub tap_count:   u32,
    pub sample_rate: u32,
    pub trimmed:     bool,   // true = original was > 512 taps and was trimmed
    pub size_bytes:  u64,
    pub imported_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub id:            String,
    pub name:          String,
    pub model_id:      Option<String>,
    pub ir_id:         Option<String>,
    pub input_gain:    f32,   // 0.0..2.0
    pub output_volume: f32,   // 0.0..1.0
    pub bypass:        bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageEntry {
    pub entry_type: String, // "model" | "ir" | "preset"
    pub name:       String,
    pub size_bytes: u32,
    pub offset:     u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSummary {
    pub entries:          Vec<ImageEntry>,
    pub total_bytes:      u32,
    pub partition_bytes:  u32, // 6 MB
    pub free_bytes:       u32,
    pub image_path:       String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceStatus {
    pub connected: bool,
}
