use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use serde_json::{json, Value};
use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};
use crate::types::{BgMode, Settings, TextColorMode, User};

pub fn data_dir() -> PathBuf {
    let base = env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(base).join("rustblox");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn log(msg: &str) {
    if let Ok(mut f) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(data_dir().join("log.txt"))
    {
        let _ = writeln!(f, "{}", msg);
    }
}

pub fn read_lines(file: &str) -> Vec<String> {
    fs::read_to_string(data_dir().join(file))
        .map(|t| {
            t.lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

pub fn write_lines(file: &str, lines: &[String]) {
    let _ = fs::write(data_dir().join(file), lines.join("\n"));
}

pub fn read_ids(file: &str) -> Vec<u64> {
    read_lines(file).iter().filter_map(|l| l.parse().ok()).collect()
}

pub fn write_ids(file: &str, ids: &[u64]) {
    let lines: Vec<String> = ids.iter().map(|i| i.to_string()).collect();
    write_lines(file, &lines);
}

fn read_opt_rgb(v: &Value, key: &str) -> Option<[u8; 3]> {
    let arr = v[key].as_array()?;
    if arr.len() != 3 {
        return None;
    }
    Some([
        arr[0].as_u64()?.min(255) as u8,
        arr[1].as_u64()?.min(255) as u8,
        arr[2].as_u64()?.min(255) as u8,
    ])
}

fn opt_rgb_json(c: Option<[u8; 3]>) -> Value {
    match c {
        Some([r, g, b]) => serde_json::json!([r, g, b]),
        None => Value::Null,
    }
}

pub fn load_settings() -> Settings {
    let Ok(text) = fs::read_to_string(data_dir().join("settings.json")) else {
        return Settings::default();
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return Settings::default();
    };
    let def = Settings::default();
    let bg_mode = match v["bg_mode"].as_str() {
        Some("color") => BgMode::SolidColor,
        Some("wallpaper") => BgMode::Wallpaper,
        _ => BgMode::ThemeDefault,
    };
    let bg_color = read_opt_rgb(&v, "bg_color").unwrap_or(def.bg_color);
    let bg_wallpaper = v["bg_wallpaper"].as_str().unwrap_or("").to_string();
    let bg_overlay = v["bg_overlay"]
        .as_f64()
        .map(|f| (f as f32).clamp(0.0, 0.85))
        .unwrap_or(def.bg_overlay);
    let text_mode = match v["text_mode"].as_str() {
        Some("white") => TextColorMode::White,
        Some("green") => TextColorMode::Green,
        Some("dark") => TextColorMode::Dark,
        _ => TextColorMode::Auto,
    };
    Settings {
        dark: v["dark"].as_bool().unwrap_or(false),
        bg_mode,
        bg_color,
        bg_wallpaper,
        bg_overlay,
        text_mode,
        borders_color: read_opt_rgb(&v, "borders_color"),
        profile_color: read_opt_rgb(&v, "profile_color"),
        friends_color: read_opt_rgb(&v, "friends_color"),
        games_border_color: read_opt_rgb(&v, "games_border_color"),
        text_border_color: read_opt_rgb(&v, "text_border_color"),
        target_fps: v["target_fps"].as_u64().and_then(|n| u32::try_from(n).ok()),
        auto_close_roblox: v["auto_close_roblox"].as_bool().unwrap_or(true),
        notifications_silent: v["notifications_silent"].as_bool().unwrap_or(false),
        auto_check_updates: v["auto_check_updates"].as_bool().unwrap_or(true),
    }
}

pub fn save_settings(s: &Settings) {
    let mode_str = match s.bg_mode {
        BgMode::ThemeDefault => "default",
        BgMode::SolidColor => "color",
        BgMode::Wallpaper => "wallpaper",
    };
    let text_str = match s.text_mode {
        TextColorMode::Auto => "auto",
        TextColorMode::White => "white",
        TextColorMode::Green => "green",
        TextColorMode::Dark => "dark",
    };
    let v = serde_json::json!({
        "dark": s.dark,
        "bg_mode": mode_str,
        "bg_color": [s.bg_color[0], s.bg_color[1], s.bg_color[2]],
        "bg_wallpaper": s.bg_wallpaper,
        "bg_overlay": s.bg_overlay,
        "text_mode": text_str,
        "borders_color": opt_rgb_json(s.borders_color),
        "profile_color": opt_rgb_json(s.profile_color),
        "friends_color": opt_rgb_json(s.friends_color),
        "games_border_color": opt_rgb_json(s.games_border_color),
        "text_border_color": opt_rgb_json(s.text_border_color),
        "target_fps": s.target_fps,
        "auto_close_roblox": s.auto_close_roblox,
        "notifications_silent": s.notifications_silent,
        "auto_check_updates": s.auto_check_updates,
    });
    if let Ok(text) = serde_json::to_string_pretty(&v) {
        let _ = fs::write(data_dir().join("settings.json"), text);
    }
}

pub fn dpapi_transform(input: &[u8], encrypt: bool) -> Option<Vec<u8>> {
    if input.is_empty() {
        return None;
    }
    let in_blob = CRYPT_INTEGER_BLOB {
        cbData: u32::try_from(input.len()).ok()?,
        pbData: input.as_ptr() as *mut u8,
    };
    let mut out_blob = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    unsafe {
        let ok = if encrypt {
            CryptProtectData(
                &in_blob,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out_blob,
            )
        } else {
            CryptUnprotectData(
                &in_blob,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out_blob,
            )
        };
        if ok == 0 || out_blob.pbData.is_null() {
            return None;
        }
        let result =
            std::slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize).to_vec();
        LocalFree(out_blob.pbData as _);
        Some(result)
    }
}

pub fn save_cookie(cookie: &str) -> Result<(), String> {
    let encrypted = dpapi_transform(cookie.as_bytes(), true)
        .ok_or_else(|| "Windows DPAPI encryption failed".to_string())?;
    fs::write(data_dir().join("session.dat"), encrypted).map_err(|e| e.to_string())
}

pub fn load_cookie() -> Option<String> {
    let encrypted = fs::read(data_dir().join("session.dat")).ok()?;
    let decrypted = dpapi_transform(&encrypted, false)?;
    String::from_utf8(decrypted).ok()
}

pub fn clear_cookie() {
    let path = data_dir().join("session.dat");
    if let Ok(meta) = fs::metadata(&path) {
        let len = usize::try_from(meta.len()).unwrap_or(0).min(1024 * 64);
        if len > 0 {
            let _ = fs::write(&path, vec![0u8; len]);
        }
    }
    let _ = fs::remove_file(path);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredAccount {
    pub id: u64,
    pub name: String,
    pub display: String,
    pub headshot: String,
    pub verified: bool,
    pub cookie: String,
    pub last_used_ms: u64,
}

impl StoredAccount {
    fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "display": self.display,
            "headshot": self.headshot,
            "verified": self.verified,
            "cookie": self.cookie,
            "last_used_ms": self.last_used_ms,
        })
    }

    fn from_json(v: &Value) -> Option<Self> {
        let id = v.get("id")?.as_u64()?;
        let name = v.get("name")?.as_str()?.to_string();
        let cookie = v.get("cookie")?.as_str()?.to_string();
        if id == 0 || cookie.trim().is_empty() {
            return None;
        }
        let display = v
            .get("display")
            .and_then(|x| x.as_str())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(&name)
            .to_string();
        Some(Self {
            id,
            name,
            display,
            headshot: v
                .get("headshot")
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string(),
            verified: v.get("verified").and_then(|x| x.as_bool()).unwrap_or(false),
            cookie,
            last_used_ms: v.get("last_used_ms").and_then(|x| x.as_u64()).unwrap_or(0),
        })
    }
}

pub fn load_accounts() -> Vec<StoredAccount> {
    let Ok(encrypted) = fs::read(data_dir().join("accounts.dat")) else {
        return Vec::new();
    };
    let Some(decrypted) = dpapi_transform(&encrypted, false) else {
        return Vec::new();
    };
    let Ok(text) = String::from_utf8(decrypted) else {
        return Vec::new();
    };
    let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    arr.iter().filter_map(StoredAccount::from_json).collect()
}

pub fn save_accounts(accounts: &[StoredAccount]) -> Result<(), String> {
    let path = data_dir().join("accounts.dat");
    if accounts.is_empty() {
        if let Ok(meta) = fs::metadata(&path) {
            let len = usize::try_from(meta.len()).unwrap_or(0).min(1024 * 64);
            if len > 0 {
                let _ = fs::write(&path, vec![0u8; len]);
            }
        }
        let _ = fs::remove_file(&path);
        return Ok(());
    }
    let arr: Vec<Value> = accounts.iter().map(StoredAccount::to_json).collect();
    let text = serde_json::to_string(&Value::Array(arr)).map_err(|e| e.to_string())?;
    let encrypted = dpapi_transform(text.as_bytes(), true)
        .ok_or_else(|| "Windows DPAPI encryption failed".to_string())?;
    fs::write(path, encrypted).map_err(|e| e.to_string())
}

pub fn upsert_stored_account(
    accounts: &mut Vec<StoredAccount>,
    user: &User,
    cookie: &str,
    last_used_ms: u64,
) {
    if cookie.trim().is_empty() || user.id == 0 {
        return;
    }
    let display = if user.display.trim().is_empty() {
        user.name.clone()
    } else {
        user.display.clone()
    };
    if let Some(existing) = accounts
        .iter_mut()
        .find(|a| a.id == user.id || a.cookie == cookie)
    {
        existing.id = user.id;
        existing.name = user.name.clone();
        existing.display = display;
        existing.headshot = user.headshot.clone();
        existing.verified = user.verified;
        existing.cookie = cookie.to_string();
        existing.last_used_ms = last_used_ms;
    } else {
        accounts.push(StoredAccount {
            id: user.id,
            name: user.name.clone(),
            display,
            headshot: user.headshot.clone(),
            verified: user.verified,
            cookie: cookie.to_string(),
            last_used_ms,
        });
    }
}

pub fn remove_stored_account(accounts: &mut Vec<StoredAccount>, user_id: u64) -> Option<StoredAccount> {
    let pos = accounts.iter().position(|a| a.id == user_id)?;
    Some(accounts.remove(pos))
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SeenNotifications {
    pub update_uploads: HashSet<String>,
    pub friend_requests_by_user: HashMap<u64, HashSet<u64>>,
}

impl SeenNotifications {
    pub fn from_json_str(text: &str) -> Self {
        let Ok(v) = serde_json::from_str::<Value>(text) else {
            return Self::default();
        };
        let update_uploads = v["update_uploads"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|x| x.as_str().map(|s| s.to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        let mut friend_requests_by_user: HashMap<u64, HashSet<u64>> = HashMap::new();
        if let Some(obj) = v["friend_requests_by_user"].as_object() {
            for (k, arr) in obj {
                let Ok(owner_id) = k.parse::<u64>() else {
                    continue;
                };
                let ids: HashSet<u64> = arr
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|x| x.as_u64())
                    .filter(|&id| id != 0)
                    .collect();
                if !ids.is_empty() {
                    friend_requests_by_user.insert(owner_id, ids);
                }
            }
        }
        Self {
            update_uploads,
            friend_requests_by_user,
        }
    }

    pub fn to_json_value(&self) -> Value {
        let mut updates: Vec<&str> = self.update_uploads.iter().map(|s| s.as_str()).collect();
        updates.sort_unstable();
        let mut fr_map = serde_json::Map::new();
        for (&owner_id, set) in &self.friend_requests_by_user {
            if set.is_empty() {
                continue;
            }
            let mut ids: Vec<u64> = set.iter().copied().collect();
            ids.sort_unstable();
            fr_map.insert(owner_id.to_string(), json!(ids));
        }
        json!({
            "update_uploads": updates,
            "friend_requests_by_user": Value::Object(fr_map),
        })
    }
}

pub fn load_seen_notifications() -> SeenNotifications {
    let path = data_dir().join("seen_notifications.json");
    let Ok(text) = fs::read_to_string(path) else {
        return SeenNotifications::default();
    };
    SeenNotifications::from_json_str(&text)
}

pub fn save_seen_notifications(seen: &SeenNotifications) {
    if let Ok(text) = serde_json::to_string_pretty(&seen.to_json_value()) {
        let _ = fs::write(data_dir().join("seen_notifications.json"), text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_user(id: u64, name: &str) -> User {
        User {
            id,
            name: name.to_string(),
            display: format!("{name} Display"),
            headshot: "https://example.com/a.png".to_string(),
            verified: true,
            ..Default::default()
        }
    }

    #[test]
    fn upsert_and_remove_accounts_behaves_cleanly() {
        let mut vault = Vec::new();
        let u1 = sample_user(101, "alice");
        let u2 = sample_user(202, "bob");

        upsert_stored_account(&mut vault, &u1, "cookie_1", 10);
        upsert_stored_account(&mut vault, &u2, "cookie_2", 20);
        assert_eq!(vault.len(), 2);

        let mut u1_updated = u1.clone();
        u1_updated.name = "alice_new".to_string();
        upsert_stored_account(&mut vault, &u1_updated, "cookie_1_new", 30);
        assert_eq!(vault.len(), 2);
        assert_eq!(vault[0].name, "alice_new");
        assert_eq!(vault[0].cookie, "cookie_1_new");

        let removed = remove_stored_account(&mut vault, 101);
        assert!(removed.is_some());
        assert_eq!(vault.len(), 1);
        assert_eq!(vault[0].id, 202);
    }

    #[test]
    fn seen_notifications_roundtrip_and_dedup() {
        let mut seen = SeenNotifications::default();
        assert!(seen.update_uploads.insert("version-abc123".to_string()));
        assert!(!seen.update_uploads.insert("version-abc123".to_string()));
        let reqs = seen.friend_requests_by_user.entry(42).or_default();
        reqs.insert(1001);
        reqs.insert(1002);

        let serialized = seen.to_json_value().to_string();
        let parsed = SeenNotifications::from_json_str(&serialized);
        assert_eq!(parsed, seen);
        assert!(parsed.update_uploads.contains("version-abc123"));
        assert!(parsed.friend_requests_by_user[&42].contains(&1001));
        assert!(parsed.friend_requests_by_user[&42].contains(&1002));
        assert!(!parsed.friend_requests_by_user[&42].contains(&1003));
    }
}
