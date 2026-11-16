use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde_json::{json, Value};

pub const DEFAULT_DISCORD_APP_ID: &str = "1005469189907173486";
const MAX_IPC_FRAME_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscordRpcConfig {
    pub enabled: bool,
    pub show_launcher_idle: bool,
    pub app_id: String,
}

impl Default for DiscordRpcConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            show_launcher_idle: true,
            app_id: String::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscordActivity {
    pub details: String,
    pub state: String,
    pub start_epoch_secs: u64,
    pub large_image: Option<String>,
    pub large_text: Option<String>,
    pub small_image: Option<String>,
    pub small_text: Option<String>,
    pub button_label: Option<String>,
    pub button_url: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct DiscordRpcStatus {
    pub connected: bool,
    pub status_text: String,
}

enum RpcCmd {
    Sync {
        config: DiscordRpcConfig,
        activity: Option<DiscordActivity>,
    },
    Shutdown,
}

pub fn resolve_app_id(custom: &str) -> &str {
    let trimmed = custom.trim();
    if trimmed.len() >= 15 && trimmed.bytes().all(|b| b.is_ascii_digit()) {
        trimmed
    } else {
        DEFAULT_DISCORD_APP_ID
    }
}

pub fn sanitize_rpc_text(raw: &str, fallback: &str) -> String {
    let trimmed = raw.trim();
    let chosen = if trimmed.chars().count() >= 2 {
        trimmed
    } else {
        fallback
    };
    let mut out = String::new();
    for ch in chosen.chars().take(120) {
        out.push(ch);
    }
    out
}

pub fn encode_ipc_frame(opcode: u32, payload: &str) -> Vec<u8> {
    let bytes = payload.as_bytes();
    let mut buf = Vec::with_capacity(8 + bytes.len());
    buf.extend_from_slice(&opcode.to_le_bytes());
    buf.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    buf.extend_from_slice(bytes);
    buf
}

pub fn decode_ipc_frame_header(header: [u8; 8]) -> (u32, usize) {
    let opcode = u32::from_le_bytes([header[0], header[1], header[2], header[3]]);
    let len = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as usize;
    (opcode, len)
}

pub fn build_set_activity_payload(
    pid: u32,
    activity: Option<&DiscordActivity>,
    nonce: u64,
) -> String {
    let activity_val = match activity {
        Some(a) => {
            let mut obj = serde_json::Map::new();
            obj.insert(
                "details".to_string(),
                Value::String(sanitize_rpc_text(&a.details, "RustBlox")),
            );
            obj.insert(
                "state".to_string(),
                Value::String(sanitize_rpc_text(&a.state, "Active")),
            );
            if a.start_epoch_secs > 0 {
                obj.insert(
                    "timestamps".to_string(),
                    json!({ "start": a.start_epoch_secs }),
                );
            }

            let mut assets = serde_json::Map::new();
            if let Some(img) = a
                .large_image
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                assets.insert("large_image".to_string(), Value::String(img.to_string()));
            }
            if let Some(txt) = a
                .large_text
                .as_deref()
                .map(str::trim)
                .filter(|s| s.chars().count() >= 2)
            {
                assets.insert(
                    "large_text".to_string(),
                    Value::String(sanitize_rpc_text(txt, "Roblox")),
                );
            }
            if let Some(img) = a
                .small_image
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                assets.insert("small_image".to_string(), Value::String(img.to_string()));
            }
            if let Some(txt) = a
                .small_text
                .as_deref()
                .map(str::trim)
                .filter(|s| s.chars().count() >= 2)
            {
                assets.insert(
                    "small_text".to_string(),
                    Value::String(sanitize_rpc_text(txt, "RustBlox")),
                );
            }
            if !assets.is_empty() {
                obj.insert("assets".to_string(), Value::Object(assets));
            }

            if let (Some(label), Some(url)) = (a.button_label.as_deref(), a.button_url.as_deref()) {
                let lbl = label.trim();
                let u = url.trim();
                if lbl.chars().count() >= 2 && (u.starts_with("https://") || u.starts_with("http://"))
                {
                    let mut short_lbl = String::new();
                    for ch in lbl.chars().take(32) {
                        short_lbl.push(ch);
                    }
                    obj.insert(
                        "buttons".to_string(),
                        json!([{ "label": short_lbl, "url": u }]),
                    );
                }
            }

            Value::Object(obj)
        }
        None => Value::Null,
    };

    json!({
        "cmd": "SET_ACTIVITY",
        "args": {
            "pid": pid,
            "activity": activity_val
        },
        "nonce": format!("rbx-{nonce}")
    })
    .to_string()
}

pub struct DiscordRpcClient {
    tx: Sender<RpcCmd>,
    status: Arc<Mutex<DiscordRpcStatus>>,
    last_sent: Mutex<Option<(DiscordRpcConfig, Option<DiscordActivity>)>>,
    stopped: AtomicBool,
}

impl DiscordRpcClient {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        let status = Arc::new(Mutex::new(DiscordRpcStatus {
            connected: false,
            status_text: "Waiting for Discord".to_string(),
        }));
        let status_bg = Arc::clone(&status);
        thread::Builder::new()
            .name("discord-rpc".to_string())
            .spawn(move || rpc_worker_loop(rx, status_bg))
            .ok();

        Self {
            tx,
            status,
            last_sent: Mutex::new(None),
            stopped: AtomicBool::new(false),
        }
    }

    pub fn sync(&self, config: DiscordRpcConfig, activity: Option<DiscordActivity>) {
        if self.stopped.load(Ordering::Relaxed) {
            return;
        }
        let payload = (config, activity);
        if let Ok(mut guard) = self.last_sent.lock() {
            if guard.as_ref() == Some(&payload) {
                return;
            }
            *guard = Some(payload.clone());
        }
        let _ = self.tx.send(RpcCmd::Sync {
            config: payload.0,
            activity: payload.1,
        });
    }

    pub fn status(&self) -> DiscordRpcStatus {
        self.status
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    pub fn shutdown(&self) {
        if !self.stopped.swap(true, Ordering::Relaxed) {
            let _ = self.tx.send(RpcCmd::Shutdown);
        }
    }
}

impl Drop for DiscordRpcClient {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn set_status(status: &Arc<Mutex<DiscordRpcStatus>>, connected: bool, text: impl Into<String>) {
    if let Ok(mut s) = status.lock() {
        s.connected = connected;
        s.status_text = text.into();
    }
}

#[cfg(windows)]
mod ipc {
    use std::fs::{File, OpenOptions};
    use std::io::{Read, Write};

    use super::{decode_ipc_frame_header, encode_ipc_frame, MAX_IPC_FRAME_BYTES};

    pub struct PipeConn {
        file: File,
        pub app_id: String,
    }

    pub fn connect(app_id: &str) -> Result<PipeConn, String> {
        let mut last_err = "Discord desktop app not detected".to_string();
        for idx in 0..10 {
            let path = format!(r"\\.\pipe\discord-ipc-{idx}");
            match OpenOptions::new().read(true).write(true).open(&path) {
                Ok(mut file) => {
                    let handshake = serde_json::json!({
                        "v": 1,
                        "client_id": app_id
                    })
                    .to_string();
                    let frame = encode_ipc_frame(0, &handshake);
                    if let Err(e) = file.write_all(&frame).and_then(|_| file.flush()) {
                        last_err = e.to_string();
                        continue;
                    }
                    match read_frame(&mut file) {
                        Ok((_op, body)) => {
                            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                                if v["evt"].as_str() == Some("ERROR") {
                                    last_err = v["data"]["message"]
                                        .as_str()
                                        .unwrap_or("Discord rejected application ID")
                                        .to_string();
                                    continue;
                                }
                            }
                            return Ok(PipeConn {
                                file,
                                app_id: app_id.to_string(),
                            });
                        }
                        Err(e) => {
                            last_err = e;
                        }
                    }
                }
                Err(e) => {
                    last_err = e.to_string();
                }
            }
        }
        Err(last_err)
    }

    pub fn send_frame(conn: &mut PipeConn, payload: &str) -> Result<(), String> {
        let frame = encode_ipc_frame(1, payload);
        conn.file
            .write_all(&frame)
            .and_then(|_| conn.file.flush())
            .map_err(|e| e.to_string())?;
        let _ = read_frame(&mut conn.file)?;
        Ok(())
    }

    fn read_frame(file: &mut File) -> Result<(u32, String), String> {
        let mut hdr = [0u8; 8];
        file.read_exact(&mut hdr).map_err(|e| e.to_string())?;
        let (op, len) = decode_ipc_frame_header(hdr);
        if len > MAX_IPC_FRAME_BYTES {
            return Err("Discord IPC frame exceeded size limit".to_string());
        }
        let mut buf = vec![0u8; len];
        file.read_exact(&mut buf).map_err(|e| e.to_string())?;
        Ok((op, String::from_utf8_lossy(&buf).into_owned()))
    }
}

#[cfg(not(windows))]
mod ipc {
    pub struct PipeConn {
        pub app_id: String,
    }

    pub fn connect(app_id: &str) -> Result<PipeConn, String> {
        let _ = app_id;
        Err("Discord IPC supported on Windows".to_string())
    }

    pub fn send_frame(_conn: &mut PipeConn, _payload: &str) -> Result<(), String> {
        Err("Discord IPC supported on Windows".to_string())
    }
}

fn rpc_worker_loop(rx: mpsc::Receiver<RpcCmd>, status: Arc<Mutex<DiscordRpcStatus>>) {
    let pid = std::process::id();
    let mut cfg = DiscordRpcConfig::default();
    let mut desired: Option<DiscordActivity> = None;
    let mut conn: Option<ipc::PipeConn> = None;
    let mut dirty = false;
    let mut nonce: u64 = 1;

    loop {
        let wait_time = if !cfg.enabled {
            Duration::from_secs(3600)
        } else if conn.is_none() {
            Duration::from_secs(4)
        } else {
            Duration::from_secs(30)
        };

        let msg = if cfg.enabled {
            rx.recv_timeout(wait_time)
        } else {
            rx.recv().map_err(|_| RecvTimeoutError::Disconnected)
        };

        let mut shutdown = false;
        match msg {
            Ok(RpcCmd::Sync { config, activity }) => {
                cfg = config;
                desired = activity;
                dirty = true;
            }
            Ok(RpcCmd::Shutdown) | Err(RecvTimeoutError::Disconnected) => {
                shutdown = true;
            }
            Err(RecvTimeoutError::Timeout) => {}
        }

        // Drain any burst of newer updates so we always publish the latest state.
        while let Ok(next) = rx.try_recv() {
            match next {
                RpcCmd::Sync { config, activity } => {
                    cfg = config;
                    desired = activity;
                    dirty = true;
                }
                RpcCmd::Shutdown => {
                    shutdown = true;
                }
            }
        }

        if shutdown {
            if let Some(c) = &mut conn {
                nonce = nonce.wrapping_add(1);
                let clear_json = build_set_activity_payload(pid, None, nonce);
                let _ = ipc::send_frame(c, &clear_json);
            }
            set_status(&status, false, "Disabled");
            break;
        }

        if !cfg.enabled {
            if let Some(c) = &mut conn {
                nonce = nonce.wrapping_add(1);
                let clear_json = build_set_activity_payload(pid, None, nonce);
                let _ = ipc::send_frame(c, &clear_json);
            }
            conn = None;
            dirty = false;
            set_status(&status, false, "Disabled");
            continue;
        }

        let target_app_id = resolve_app_id(&cfg.app_id).to_string();
        if conn
            .as_ref()
            .is_some_and(|c| c.app_id != target_app_id)
        {
            conn = None;
            dirty = true;
        }

        if conn.is_none() {
            match ipc::connect(&target_app_id) {
                Ok(new_conn) => {
                    conn = Some(new_conn);
                    dirty = true;
                    set_status(&status, true, "Connected to Discord");
                }
                Err(_) => {
                    set_status(&status, false, "Waiting for Discord desktop app");
                    continue;
                }
            }
        }

        if dirty {
            if let Some(c) = &mut conn {
                nonce = nonce.wrapping_add(1);
                let payload = build_set_activity_payload(pid, desired.as_ref(), nonce);
                match ipc::send_frame(c, &payload) {
                    Ok(()) => {
                        dirty = false;
                        if let Some(act) = &desired {
                            set_status(&status, true, format!("Connected • {}", act.details));
                        } else {
                            set_status(&status, true, "Connected (idle hidden)");
                        }
                    }
                    Err(_) => {
                        conn = None;
                        set_status(&status, false, "Reconnecting to Discord...");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_and_decodes_ipc_frame_headers() {
        let frame = encode_ipc_frame(1, "{\"ok\":true}");
        let hdr: [u8; 8] = frame[..8].try_into().unwrap();
        assert_eq!(decode_ipc_frame_header(hdr), (1, 11));
        assert_eq!(&frame[8..], b"{\"ok\":true}");
    }

    #[test]
    fn resolves_default_or_custom_discord_app_id() {
        assert_eq!(resolve_app_id(""), DEFAULT_DISCORD_APP_ID);
        assert_eq!(resolve_app_id("not-an-id"), DEFAULT_DISCORD_APP_ID);
        assert_eq!(resolve_app_id("12345"), DEFAULT_DISCORD_APP_ID);
        assert_eq!(
            resolve_app_id(" 123456789012345678 "),
            "123456789012345678"
        );
    }

    #[test]
    fn builds_valid_set_activity_and_clear_payloads() {
        let act = DiscordActivity {
            details: "Blox Fruits".to_string(),
            state: "By Gamer Robot".to_string(),
            start_epoch_secs: 1_700_000_000,
            large_image: Some("https://tr.rbxcdn.com/example.png".to_string()),
            large_text: Some("Blox Fruits".to_string()),
            small_image: None,
            small_text: None,
            button_label: Some("Open on Roblox".to_string()),
            button_url: Some("https://www.roblox.com/games/2753915549".to_string()),
        };
        let raw = build_set_activity_payload(42, Some(&act), 7);
        let v: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(v["cmd"], "SET_ACTIVITY");
        assert_eq!(v["args"]["pid"], 42);
        assert_eq!(v["args"]["activity"]["details"], "Blox Fruits");
        assert_eq!(v["args"]["activity"]["timestamps"]["start"], 1_700_000_000);
        assert_eq!(
            v["args"]["activity"]["assets"]["large_image"],
            "https://tr.rbxcdn.com/example.png"
        );
        assert_eq!(
            v["args"]["activity"]["buttons"][0]["url"],
            "https://www.roblox.com/games/2753915549"
        );

        let cleared = build_set_activity_payload(42, None, 8);
        let vc: Value = serde_json::from_str(&cleared).unwrap();
        assert!(vc["args"]["activity"].is_null());
    }
}
