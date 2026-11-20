//! 界面偏好持久化：主题模式、收藏工具、最近使用及界面缩放比例，异常时静默降级。
use crate::theme::ThemeMode;
use serde_json::{Value, json};
use std::path::PathBuf;

pub(super) struct UiPrefs {
    path: Option<PathBuf>,
    pub(super) theme: ThemeMode,
    pub(super) zoom: f32,
    pub(super) favorites: Vec<String>,
    pub(super) recent: Vec<String>,
}

impl UiPrefs {
    pub(super) fn load(path: Option<PathBuf>) -> Self {
        let mut prefs = Self {
            path: path.clone(),
            theme: ThemeMode::Light,
            zoom: 1.0,
            favorites: Vec::new(),
            recent: Vec::new(),
        };
        let Some(path) = path else {
            return prefs;
        };
        let Ok(text) = std::fs::read_to_string(path) else {
            return prefs;
        };
        let Ok(root) = serde_json::from_str::<Value>(&text) else {
            return prefs;
        };
        if let Some(mode) = root["theme"].as_str() {
            prefs.theme = ThemeMode::from_str(mode);
        }
        if let Some(zoom) = root["zoom"].as_f64() {
            let z = zoom as f32;
            if (0.85..=1.35).contains(&z) {
                prefs.zoom = z;
            }
        }
        if let Some(items) = root["favorites"].as_array() {
            prefs.favorites = items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect();
        }
        if let Some(items) = root["recent"].as_array() {
            prefs.recent = items
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .take(8)
                .collect();
        }
        prefs
    }

    pub(super) fn save(&self) {
        let Some(path) = &self.path else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let body = json!({
            "version": 1,
            "theme": self.theme.as_str(),
            "zoom": self.zoom,
            "favorites": self.favorites,
            "recent": self.recent,
        });
        if let Ok(bytes) = serde_json::to_vec_pretty(&body) {
            let _ = std::fs::write(path, bytes);
        }
    }
}
