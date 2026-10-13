mod commands;
mod crypto;
mod error;
mod models;
mod parser;
mod store;
mod vault;

use std::sync::atomic::Ordering;

use tauri::Manager;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let store = store::Store::open(&dir.join("keybox.db"))?;
            app.manage(AppState::new(vault::Vault::new(store)?));

            if let Some(window) = app.get_webview_window("main") {
                let ok = apply_window_effect(&window);
                app.state::<AppState>().window_effect.store(ok, Ordering::Relaxed);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_info,
            commands::get_settings,
            commands::update_settings,
            commands::list_entries,
            commands::get_entry,
            commands::create_entry,
            commands::create_entries,
            commands::update_entry,
            commands::delete_entry,
            commands::parse_text,
            commands::copy_field,
            commands::copy_text,
            commands::inspect_import_file,
            commands::import_from_file,
            commands::export_to_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[allow(unused_variables)]
fn apply_window_effect(window: &tauri::WebviewWindow) -> bool {
    #[cfg(target_os = "macos")]
    {
        use window_vibrancy::{NSVisualEffectMaterial, NSVisualEffectState, apply_vibrancy};
        return apply_vibrancy(
            window,
            NSVisualEffectMaterial::Sidebar,
            Some(NSVisualEffectState::FollowsWindowActiveState),
            None,
        )
        .is_ok();
    }
    #[cfg(target_os = "windows")]
    {
        use window_vibrancy::{apply_acrylic, apply_mica};
        return apply_mica(window, None).is_ok() || apply_acrylic(window, Some((245, 245, 247, 200))).is_ok();
    }
    #[allow(unreachable_code)]
    false
}
