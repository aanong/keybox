use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

pub const CATEGORIES: [&str; 4] = ["database", "website", "server", "other"];

/// The payload of an entry; stored as JSON.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Zeroize, PartialEq)]
#[serde(default)]
pub struct EntryData {
    pub title: String,
    pub category: String,
    pub host: String,
    pub port: Option<u16>,
    pub username: String,
    pub password: String,
    pub notes: String,
    pub tags: Vec<String>,
    pub favorite: bool,
}

impl EntryData {
    pub fn normalize(&mut self) {
        self.title = self.title.trim().to_string();
        self.host = self.host.trim().to_string();
        self.username = self.username.trim().to_string();
        if !CATEGORIES.contains(&self.category.as_str()) {
            self.category = "other".into();
        }
        let mut tags: Vec<String> = self
            .tags
            .iter()
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        tags.dedup();
        self.tags = tags;
        if self.title.is_empty() {
            self.title = if !self.host.is_empty() {
                self.host.clone()
            } else if !self.username.is_empty() {
                self.username.clone()
            } else {
                "未命名".into()
            };
        }
    }

    /// Identity used for duplicate detection during import.
    pub fn identity(&self) -> (String, String) {
        let key = if self.host.is_empty() { self.title.to_lowercase() } else { self.host.to_lowercase() };
        (key, self.username.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    #[serde(flatten)]
    pub data: EntryData,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Entry as shown in lists: never carries the password.
#[derive(Debug, Clone, Serialize)]
pub struct EntryView {
    pub id: String,
    pub title: String,
    pub category: String,
    pub host: String,
    pub port: Option<u16>,
    pub username: String,
    pub has_password: bool,
    pub notes: String,
    pub tags: Vec<String>,
    pub favorite: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<&Entry> for EntryView {
    fn from(e: &Entry) -> Self {
        Self {
            id: e.id.clone(),
            title: e.data.title.clone(),
            category: e.data.category.clone(),
            host: e.data.host.clone(),
            port: e.data.port,
            username: e.data.username.clone(),
            has_password: !e.data.password.is_empty(),
            notes: e.data.notes.clone(),
            tags: e.data.tags.clone(),
            favorite: e.data.favorite,
            created_at: e.created_at,
            updated_at: e.updated_at,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// 0 = never clear
    pub clipboard_clear_seconds: u32,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ConflictMode {
    Skip,
    Overwrite,
    KeepBoth,
}

#[derive(Debug, Default, Serialize)]
pub struct ImportReport {
    pub added: usize,
    pub updated: usize,
    pub skipped: usize,
}

pub fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}
