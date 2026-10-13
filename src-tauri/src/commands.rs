use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

use crate::error::{AppError, AppResult};
use crate::models::{ConflictMode, Entry, EntryData, EntryView, ImportReport, Settings};
use crate::parser::{self, ParsedEntry};
use crate::vault::Vault;

const MAX_IMPORT_BYTES: u64 = 50 * 1024 * 1024;

pub struct AppState {
    pub vault: Mutex<Vault>,
    pub clipboard_gen: AtomicU64,
    pub window_effect: AtomicBool,
}

impl AppState {
    pub fn new(vault: Vault) -> Self {
        Self { vault: Mutex::new(vault), clipboard_gen: AtomicU64::new(0), window_effect: AtomicBool::new(false) }
    }

    fn with<T>(&self, f: impl FnOnce(&mut Vault) -> AppResult<T>) -> AppResult<T> {
        let mut v = self.vault.lock().unwrap();
        f(&mut v)
    }
}

#[derive(Serialize)]
pub struct AppInfo {
    platform: &'static str,
    window_effect: bool,
    version: String,
}

#[tauri::command]
pub fn app_info(app: AppHandle, state: State<AppState>) -> AppInfo {
    AppInfo {
        platform: std::env::consts::OS,
        window_effect: state.window_effect.load(Ordering::Relaxed),
        version: app.package_info().version.to_string(),
    }
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> AppResult<Settings> {
    state.with(|v| v.settings())
}

#[tauri::command]
pub fn update_settings(state: State<AppState>, settings: Settings) -> AppResult<Settings> {
    state.with(|v| {
        let mut s = settings;
        s.clipboard_clear_seconds = s.clipboard_clear_seconds.min(600);
        v.save_settings(&s)?;
        Ok(s)
    })
}

#[tauri::command]
pub fn list_entries(state: State<AppState>) -> AppResult<Vec<EntryView>> {
    state.with(|v| v.list())
}

#[tauri::command]
pub fn get_entry(state: State<AppState>, id: String) -> AppResult<Entry> {
    state.with(|v| v.get(&id).cloned())
}

#[tauri::command]
pub fn create_entry(state: State<AppState>, data: EntryData) -> AppResult<EntryView> {
    state.with(|v| v.create(data))
}

#[tauri::command]
pub fn create_entries(state: State<AppState>, items: Vec<EntryData>) -> AppResult<Vec<EntryView>> {
    state.with(|v| items.into_iter().map(|d| v.create(d)).collect())
}

#[tauri::command]
pub fn update_entry(state: State<AppState>, id: String, data: EntryData) -> AppResult<EntryView> {
    state.with(|v| v.update(&id, data))
}

#[tauri::command]
pub fn delete_entry(state: State<AppState>, id: String) -> AppResult<()> {
    state.with(|v| v.delete(&id))
}

#[tauri::command]
pub fn parse_text(text: String) -> Vec<ParsedEntry> {
    parser::parse(&text)
}

/// Copies a field of an entry. Passwords are cleared from the clipboard after
/// the configured delay (only if the clipboard still holds the same value).
#[tauri::command]
pub fn copy_field(app: AppHandle, state: State<AppState>, id: String, field: String) -> AppResult<()> {
    let (value, clear_after) = state.with(|v| {
        let e = v.get(&id)?;
        let value = match field.as_str() {
            "password" => e.data.password.clone(),
            "username" => e.data.username.clone(),
            "host" => e.data.host.clone(),
            "port" => e.data.port.map(|p| p.to_string()).unwrap_or_default(),
            _ => return Err(AppError::Invalid("未知字段".into())),
        };
        let secs = if field == "password" { v.settings()?.clipboard_clear_seconds } else { 0 };
        Ok((value, secs))
    })?;
    copy_and_schedule(&app, &state, value, clear_after)
}

#[tauri::command]
pub fn copy_text(app: AppHandle, state: State<AppState>, text: String, sensitive: bool) -> AppResult<()> {
    let secs = if sensitive { state.with(|v| v.settings())?.clipboard_clear_seconds } else { 0 };
    copy_and_schedule(&app, &state, text, secs)
}

fn copy_and_schedule(app: &AppHandle, state: &AppState, value: String, clear_after: u32) -> AppResult<()> {
    app.clipboard()
        .write_text(value.clone())
        .map_err(|e| AppError::Invalid(format!("写入剪贴板失败：{e}")))?;
    let generation = state.clipboard_gen.fetch_add(1, Ordering::SeqCst) + 1;
    if clear_after > 0 {
        let app = app.clone();
        let value = zeroize::Zeroizing::new(value);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(clear_after as u64));
            let st = app.state::<AppState>();
            if st.clipboard_gen.load(Ordering::SeqCst) != generation {
                return;
            }
            if app.clipboard().read_text().ok().as_deref() == Some(value.as_str()) {
                let _ = app.clipboard().write_text(String::new());
            }
        });
    }
    Ok(())
}

#[derive(Serialize)]
pub struct ImportInfo {
    encrypted: bool,
}

fn read_import_file(path: &str) -> AppResult<String> {
    let meta = std::fs::metadata(path)?;
    if meta.len() > MAX_IMPORT_BYTES {
        return Err(AppError::Invalid("文件过大（超过 50MB）".into()));
    }
    Ok(std::fs::read_to_string(path)?)
}

#[tauri::command]
pub fn inspect_import_file(path: String) -> AppResult<ImportInfo> {
    let json = read_import_file(&path)?;
    serde_json::from_str::<serde_json::Value>(&json)?;
    Ok(ImportInfo { encrypted: Vault::is_encrypted_export(&json) })
}

#[tauri::command]
pub async fn import_from_file(
    state: State<'_, AppState>,
    path: String,
    password: Option<String>,
    mode: ConflictMode,
) -> AppResult<ImportReport> {
    let json = read_import_file(&path)?;
    state.with(|v| v.import_json(&json, password.as_deref(), mode))
}

#[tauri::command]
pub async fn export_to_file(state: State<'_, AppState>, path: String, password: Option<String>) -> AppResult<usize> {
    let (json, count) = state.with(|v| Ok((v.export_json(password.as_deref())?, v.list()?.len())))?;
    std::fs::write(&path, json)?;
    Ok(count)
}
