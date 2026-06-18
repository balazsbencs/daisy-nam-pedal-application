use tauri::{AppHandle, Manager, Emitter};
use tauri_plugin_shell::ShellExt;
use std::fs;
use crate::types::{ImageSummary, ImageEntry};
use crate::store::{AppDirs, load_models, load_irs, load_presets};
use crate::image_builder::{self, Blob, pack_preset_blob};
use crate::wav;

fn dirs(app: &AppHandle) -> Result<AppDirs, String> {
    let path = app.path().app_data_dir()
        .map_err(|e| format!("No app data dir: {e}"))?;
    AppDirs::new(&path).map_err(|e| format!("Failed to create dirs: {e}"))
}

/// Detect whether a DFU device (0483:df11) is connected by calling bundled dfu-util -l.
#[tauri::command]
pub async fn detect_device(app: AppHandle) -> Result<bool, String> {
    let output = run_dfu_util(&app, &["-l"]).await?;
    Ok(output.contains("0483:df11"))
}

/// Build the QSPI data image from the current library. Returns a summary.
/// Writes the image to the app tmp directory.
#[tauri::command]
pub async fn build_image(app: AppHandle) -> Result<ImageSummary, String> {
    let dirs   = dirs(&app)?;
    let models = load_models(&dirs);
    let irs    = load_irs(&dirs);
    let presets = load_presets(&dirs);

    let mut blobs: Vec<Blob> = Vec::new();

    // --- Models (type 0) ---
    for m in &models {
        let path = dirs.model_path(&m.id, &m.name);
        let data = fs::read(&path)
            .map_err(|e| format!("Cannot read model '{}': {e}", m.name))?;
        blobs.push(Blob {
            entry_type: 0,
            name:       m.name.clone(),
            data,
            samplerate: 0,
        });
    }

    // --- IRs (type 1) — re-read WAV and convert to float32 taps ---
    for r in &irs {
        let path   = dirs.ir_path(&r.id, &r.name);
        let ir     = wav::load_ir(&path)?;
        let data   = wav::taps_to_bytes(&ir.taps);
        blobs.push(Blob {
            entry_type: 1,
            name:       r.name.clone(),
            data,
            samplerate: ir.sample_rate,
        });
    }

    // --- Presets (type 2) — resolve model/IR names and pack ---
    for p in &presets {
        let model_name = p.model_id.as_ref()
            .and_then(|id| models.iter().find(|m| &m.id == id))
            .map(|m| m.name.as_str())
            .unwrap_or("");
        let ir_name = p.ir_id.as_ref()
            .and_then(|id| irs.iter().find(|r| &r.id == id))
            .map(|r| r.name.as_str())
            .unwrap_or("");
        let data = pack_preset_blob(model_name, ir_name,
                                    p.input_gain, p.output_volume, p.bypass);
        blobs.push(Blob {
            entry_type: 2,
            name:       p.name.clone(),
            data,
            samplerate: 0,
        });
    }

    let built      = image_builder::build(&blobs);
    let image_path = dirs.tmp.join("data_image.bin");
    fs::write(&image_path, &built.data)
        .map_err(|e| format!("Failed to write image: {e}"))?;

    let total_bytes = built.data.len() as u32;
    let partition   = image_builder::partition_size();

    let entries = built.entries.iter().map(|(t, name, offset, length)| {
        ImageEntry {
            entry_type: match t { 0 => "model", 1 => "ir", _ => "preset" }.into(),
            name:       name.clone(),
            size_bytes: *length,
            offset:     *offset,
        }
    }).collect();

    Ok(ImageSummary {
        entries,
        total_bytes,
        partition_bytes: partition,
        free_bytes: partition.saturating_sub(total_bytes),
        image_path: image_path.to_string_lossy().into(),
    })
}

/// Flash a previously built image to the device.
/// Emits "flash-progress" events with { percent: u8, message: String } during flashing.
#[tauri::command]
pub async fn flash_image(app: AppHandle, image_path: String) -> Result<(), String> {
    // Verify the image exists.
    if !std::path::Path::new(&image_path).exists() {
        return Err("Image file not found — build first.".into());
    }

    app.emit("flash-progress", serde_json::json!({ "percent": 0, "message": "Starting DFU..." }))
        .ok();

    let output = run_dfu_util(&app, &[
        "-a", "0",
        "-s", "0x90200000:leave",
        "-D", &image_path,
        "-d", ",0483:df11",
    ]).await?;

    if output.contains("Error") || output.contains("error") {
        return Err(format!("dfu-util reported an error:\n{output}"));
    }

    app.emit("flash-progress", serde_json::json!({ "percent": 100, "message": "Done!" })).ok();
    Ok(())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

async fn run_dfu_util(app: &AppHandle, args: &[&str]) -> Result<String, String> {
    // Tauri sidecar: binary must be registered in tauri.conf.json under
    // "bundle.externalBin". Naming: "dfu-util" -> Tauri appends platform triple.
    let sidecar = app.shell()
        .sidecar("dfu-util")
        .map_err(|e| format!("dfu-util sidecar not found: {e}"))?
        .args(args);

    let output = sidecar.output().await
        .map_err(|e| format!("Failed to run dfu-util: {e}"))?;

    // dfu-util writes progress to stderr; combine both.
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    Ok(format!("{stdout}{stderr}"))
}
