/// store.rs — app data directory layout and JSON persistence for the library.
use std::path::{Path, PathBuf};
use std::fs;
use serde_json;
use crate::types::{ModelInfo, IrInfo, Preset};

pub struct AppDirs {
    pub root:    PathBuf,
    pub models:  PathBuf,
    pub irs:     PathBuf,
    pub tmp:     PathBuf,
}

impl AppDirs {
    pub fn new(app_data_dir: &Path) -> std::io::Result<Self> {
        let root   = app_data_dir.to_path_buf();
        let models = root.join("models");
        let irs    = root.join("irs");
        let tmp    = root.join("tmp");
        fs::create_dir_all(&models)?;
        fs::create_dir_all(&irs)?;
        fs::create_dir_all(&tmp)?;
        Ok(Self { root, models, irs, tmp })
    }

    pub fn presets_json(&self) -> PathBuf { self.root.join("presets.json") }
    pub fn model_meta(&self)  -> PathBuf { self.root.join("models.json") }
    pub fn ir_meta(&self)     -> PathBuf { self.root.join("irs.json") }
    pub fn tokens_json(&self) -> PathBuf { self.root.join("tone3000_tokens.json") }

    pub fn model_path(&self, id: &str, name: &str) -> PathBuf {
        self.models.join(format!("{id}_{name}.namb"))
    }
    pub fn ir_path(&self, id: &str, name: &str) -> PathBuf {
        self.irs.join(format!("{id}_{name}.wav"))
    }
}

pub fn load_models(dirs: &AppDirs) -> Vec<ModelInfo> {
    read_json(&dirs.model_meta()).unwrap_or_default()
}

pub fn save_models(dirs: &AppDirs, models: &Vec<ModelInfo>) {
    write_json(&dirs.model_meta(), models);
}

pub fn load_irs(dirs: &AppDirs) -> Vec<IrInfo> {
    read_json(&dirs.ir_meta()).unwrap_or_default()
}

pub fn save_irs(dirs: &AppDirs, irs: &Vec<IrInfo>) {
    write_json(&dirs.ir_meta(), irs);
}

pub fn load_presets(dirs: &AppDirs) -> Vec<Preset> {
    read_json(&dirs.presets_json()).unwrap_or_default()
}

pub fn save_presets(dirs: &AppDirs, presets: &Vec<Preset>) {
    write_json(&dirs.presets_json(), presets);
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let data = fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) {
    if let Ok(json) = serde_json::to_string_pretty(value) {
        let _ = fs::write(path, json);
    }
}
