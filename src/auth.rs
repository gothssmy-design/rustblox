use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use serde_json::Value;
use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};
use crate::types::{BgMode, Settings, TextColorMode};

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
    let bg_color = match v["bg_color"].as_array() {
        Some(arr) if arr.len() == 3 => [
            arr[0].as_u64().unwrap_or(def.bg_color[0] as u64).min(255) as u8,
            arr[1].as_u64().unwrap_or(def.bg_color[1] as u64).min(255) as u8,
            arr[2].as_u64().unwrap_or(def.bg_color[2] as u64).min(255) as u8,
        ],
        _ => def.bg_color,
    };
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
        target_fps: v["target_fps"].as_u64().and_then(|n| u32::try_from(n).ok()),
        auto_close_roblox: v["auto_close_roblox"].as_bool().unwrap_or(true),
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
        "target_fps": s.target_fps,
        "auto_close_roblox": s.auto_close_roblox,
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
