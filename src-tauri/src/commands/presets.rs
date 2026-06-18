use tauri::{AppHandle, Manager};
use uuid::Uuid;
use crate::types::Preset;
use crate::store::{AppDirs, load_presets, save_presets};

fn dirs(app: &AppHandle) -> Result<AppDirs, String> {
    let path = app.path().app_data_dir()
        .map_err(|e| format!("No app data dir: {e}"))?;
    AppDirs::new(&path).map_err(|e| format!("Failed to create dirs: {e}"))
}

#[tauri::command]
pub fn list_presets(app: AppHandle) -> Result<Vec<Preset>, String> {
    Ok(load_presets(&dirs(&app)?))
}

#[tauri::command]
pub fn save_preset(app: AppHandle, preset: Preset) -> Result<Preset, String> {
    let dirs = dirs(&app)?;
    let mut presets = load_presets(&dirs);

    let is_new = preset.id.is_empty();
    let mut p = preset;
    if is_new {
        p.id = Uuid::new_v4().to_string();
        presets.push(p.clone());
    } else if let Some(existing) = presets.iter_mut().find(|x| x.id == p.id) {
        *existing = p.clone();
    } else {
        // ID provided but not found — treat as new.
        presets.push(p.clone());
    }

    save_presets(&dirs, &presets);
    Ok(p)
}

#[tauri::command]
pub fn delete_preset(app: AppHandle, id: String) -> Result<(), String> {
    let dirs = dirs(&app)?;
    let mut presets = load_presets(&dirs);
    presets.retain(|p| p.id != id);
    save_presets(&dirs, &presets);
    Ok(())
}

/// Persist a reordered preset list (drag-and-drop result from the frontend).
#[tauri::command]
pub fn reorder_presets(app: AppHandle, ordered_ids: Vec<String>) -> Result<Vec<Preset>, String> {
    let dirs = dirs(&app)?;
    let mut presets = load_presets(&dirs);

    // Re-sort by the order of ordered_ids; any IDs not in the list go to the end.
    presets.sort_by_key(|p| {
        ordered_ids.iter().position(|id| id == &p.id).unwrap_or(usize::MAX)
    });

    save_presets(&dirs, &presets);
    Ok(presets)
}
