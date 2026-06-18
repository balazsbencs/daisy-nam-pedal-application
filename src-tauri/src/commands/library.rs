use tauri::{AppHandle, Manager};
use uuid::Uuid;
use std::fs;
use chrono::Utc;
use crate::types::{ModelInfo, IrInfo};
use crate::store::{AppDirs, load_models, save_models, load_irs, save_irs};
use crate::wav;

fn dirs(app: &AppHandle) -> Result<AppDirs, String> {
    let path = app.path().app_data_dir()
        .map_err(|e| format!("No app data dir: {e}"))?;
    AppDirs::new(&path).map_err(|e| format!("Failed to create dirs: {e}"))
}

// ---- Models -----------------------------------------------------------------

#[tauri::command]
pub fn list_models(app: AppHandle) -> Result<Vec<ModelInfo>, String> {
    Ok(load_models(&dirs(&app)?))
}

#[tauri::command]
pub fn import_model(app: AppHandle, src_path: String) -> Result<ModelInfo, String> {
    let dirs = dirs(&app)?;
    let src = std::path::Path::new(&src_path);

    if src.extension().and_then(|e| e.to_str()) != Some("namb") {
        return Err("File must have a .namb extension".into());
    }

    let name = src.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("model")
        .to_string();

    let id   = Uuid::new_v4().to_string();
    let dest = dirs.model_path(&id, &name);
    fs::copy(src, &dest).map_err(|e| format!("Copy failed: {e}"))?;

    let size_bytes = fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    let info = ModelInfo {
        id,
        name,
        size_bytes,
        imported_at: Utc::now().to_rfc3339(),
        tone3000_id: None,
        tone3000_model_id: None,
    };

    let mut models = load_models(&dirs);
    models.push(info.clone());
    save_models(&dirs, &models);
    Ok(info)
}

#[tauri::command]
pub fn rename_model(app: AppHandle, id: String, new_name: String) -> Result<ModelInfo, String> {
    let dirs = dirs(&app)?;
    let mut models = load_models(&dirs);

    let pos = models.iter().position(|m| m.id == id)
        .ok_or("Model not found")?;

    let old_path = dirs.model_path(&models[pos].id, &models[pos].name);
    let new_path = dirs.model_path(&id, &new_name);

    if old_path != new_path {
        fs::rename(&old_path, &new_path)
            .map_err(|e| format!("Failed to rename file: {e}"))?;
    }

    models[pos].name = new_name;
    save_models(&dirs, &models);
    Ok(models[pos].clone())
}

#[tauri::command]
pub fn delete_model(app: AppHandle, id: String) -> Result<(), String> {
    let dirs = dirs(&app)?;
    let mut models = load_models(&dirs);

    if let Some(pos) = models.iter().position(|m| m.id == id) {
        let m    = &models[pos];
        let path = dirs.model_path(&m.id, &m.name);
        let _    = fs::remove_file(path);
        models.remove(pos);
        save_models(&dirs, &models);
    }
    Ok(())
}

// ---- IRs --------------------------------------------------------------------

#[tauri::command]
pub fn list_irs(app: AppHandle) -> Result<Vec<IrInfo>, String> {
    Ok(load_irs(&dirs(&app)?))
}

#[tauri::command]
pub fn import_ir(app: AppHandle, src_path: String) -> Result<IrInfo, String> {
    let dirs = dirs(&app)?;
    let src  = std::path::Path::new(&src_path);

    if src.extension().and_then(|e| e.to_str()) != Some("wav") {
        return Err("File must have a .wav extension".into());
    }

    let name = src.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("ir")
        .to_string();

    // Validate + measure WAV (trimming happens at image-build time).
    let ir = wav::load_ir(src)?;

    let id   = Uuid::new_v4().to_string();
    let dest = dirs.ir_path(&id, &name);
    fs::copy(src, &dest).map_err(|e| format!("Copy failed: {e}"))?;

    let size_bytes = fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    let info = IrInfo {
        id,
        name,
        tap_count:   ir.taps.len() as u32,
        sample_rate: ir.sample_rate,
        trimmed:     ir.trimmed,
        size_bytes,
        imported_at: Utc::now().to_rfc3339(),
    };

    let mut irs = load_irs(&dirs);
    irs.push(info.clone());
    save_irs(&dirs, &irs);
    Ok(info)
}

#[tauri::command]
pub fn delete_ir(app: AppHandle, id: String) -> Result<(), String> {
    let dirs = dirs(&app)?;
    let mut irs = load_irs(&dirs);

    if let Some(pos) = irs.iter().position(|r| r.id == id) {
        let r    = &irs[pos];
        let path = dirs.ir_path(&r.id, &r.name);
        let _    = fs::remove_file(path);
        irs.remove(pos);
        save_irs(&dirs, &irs);
    }
    Ok(())
}

// ---- .nam import (with conversion) ------------------------------------------

pub fn validate_nam_path(path: &std::path::Path) -> Result<String, String> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("nam") => Ok(
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("model")
                .to_string()
        ),
        Some(ext) => Err(format!("Expected a .nam file, got .{ext}")),
        None => Err("File has no extension".into()),
    }
}

#[tauri::command]
pub async fn import_model_nam(
    app: AppHandle,
    src_path: String,
) -> Result<ModelInfo, String> {
    use tauri_plugin_shell::ShellExt;

    let dirs = dirs(&app)?;
    let src  = std::path::Path::new(&src_path);
    let name = validate_nam_path(src)?;

    let run_id   = uuid::Uuid::new_v4().to_string();
    let tmp_nam  = dirs.tmp.join(format!("{run_id}_input.nam"));
    let tmp_namb = dirs.tmp.join(format!("{run_id}_output.namb"));

    fs::copy(src, &tmp_nam).map_err(|e| format!("Cannot read source: {e}"))?;

    let output = app.shell()
        .sidecar("nam2namb")
        .map_err(|e| format!("nam2namb not found: {e}"))?
        .args([
            "--slim", "0.5",
            tmp_nam.to_str().unwrap(),
            tmp_namb.to_str().unwrap(),
        ])
        .output()
        .await
        .map_err(|e| format!("Failed to run nam2namb: {e}"))?;

    let _ = fs::remove_file(&tmp_nam);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(format!("Conversion failed: {stderr}"));
    }

    let id   = uuid::Uuid::new_v4().to_string();
    let dest = dirs.model_path(&id, &name);
    fs::rename(&tmp_namb, &dest)
        .or_else(|_| fs::copy(&tmp_namb, &dest).map(|_| ())
            .and_then(|_| fs::remove_file(&tmp_namb)))
        .map_err(|e| format!("Failed to move converted file: {e}"))?;

    let size_bytes = fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    let info = ModelInfo {
        id,
        name,
        size_bytes,
        imported_at: Utc::now().to_rfc3339(),
        tone3000_id: None,
        tone3000_model_id: None,
    };

    let mut models = load_models(&dirs);
    models.push(info.clone());
    save_models(&dirs, &models);
    Ok(info)
}

// ---- Tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn validate_nam_rejects_namb() {
        assert!(validate_nam_path(Path::new("model.namb")).is_err());
    }

    #[test]
    fn validate_nam_rejects_no_extension() {
        assert!(validate_nam_path(Path::new("modelfile")).is_err());
    }

    #[test]
    fn validate_nam_accepts_and_returns_stem() {
        let name = validate_nam_path(Path::new("/some/path/my-model.nam")).unwrap();
        assert_eq!(name, "my-model");
    }
}
