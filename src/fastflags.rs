use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::{json, Map, Value};

use crate::auth::{data_dir, load_settings};
use crate::roblox_process::find_player;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RendererMode {
    #[default]
    Auto,
    D3D11,
    Vulkan,
    OpenGL,
}

impl RendererMode {
    pub const ALL: [Self; 4] = [Self::Auto, Self::D3D11, Self::Vulkan, Self::OpenGL];

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Automatic (Default)",
            Self::D3D11 => "Direct3D 11",
            Self::Vulkan => "Vulkan",
            Self::OpenGL => "OpenGL",
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::D3D11 => "d3d11",
            Self::Vulkan => "vulkan",
            Self::OpenGL => "opengl",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "d3d11" => Self::D3D11,
            "vulkan" => Self::Vulkan,
            "opengl" => Self::OpenGL,
            _ => Self::Auto,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LightingMode {
    #[default]
    Default,
    Voxel,
    ShadowMap,
    Future,
}

impl LightingMode {
    pub const ALL: [Self; 4] = [Self::Default, Self::Voxel, Self::ShadowMap, Self::Future];

    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "Game Default",
            Self::Voxel => "Voxel (Fastest)",
            Self::ShadowMap => "ShadowMap (Balanced)",
            Self::Future => "Future (High Quality)",
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Voxel => "voxel",
            Self::ShadowMap => "shadow_map",
            Self::Future => "future",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "voxel" => Self::Voxel,
            "shadow_map" => Self::ShadowMap,
            "future" => Self::Future,
            _ => Self::Default,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FastFlagsConfig {
    pub renderer: RendererMode,
    pub lighting: LightingMode,
    pub disable_shadows: bool,
    pub low_quality_textures: bool,
    pub disable_post_fx: bool,
    pub disable_grass: bool,
    pub disable_telemetry: bool,
    pub custom_flags: BTreeMap<String, Value>,
}

/// Keys controlled by rustblox's built-in FastFlags toggles so turning a
/// toggle off cleanly removes its entries from `ClientAppSettings.json`.
const BUILTIN_MANAGED_KEYS: [&str; 16] = [
    "DFIntTaskSchedulerTargetFps",
    "FFlagDebugGraphicsPreferD3D11",
    "FFlagDebugGraphicsPreferVulkan",
    "FFlagDebugGraphicsPreferOpenGL",
    "FFlagDebugForceVoxel",
    "FFlagDebugForceShadowMap",
    "FFlagDebugForceFutureIsBrightPhase3",
    "FIntRenderShadowIntensity",
    "DFFlagTextureQualityOverrideEnabled",
    "DFIntTextureQualityOverride",
    "FFlagDisablePostFx",
    "FIntFRMMinGrassDistance",
    "FIntFRMMaxGrassDistance",
    "FFlagDebugDisableTelemetryV2",
    "FFlagDebugDisableTelemetryEventIngest",
    "FFlagDebugDisableTelemetryPoint",
];

const EXTRA_MANAGED_KEYS: [&str; 1] = ["FFlagDebugDisableTelemetryCounter"];

impl FastFlagsConfig {
    pub fn apply_competitive_preset(&mut self) {
        self.renderer = RendererMode::D3D11;
        self.lighting = LightingMode::Voxel;
        self.disable_shadows = true;
        self.low_quality_textures = true;
        self.disable_post_fx = true;
        self.disable_grass = true;
        self.disable_telemetry = true;
    }

    pub fn apply_quality_preset(&mut self) {
        self.renderer = RendererMode::Auto;
        self.lighting = LightingMode::Future;
        self.disable_shadows = false;
        self.low_quality_textures = false;
        self.disable_post_fx = false;
        self.disable_grass = false;
        self.disable_telemetry = true;
    }

    pub fn reset_all(&mut self) {
        *self = Self::default();
    }
}

pub fn load_fastflags() -> FastFlagsConfig {
    let path = data_dir().join("fastflags.json");
    let Ok(text) = fs::read_to_string(&path) else {
        return FastFlagsConfig::default();
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return FastFlagsConfig::default();
    };
    let mut cfg = FastFlagsConfig::default();
    if let Some(s) = v.get("renderer").and_then(|x| x.as_str()) {
        cfg.renderer = RendererMode::from_str(s);
    }
    if let Some(s) = v.get("lighting").and_then(|x| x.as_str()) {
        cfg.lighting = LightingMode::from_str(s);
    }
    cfg.disable_shadows = v
        .get("disable_shadows")
        .and_then(|x| x.as_bool())
        .unwrap_or(false);
    cfg.low_quality_textures = v
        .get("low_quality_textures")
        .and_then(|x| x.as_bool())
        .unwrap_or(false);
    cfg.disable_post_fx = v
        .get("disable_post_fx")
        .and_then(|x| x.as_bool())
        .unwrap_or(false);
    cfg.disable_grass = v
        .get("disable_grass")
        .and_then(|x| x.as_bool())
        .unwrap_or(false);
    cfg.disable_telemetry = v
        .get("disable_telemetry")
        .and_then(|x| x.as_bool())
        .unwrap_or(false);
    if let Some(obj) = v.get("custom_flags").and_then(|x| x.as_object()) {
        for (k, val) in obj {
            cfg.custom_flags.insert(k.clone(), val.clone());
        }
    }
    cfg
}

pub fn save_fastflags(cfg: &FastFlagsConfig) -> Result<(), String> {
    let custom_obj: Map<String, Value> = cfg
        .custom_flags
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let v = json!({
        "renderer": cfg.renderer.as_str(),
        "lighting": cfg.lighting.as_str(),
        "disable_shadows": cfg.disable_shadows,
        "low_quality_textures": cfg.low_quality_textures,
        "disable_post_fx": cfg.disable_post_fx,
        "disable_grass": cfg.disable_grass,
        "disable_telemetry": cfg.disable_telemetry,
        "custom_flags": Value::Object(custom_obj),
    });
    let text = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?;
    fs::write(data_dir().join("fastflags.json"), text).map_err(|e| e.to_string())
}

pub fn parse_flag_value(raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.eq_ignore_ascii_case("true") {
        return Value::Bool(true);
    }
    if trimmed.eq_ignore_ascii_case("false") {
        return Value::Bool(false);
    }
    if trimmed.eq_ignore_ascii_case("null") {
        return Value::Null;
    }
    if let Ok(int_val) = trimmed.parse::<i64>() {
        return Value::from(int_val);
    }
    if let Ok(float_val) = trimmed.parse::<f64>() {
        if let Some(num) = serde_json::Number::from_f64(float_val) {
            return Value::Number(num);
        }
    }
    if let Ok(Value::String(unquoted)) = serde_json::from_str::<Value>(trimmed) {
        return Value::String(unquoted);
    }
    Value::String(trimmed.to_string())
}

pub fn format_flag_value(val: &Value) -> String {
    match val {
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

pub fn build_client_app_settings(
    cfg: &FastFlagsConfig,
    target_fps: Option<u32>,
    existing: Option<&Map<String, Value>>,
    removed_keys: &[&str],
) -> Map<String, Value> {
    let mut map = existing.cloned().unwrap_or_default();

    for key in BUILTIN_MANAGED_KEYS.iter().chain(EXTRA_MANAGED_KEYS.iter()) {
        map.remove(*key);
    }
    for key in removed_keys {
        map.remove(*key);
    }

    if let Some(fps) = target_fps {
        map.insert("DFIntTaskSchedulerTargetFps".to_string(), Value::from(fps));
    }

    match cfg.renderer {
        RendererMode::Auto => {}
        RendererMode::D3D11 => {
            map.insert("FFlagDebugGraphicsPreferD3D11".to_string(), Value::Bool(true));
        }
        RendererMode::Vulkan => {
            map.insert("FFlagDebugGraphicsPreferVulkan".to_string(), Value::Bool(true));
        }
        RendererMode::OpenGL => {
            map.insert("FFlagDebugGraphicsPreferOpenGL".to_string(), Value::Bool(true));
        }
    }

    match cfg.lighting {
        LightingMode::Default => {}
        LightingMode::Voxel => {
            map.insert("FFlagDebugForceVoxel".to_string(), Value::Bool(true));
        }
        LightingMode::ShadowMap => {
            map.insert("FFlagDebugForceShadowMap".to_string(), Value::Bool(true));
        }
        LightingMode::Future => {
            map.insert(
                "FFlagDebugForceFutureIsBrightPhase3".to_string(),
                Value::Bool(true),
            );
        }
    }

    if cfg.disable_shadows {
        map.insert("FIntRenderShadowIntensity".to_string(), Value::from(0));
    }

    if cfg.low_quality_textures {
        map.insert(
            "DFFlagTextureQualityOverrideEnabled".to_string(),
            Value::Bool(true),
        );
        map.insert("DFIntTextureQualityOverride".to_string(), Value::from(0));
    }

    if cfg.disable_post_fx {
        map.insert("FFlagDisablePostFx".to_string(), Value::Bool(true));
    }

    if cfg.disable_grass {
        map.insert("FIntFRMMinGrassDistance".to_string(), Value::from(0));
        map.insert("FIntFRMMaxGrassDistance".to_string(), Value::from(0));
    }

    if cfg.disable_telemetry {
        map.insert("FFlagDebugDisableTelemetryV2".to_string(), Value::Bool(true));
        map.insert(
            "FFlagDebugDisableTelemetryEventIngest".to_string(),
            Value::Bool(true),
        );
        map.insert("FFlagDebugDisableTelemetryPoint".to_string(), Value::Bool(true));
        map.insert(
            "FFlagDebugDisableTelemetryCounter".to_string(),
            Value::Bool(true),
        );
    }

    // Apply user custom flags last so explicit overrides win.
    for (k, v) in &cfg.custom_flags {
        let key = k.trim();
        if !key.is_empty() {
            map.insert(key.to_string(), v.clone());
        }
    }

    map
}

pub fn import_custom_flags_json(raw: &str, cfg: &mut FastFlagsConfig) -> Result<usize, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Paste a JSON object of FastFlags first.".to_string());
    }
    let val: Value = serde_json::from_str(trimmed)
        .map_err(|e| format!("Invalid JSON: {e}"))?;
    let Some(obj) = val.as_object() else {
        return Err("Expected a JSON object like {\"FFlagExample\": true}.".to_string());
    };
    let mut added = 0usize;
    for (k, v) in obj {
        let key = k.trim();
        if !key.is_empty() {
            cfg.custom_flags.insert(key.to_string(), v.clone());
            added += 1;
        }
    }
    if added == 0 {
        return Err("JSON object contained no flag keys.".to_string());
    }
    Ok(added)
}

pub fn export_effective_flags_json(cfg: &FastFlagsConfig, target_fps: Option<u32>) -> String {
    let map = build_client_app_settings(cfg, target_fps, None, &[]);
    serde_json::to_string_pretty(&Value::Object(map)).unwrap_or_else(|_| "{}".to_string())
}

pub fn sync_to_version_dir(
    version_dir: &Path,
    cfg: &FastFlagsConfig,
    target_fps: Option<u32>,
    removed_keys: &[&str],
) -> Result<(), String> {
    let cs_dir = version_dir.join("ClientSettings");
    let cs_file = cs_dir.join("ClientAppSettings.json");

    let existing = if let Ok(text) = fs::read_to_string(&cs_file) {
        serde_json::from_str::<Map<String, Value>>(&text).ok()
    } else {
        None
    };

    let updated = build_client_app_settings(cfg, target_fps, existing.as_ref(), removed_keys);
    if updated.is_empty() {
        if cs_file.exists() {
            let _ = fs::remove_file(&cs_file);
        }
    } else {
        fs::create_dir_all(&cs_dir).map_err(|e| e.to_string())?;
        let text = serde_json::to_string_pretty(&Value::Object(updated))
            .map_err(|e| e.to_string())?;
        fs::write(&cs_file, text).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn sync_client_fastflags(
    cfg: &FastFlagsConfig,
    target_fps: Option<u32>,
    removed_keys: &[&str],
) -> Result<(), String> {
    let player = find_player()
        .ok_or_else(|| "Roblox is not installed. Install it once from roblox.com first.".to_string())?;
    let version_dir = player
        .parent()
        .ok_or_else(|| "Could not determine Roblox version directory".to_string())?;
    sync_to_version_dir(version_dir, cfg, target_fps, removed_keys)
}

pub fn sync_saved_fastflags_before_launch() -> Result<(), String> {
    let cfg = load_fastflags();
    let settings = load_settings();
    sync_client_fastflags(&cfg, settings.target_fps, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_flag_value_types() {
        assert_eq!(parse_flag_value("true"), Value::Bool(true));
        assert_eq!(parse_flag_value(" FALSE "), Value::Bool(false));
        assert_eq!(parse_flag_value("240"), Value::from(240));
        assert_eq!(parse_flag_value("-5"), Value::from(-5));
        assert_eq!(parse_flag_value("hello"), Value::String("hello".to_string()));
        assert_eq!(
            parse_flag_value("\"quoted\""),
            Value::String("quoted".to_string())
        );
    }

    #[test]
    fn builds_and_removes_builtin_flags_while_keeping_unrelated() {
        let mut existing = Map::new();
        existing.insert("FStringThirdPartyFlag".to_string(), Value::from("keep"));
        existing.insert("FIntRenderShadowIntensity".to_string(), Value::from(0));

        let mut cfg = FastFlagsConfig::default();
        cfg.renderer = RendererMode::Vulkan;
        cfg.disable_telemetry = true;
        cfg.custom_flags
            .insert("FFlagCustomFlag".to_string(), Value::Bool(true));

        let out = build_client_app_settings(&cfg, Some(144), Some(&existing), &[]);
        assert_eq!(out.get("FStringThirdPartyFlag"), Some(&Value::from("keep")));
        assert_eq!(out.get("FIntRenderShadowIntensity"), None);
        assert_eq!(out.get("DFIntTaskSchedulerTargetFps"), Some(&Value::from(144)));
        assert_eq!(
            out.get("FFlagDebugGraphicsPreferVulkan"),
            Some(&Value::Bool(true))
        );
        assert_eq!(
            out.get("FFlagDebugDisableTelemetryV2"),
            Some(&Value::Bool(true))
        );
        assert_eq!(out.get("FFlagCustomFlag"), Some(&Value::Bool(true)));
    }

    #[test]
    fn import_custom_flags_json_merges_valid_object() {
        let mut cfg = FastFlagsConfig::default();
        let count = import_custom_flags_json(
            r#"{"FFlagA": true, "DFIntB": 120, "FStringC": "abc"}"#,
            &mut cfg,
        )
        .expect("import valid json");
        assert_eq!(count, 3);
        assert_eq!(cfg.custom_flags.get("DFIntB"), Some(&Value::from(120)));
    }
}
