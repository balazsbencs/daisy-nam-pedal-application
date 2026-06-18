mod types;
mod store;
mod image_builder;
mod wav;
mod commands;

use commands::{library, presets, flash, discover};
use commands::discover::DiscoverState;
use tauri_plugin_deep_link::DeepLinkExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_deep_link::init())
        .manage(DiscoverState(std::sync::Mutex::new(None)))
        .setup(|app| {
            // Register the daisy:// scheme with the OS so it opens this app.
            // Required in dev mode; bundled builds use Info.plist / registry.
            if let Err(e) = app.deep_link().register("daisy") {
                eprintln!("deep-link register failed: {e}");
            }

            let app_handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                let app_handle = app_handle.clone();
                for url in event.urls() {
                    let url_str = url.to_string();
                    if url_str.starts_with("daisy://oauth-callback") {
                        let app_handle = app_handle.clone();
                        tauri::async_runtime::spawn(async move {
                            use tauri::Emitter;
                            match commands::discover::handle_oauth_callback(&app_handle, &url_str).await {
                                Ok(tokens) => {
                                    let _ = app_handle.emit("tone3000-auth-result", serde_json::json!({
                                        "success": true,
                                        "username": tokens.username,
                                        "avatar_url": tokens.avatar_url,
                                    }));
                                }
                                Err(e) => {
                                    let _ = app_handle.emit("tone3000-auth-result", serde_json::json!({
                                        "success": false,
                                        "error": e,
                                    }));
                                }
                            }
                        });
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Library
            library::list_models,
            library::import_model,
            library::rename_model,
            library::delete_model,
            library::import_model_nam,
            library::list_irs,
            library::import_ir,
            library::delete_ir,
            // Presets
            presets::list_presets,
            presets::save_preset,
            presets::delete_preset,
            presets::reorder_presets,
            // Flash
            flash::detect_device,
            flash::build_image,
            flash::flash_image,
            // Discover
            discover::tone3000_auth_start,
            discover::tone3000_auth_cancel,
            discover::tone3000_check_auth,
            discover::tone3000_sign_out,
            discover::tone3000_search,
            discover::tone3000_list_models,
            discover::download_tone,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
