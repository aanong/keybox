use base64::{Engine, engine::general_purpose::STANDARD as B64};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

use crate::crypto::{self, KdfParams};
use crate::error::{AppError, AppResult};
use crate::models::{ConflictMode, Entry, EntryData, EntryView, ImportReport, Settings, now_ts};
use crate::store::{RawEntry, Store};

const META_SETTINGS: &str = "settings";
const EXPORT_FORMAT: &str = "keybox-export";
const EXPORT_AAD: &[u8] = b"keybox-export-v1";
pub const MIN_PASSWORD_LEN: usize = 6;

pub struct Vault {
    store: Store,
    entries: Vec<Entry>,
}

#[derive(Serialize, Deserialize)]
struct ExportFile {
    format: String,
    version: u32,
    encrypted: bool,
    exported_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    kdf: Option<KdfParams>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cipher: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    entries: Option<Vec<ExportEntry>>,
}

#[derive(Serialize, Deserialize, Clone)]
struct ExportEntry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(flatten)]
    data: EntryData,
    #[serde(default)]
    created_at: Option<i64>,
    #[serde(default)]
    updated_at: Option<i64>,
}

impl Vault {
    pub fn new(store: Store) -> AppResult<Self> {
        let mut entries = Vec::new();
        for raw in store.list()? {
            let data: EntryData = serde_json::from_slice(&raw.blob)?;
            entries.push(Entry { id: raw.id, data, created_at: raw.created_at, updated_at: raw.updated_at });
        }
        Ok(Self { store, entries })
    }

    fn check_password_strength(pw: &str) -> AppResult<()> {
        if pw.chars().count() < MIN_PASSWORD_LEN {
            return Err(AppError::Invalid(format!("导出密码至少需要 {MIN_PASSWORD_LEN} 个字符")));
        }
        Ok(())
    }

    pub fn settings(&self) -> AppResult<Settings> {
        Ok(match self.store.get_meta(META_SETTINGS)? {
            Some(raw) => serde_json::from_slice(&raw).unwrap_or_default(),
            None => Settings::default(),
        })
    }

    pub fn save_settings(&self, s: &Settings) -> AppResult<()> {
        self.store.set_meta(META_SETTINGS, &serde_json::to_vec(s)?)
    }

    // ---------- entries ----------

    fn to_raw(e: &Entry) -> AppResult<RawEntry> {
        Ok(RawEntry {
            id: e.id.clone(),
            blob: serde_json::to_vec(&e.data)?,
            created_at: e.created_at,
            updated_at: e.updated_at,
        })
    }

    pub fn list(&self) -> AppResult<Vec<EntryView>> {
        let mut v: Vec<EntryView> = self.entries.iter().map(EntryView::from).collect();
        v.sort_by_key(|e| std::cmp::Reverse(e.updated_at));
        Ok(v)
    }

    pub fn get(&self, id: &str) -> AppResult<&Entry> {
        self.entries.iter().find(|e| e.id == id).ok_or(AppError::NotFound)
    }

    pub fn create(&mut self, mut data: EntryData) -> AppResult<EntryView> {
        data.normalize();
        let now = now_ts();
        let e = Entry { id: uuid::Uuid::new_v4().to_string(), data, created_at: now, updated_at: now };
        self.store.upsert(&Self::to_raw(&e)?)?;
        let view = EntryView::from(&e);
        self.entries.push(e);
        Ok(view)
    }

    pub fn update(&mut self, id: &str, mut data: EntryData) -> AppResult<EntryView> {
        data.normalize();
        let idx = self.entries.iter().position(|e| e.id == id).ok_or(AppError::NotFound)?;
        let mut e = self.entries[idx].clone();
        e.data = data;
        e.updated_at = now_ts();
        self.store.upsert(&Self::to_raw(&e)?)?;
        let view = EntryView::from(&e);
        self.entries[idx] = e;
        Ok(view)
    }

    pub fn delete(&mut self, id: &str) -> AppResult<()> {
        if self.store.delete(id)? == 0 {
            return Err(AppError::NotFound);
        }
        self.entries.retain(|e| e.id != id);
        Ok(())
    }

    // ---------- import / export ----------

    pub fn export_json(&self, password: Option<&str>) -> AppResult<String> {
        let entries: Vec<ExportEntry> = self
            .entries
            .iter()
            .map(|e| ExportEntry {
                id: Some(e.id.clone()),
                data: e.data.clone(),
                created_at: Some(e.created_at),
                updated_at: Some(e.updated_at),
            })
            .collect();
        let mut file = ExportFile {
            format: EXPORT_FORMAT.into(),
            version: 1,
            encrypted: false,
            exported_at: now_ts(),
            kdf: None,
            cipher: None,
            data: None,
            entries: None,
        };
        match password.filter(|p| !p.is_empty()) {
            Some(pw) => {
                Self::check_password_strength(pw)?;
                let kdf = KdfParams::new_random();
                let key = kdf.derive(pw)?;
                let plain = Zeroizing::new(serde_json::to_vec(&entries)?);
                file.encrypted = true;
                file.data = Some(B64.encode(crypto::encrypt(&key, &plain, EXPORT_AAD)?));
                file.cipher = Some("xchacha20poly1305".into());
                file.kdf = Some(kdf);
            }
            None => file.entries = Some(entries),
        }
        Ok(serde_json::to_string_pretty(&file)?)
    }

    /// Returns true if the given JSON is an encrypted export (needs a password).
    pub fn is_encrypted_export(json: &str) -> bool {
        serde_json::from_str::<serde_json::Value>(json)
            .ok()
            .and_then(|v| v.get("encrypted").and_then(|e| e.as_bool()))
            .unwrap_or(false)
    }

    fn parse_export(json: &str, password: Option<&str>) -> AppResult<Vec<ExportEntry>> {
        let value: serde_json::Value = serde_json::from_str(json)?;
        // Lenient: accept a bare array of entries.
        if value.is_array() {
            return Ok(serde_json::from_value(value)?);
        }
        let file: ExportFile = serde_json::from_value(value)?;
        if !file.encrypted {
            return file.entries.ok_or_else(|| AppError::Invalid("文件中没有 entries 字段".into()));
        }
        let pw = password.filter(|p| !p.is_empty()).ok_or(AppError::NeedPassword)?;
        let kdf = file.kdf.ok_or_else(|| AppError::Invalid("缺少 kdf 参数".into()))?;
        let data = file.data.ok_or_else(|| AppError::Invalid("缺少加密数据".into()))?;
        let blob = B64.decode(data).map_err(|_| AppError::Invalid("加密数据不是合法的 base64".into()))?;
        let key = kdf.derive(pw)?;
        let plain = crypto::decrypt(&key, &blob, EXPORT_AAD).map_err(|_| AppError::WrongPassword)?;
        Ok(serde_json::from_slice(&plain)?)
    }

    pub fn import_json(&mut self, json: &str, password: Option<&str>, mode: ConflictMode) -> AppResult<ImportReport> {
        let incoming = Self::parse_export(json, password)?;
        let mut report = ImportReport::default();
        let mut entries = self.entries.clone();
        let now = now_ts();
        let mut changed: Vec<usize> = Vec::new();

        for item in incoming {
            let mut data = item.data;
            data.normalize();
            let existing = entries.iter().position(|e| e.data.identity() == data.identity());
            match (existing, mode) {
                (Some(_), ConflictMode::Skip) => report.skipped += 1,
                (Some(idx), ConflictMode::Overwrite) => {
                    if entries[idx].data == data {
                        report.skipped += 1;
                    } else {
                        entries[idx].data = data;
                        entries[idx].updated_at = now;
                        changed.push(idx);
                        report.updated += 1;
                    }
                }
                _ => {
                    let id = item
                        .id
                        .filter(|id| !id.is_empty() && !entries.iter().any(|e| &e.id == id))
                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                    entries.push(Entry {
                        id,
                        data,
                        created_at: item.created_at.unwrap_or(now),
                        updated_at: item.updated_at.unwrap_or(now),
                    });
                    changed.push(entries.len() - 1);
                    report.added += 1;
                }
            }
        }

        let raws = changed
            .iter()
            .map(|&i| Self::to_raw(&entries[i]))
            .collect::<AppResult<Vec<_>>>()?;
        self.store.transaction(|s| raws.iter().try_for_each(|r| s.upsert(r)))?;
        self.entries = entries;
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(title: &str, host: &str, user: &str, pw: &str) -> EntryData {
        EntryData {
            title: title.into(),
            category: "database".into(),
            host: host.into(),
            port: Some(3306),
            username: user.into(),
            password: pw.into(),
            ..Default::default()
        }
    }

    fn new_vault() -> Vault {
        Vault::new(Store::open_in_memory().unwrap()).unwrap()
    }

    #[test]
    fn export_import_roundtrip() {
        let mut v = new_vault();
        v.create(sample("a", "h.com", "root", "secret-pass-一")).unwrap();
        v.create(sample("中文", "", "张三", "密码")).unwrap();

        for pw in [None, Some("export-pw")] {
            let json = v.export_json(pw).unwrap();
            assert_eq!(Vault::is_encrypted_export(&json), pw.is_some());
            if pw.is_some() {
                assert!(!json.contains("secret-pass") && !json.contains("h.com"));
            }
            let mut other = new_vault();
            if pw.is_some() {
                assert!(matches!(other.import_json(&json, None, ConflictMode::Skip), Err(AppError::NeedPassword)));
                assert!(matches!(
                    other.import_json(&json, Some("bad-pw"), ConflictMode::Skip),
                    Err(AppError::WrongPassword)
                ));
            }
            let r = other.import_json(&json, pw, ConflictMode::Skip).unwrap();
            assert_eq!(r.added, 2);
            let r = other.import_json(&json, pw, ConflictMode::Skip).unwrap();
            assert_eq!((r.added, r.skipped), (0, 2));
            let r = other.import_json(&json, pw, ConflictMode::KeepBoth).unwrap();
            assert_eq!(r.added, 2);
            assert_eq!(other.list().unwrap().len(), 4);
        }
    }

    #[test]
    fn import_overwrite_and_bare_array() {
        let mut v = new_vault();
        v.create(sample("a", "h.com", "root", "old")).unwrap();
        let json = r#"[{"title":"a","host":"H.com","username":"root","password":"new"}]"#;
        let r = v.import_json(json, None, ConflictMode::Overwrite).unwrap();
        assert_eq!(r.updated, 1);
        let id = v.list().unwrap()[0].id.clone();
        assert_eq!(v.get(&id).unwrap().data.password, "new");
        assert_eq!(v.get(&id).unwrap().data.password, "new");
    }

    #[test]
    fn persists_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("k.db");
        let id = {
            let mut v = Vault::new(Store::open(&path).unwrap()).unwrap();
            v.create(sample("中文", "h.com", "张三", "密码!@#")).unwrap().id
        };
        let v = Vault::new(Store::open(&path).unwrap()).unwrap();
        assert_eq!(v.get(&id).unwrap().data.password, "密码!@#");
    }
}
