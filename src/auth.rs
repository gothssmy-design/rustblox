use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use serde_json::Value;
use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};
use crate::types::Settings;

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
    Settings {
        dark: v["dark"].as_bool().unwrap_or(false),
        target_fps: v["target_fps"].as_u64().and_then(|n| u32::try_from(n).ok()),
        auto_close_roblox: v["auto_close_roblox"].as_bool().unwrap_or(true),
    }
}

pub fn save_settings(s: &Settings) {
    let v = serde_json::json!({
        "dark": s.dark,
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
