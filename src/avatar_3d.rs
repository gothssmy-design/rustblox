// src/avatar_3d.rs
// Real-time 3D Roblox Avatar parser, loader, and GPU-accelerated egui renderer

use eframe::egui::{self, Color32, Painter, Pos2, Rect, Shape, Stroke, Vec2};
use serde_json::Value;
use std::io::Read;

/// Roblox CDN sharding algorithm: assets starting with 30DAY- or raw hex hashes
/// must be downloaded from t{sub}.rbxcdn.com where sub = (hash bytes XOR 31) % 8.
pub fn rbx_cdn_url(hash: &str) -> String {
    let mut acc: u32 = 31;
    for b in hash.bytes() {
        acc ^= b as u32;
    }
    let sub = acc % 8;
    format!("https://t{sub}.rbxcdn.com/{hash}")
}

/// Download raw asset bytes from Roblox CDN, automatically decompressing gzip if present.
pub fn download_rbx_asset(hash: &str) -> Result<Vec<u8>, String> {
    let url = rbx_cdn_url(hash);
    let resp = ureq::get(&url)
        .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .set("Referer", "https://www.roblox.com/")
        .call()
        .map_err(|e| format!("CDN fetch failed for {url}: {e}"))?;

    let mut bytes = Vec::new();
    resp.into_reader()
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Read error: {e}"))?;

    // Check for GZip magic header (0x1f 0x8b)
    if bytes.len() > 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        use flate2::read::GzDecoder;
        let mut gz = GzDecoder::new(&bytes[..]);
        let mut decompressed = Vec::new();
        gz.read_to_end(&mut decompressed)
            .map_err(|e| format!("Gzip decode failed: {e}"))?;
        Ok(decompressed)
    } else {
        Ok(bytes)
    }
}

/// Single 3D vertex with position, normal, UV coordinates, and color
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct AvatarVertex {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub color: Color32,
}

/// Single 3D triangle
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct AvatarTriangle {
    pub v: [AvatarVertex; 3],
    pub texture_index: Option<usize>,
    pub base_color: Color32,
    pub normal: [f32; 3],
}

/// Parsed 3D model with normalized coordinates
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Avatar3DModel {
    pub triangles: Vec<AvatarTriangle>,
    pub center: [f32; 3],
    pub scale: f32,
    pub texture_hashes: Vec<String>,
}

/// Loaded texture data ready for egui
pub struct LoadedTexture {
    pub hash: String,
    pub image: egui::ColorImage,
    pub handle: Option<egui::TextureHandle>,
}

/// State container for the 3D avatar viewport
pub struct Avatar3DState {
    pub is_3d: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub model: Option<Avatar3DModel>,
    pub textures: Vec<LoadedTexture>,
    pub loading: bool,
    pub err: Option<String>,
    pub loaded_user_id: Option<u64>,
}

const DEFAULT_YAW: f32 = 0.0;
const DEFAULT_PITCH: f32 = 0.05;

impl Default for Avatar3DState {
    fn default() -> Self {
        Self {
            is_3d: true,
            yaw: DEFAULT_YAW, // Character faces forward directly at camera
            pitch: DEFAULT_PITCH,
            zoom: 1.0,
            model: None,
            textures: Vec::new(),
            loading: false,
            err: None,
            loaded_user_id: None,
        }
    }
}

/// Material definition parsed from MTL file
struct MaterialDef {
    name: String,
    texture_hash: Option<String>,
    color: Color32,
}

/// Parse OBJ and MTL bytes into an Avatar3DModel
pub fn parse_avatar_3d(
    obj_bytes: &[u8],
    mtl_bytes: Option<&[u8]>,
    manifest_textures: &[String],
) -> Result<Avatar3DModel, String> {
    let obj_str = String::from_utf8_lossy(obj_bytes);

    // 1. Parse MTL if available
    let mut materials: Vec<MaterialDef> = Vec::new();
    if let Some(mb) = mtl_bytes {
        let mtl_str = String::from_utf8_lossy(mb);
        let mut cur_name = String::new();
        let mut cur_tex = None;
        let mut cur_col = Color32::from_rgb(210, 210, 215);

        for line in mtl_str.lines() {
            let line = line.trim();
            if line.starts_with("newmtl ") {
                if !cur_name.is_empty() {
                    materials.push(MaterialDef {
                        name: cur_name.clone(),
                        texture_hash: cur_tex.take(),
                        color: cur_col,
                    });
                }
                cur_name = line["newmtl ".len()..].trim().to_string();
                cur_col = Color32::from_rgb(210, 210, 215);
            } else if line.starts_with("Kd ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 4 {
                    let r = parts[1].parse::<f32>().unwrap_or(0.8);
                    let g = parts[2].parse::<f32>().unwrap_or(0.8);
                    let b = parts[3].parse::<f32>().unwrap_or(0.8);
                    cur_col = Color32::from_rgb(
                        (r.clamp(0.0, 1.0) * 255.0) as u8,
                        (g.clamp(0.0, 1.0) * 255.0) as u8,
                        (b.clamp(0.0, 1.0) * 255.0) as u8,
                    );
                }
            } else if line.starts_with("map_Kd ") || line.starts_with("map_Ka ") {
                let tex = line.split_whitespace().nth(1).unwrap_or("").trim();
                if !tex.is_empty() {
                    cur_tex = Some(tex.to_string());
                }
            }
        }
        if !cur_name.is_empty() {
            materials.push(MaterialDef {
                name: cur_name,
                texture_hash: cur_tex,
                color: cur_col,
            });
        }
    }

    // Build unique texture hash list
    let mut texture_hashes = manifest_textures.to_vec();
    for m in &materials {
        if let Some(h) = &m.texture_hash {
            if !texture_hashes.contains(h) {
                texture_hashes.push(h.clone());
            }
        }
    }

    // 2. Parse OBJ file
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut colors: Vec<Color32> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();

    let mut current_mtl_idx = 0usize;
    let mut triangles: Vec<AvatarTriangle> = Vec::new();

    let mut min_pos = [f32::MAX, f32::MAX, f32::MAX];
    let mut max_pos = [f32::MIN, f32::MIN, f32::MIN];

    for line in obj_str.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with("v ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let x = parts[1].parse::<f32>().unwrap_or(0.0);
                let y = parts[2].parse::<f32>().unwrap_or(0.0);
                let z = parts[3].parse::<f32>().unwrap_or(0.0);
                positions.push([x, y, z]);

                min_pos[0] = min_pos[0].min(x);
                min_pos[1] = min_pos[1].min(y);
                min_pos[2] = min_pos[2].min(z);
                max_pos[0] = max_pos[0].max(x);
                max_pos[1] = max_pos[1].max(y);
                max_pos[2] = max_pos[2].max(z);

                if parts.len() >= 7 {
                    let r = parts[4].parse::<f32>().unwrap_or(1.0);
                    let g = parts[5].parse::<f32>().unwrap_or(1.0);
                    let b = parts[6].parse::<f32>().unwrap_or(1.0);
                    colors.push(Color32::from_rgb(
                        (r.clamp(0.0, 1.0) * 255.0) as u8,
                        (g.clamp(0.0, 1.0) * 255.0) as u8,
                        (b.clamp(0.0, 1.0) * 255.0) as u8,
                    ));
                } else {
                    colors.push(Color32::WHITE);
                }
            }
        } else if line.starts_with("vt ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let u = parts[1].parse::<f32>().unwrap_or(0.0);
                let v = parts[2].parse::<f32>().unwrap_or(0.0);
                uvs.push([u, v]);
            }
        } else if line.starts_with("vn ") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let nx = parts[1].parse::<f32>().unwrap_or(0.0);
                let ny = parts[2].parse::<f32>().unwrap_or(0.0);
                let nz = parts[3].parse::<f32>().unwrap_or(0.0);
                normals.push([nx, ny, nz]);
            }
        } else if line.starts_with("usemtl ") {
            let mtl_name = line["usemtl ".len()..].trim();
            if let Some(pos) = materials.iter().position(|m| m.name == mtl_name) {
                current_mtl_idx = pos;
            }
        } else if line.starts_with("f ") {
            let face_tokens: Vec<&str> = line.split_whitespace().skip(1).collect();
            if face_tokens.len() < 3 {
                continue;
            }

            let parse_vertex = |tok: &str| -> AvatarVertex {
                let sub: Vec<&str> = tok.split('/').collect();
                let v_idx = sub.get(0).and_then(|s| s.parse::<usize>().ok()).unwrap_or(1);
                let vt_idx = sub.get(1).and_then(|s| s.parse::<usize>().ok());
                let vn_idx = sub.get(2).and_then(|s| s.parse::<usize>().ok());

                let pos = if v_idx > 0 && v_idx <= positions.len() {
                    positions[v_idx - 1]
                } else {
                    [0.0, 0.0, 0.0]
                };

                let col = if v_idx > 0 && v_idx <= colors.len() {
                    colors[v_idx - 1]
                } else {
                    Color32::WHITE
                };

                let uv = if let Some(idx) = vt_idx {
                    if idx > 0 && idx <= uvs.len() {
                        uvs[idx - 1]
                    } else {
                        [0.0, 0.0]
                    }
                } else {
                    [0.0, 0.0]
                };

                let norm = if let Some(idx) = vn_idx {
                    if idx > 0 && idx <= normals.len() {
                        normals[idx - 1]
                    } else {
                        [0.0, 0.0, 0.0]
                    }
                } else {
                    [0.0, 0.0, 0.0]
                };

                AvatarVertex {
                    pos,
                    normal: norm,
                    uv,
                    color: col,
                }
            };

            let v0 = parse_vertex(face_tokens[0]);
            let (tex_idx, base_col) = if let Some(m) = materials.get(current_mtl_idx) {
                let tidx = m
                    .texture_hash
                    .as_ref()
                    .and_then(|h| texture_hashes.iter().position(|th| th == h));
                (tidx, m.color)
            } else {
                (None, Color32::from_rgb(220, 220, 225))
            };

            for i in 1..face_tokens.len() - 1 {
                let mut v0_tri = v0.clone();
                let mut v1 = parse_vertex(face_tokens[i]);
                let mut v2 = parse_vertex(face_tokens[i + 1]);

                // Calculate face normal
                let edge1 = [
                    v1.pos[0] - v0_tri.pos[0],
                    v1.pos[1] - v0_tri.pos[1],
                    v1.pos[2] - v0_tri.pos[2],
                ];
                let edge2 = [
                    v2.pos[0] - v0_tri.pos[0],
                    v2.pos[1] - v0_tri.pos[1],
                    v2.pos[2] - v0_tri.pos[2],
                ];
                let mut fnorm = [
                    edge1[1] * edge2[2] - edge1[2] * edge2[1],
                    edge1[2] * edge2[0] - edge1[0] * edge2[2],
                    edge1[0] * edge2[1] - edge1[1] * edge2[0],
                ];
                let len = (fnorm[0] * fnorm[0] + fnorm[1] * fnorm[1] + fnorm[2] * fnorm[2]).sqrt();
                if len > 1e-6 {
                    fnorm[0] /= len;
                    fnorm[1] /= len;
                    fnorm[2] /= len;
                }

                for vert in [&mut v0_tri, &mut v1, &mut v2] {
                    let nlen2 = vert.normal[0] * vert.normal[0]
                        + vert.normal[1] * vert.normal[1]
                        + vert.normal[2] * vert.normal[2];
                    if nlen2 <= 1e-6 {
                        vert.normal = fnorm;
                    }
                }

                triangles.push(AvatarTriangle {
                    v: [v0_tri, v1, v2],
                    texture_index: tex_idx,
                    base_color: base_col,
                    normal: fnorm,
                });
            }
        }
    }

    if positions.is_empty() || triangles.is_empty() {
        return Err("No vertices or triangles found in avatar OBJ".to_string());
    }

    let center = [
        (min_pos[0] + max_pos[0]) * 0.5,
        (min_pos[1] + max_pos[1]) * 0.5,
        (min_pos[2] + max_pos[2]) * 0.5,
    ];

    let extent_y = (max_pos[1] - min_pos[1]).max(1.0);
    let scale = 2.0 / extent_y;

    Ok(Avatar3DModel {
        triangles,
        center,
        scale,
        texture_hashes,
    })
}

/// Fetch avatar 3D data from Roblox thumbnails API and CDN
pub fn load_avatar_3d_data(
    user_id: u64,
    cookie: Option<&str>,
) -> Result<(Avatar3DModel, Vec<(String, Vec<u8>)>), String> {
    let url = format!("https://thumbnails.roblox.com/v1/users/avatar-3d?userId={user_id}");
    let mut req = ureq::get(&url)
        .set("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .set("Referer", "https://www.roblox.com/");
    if let Some(c) = cookie {
        req = req.set("Cookie", &format!(".ROBLOSECURITY={c}"));
    }
    let resp = req
        .call()
        .map_err(|e| format!("avatar-3d API call error: {e}"))?;
    let json: Value = resp
        .into_json()
        .map_err(|e| format!("manifest json error: {e}"))?;

    let image_url = json["imageUrl"].as_str().ok_or_else(|| {
        let state = json["state"].as_str().unwrap_or("unknown");
        format!("Avatar 3D thumbnail status: {state}")
    })?;

    // Download manifest JSON from CDN
    let manifest_resp = ureq::get(image_url)
        .set("User-Agent", "Mozilla/5.0")
        .call()
        .map_err(|e| format!("Manifest download error: {e}"))?;
    let manifest: Value = manifest_resp
        .into_json()
        .map_err(|e| format!("Manifest parse error: {e}"))?;

    let obj_hash = manifest["obj"]
        .as_str()
        .ok_or_else(|| "No obj hash in manifest".to_string())?;
    let mtl_hash = manifest["mtl"].as_str();

    let mut tex_hashes = Vec::new();
    if let Some(arr) = manifest["textures"].as_array() {
        for t in arr {
            if let Some(h) = t.as_str() {
                tex_hashes.push(h.to_string());
            }
        }
    }

    // Download OBJ bytes
    let obj_bytes = download_rbx_asset(obj_hash)?;

    // Download MTL bytes if available
    let mtl_bytes = if let Some(mh) = mtl_hash {
        download_rbx_asset(mh).ok()
    } else {
        None
    };

    // Download texture bytes
    let mut raw_textures = Vec::new();
    for th in &tex_hashes {
        if let Ok(bytes) = download_rbx_asset(th) {
            raw_textures.push((th.clone(), bytes));
        }
    }

    let model = parse_avatar_3d(&obj_bytes, mtl_bytes.as_deref(), &tex_hashes)?;

    Ok((model, raw_textures))
}

/// Transformed triangle with depth for Painter's algorithm sorting
struct TransformedTriangle<'a> {
    p0: Pos2,
    p1: Pos2,
    p2: Pos2,
    uv0: Pos2,
    uv1: Pos2,
    uv2: Pos2,
    col0: Color32,
    col1: Color32,
    col2: Color32,
    avg_z: f32,
    tex_idx: Option<usize>,
    _tri: &'a AvatarTriangle,
}

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 1e-6 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 0.0, -1.0]
    }
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn rotate_normal(n: [f32; 3], cos_y: f32, sin_y: f32, cos_p: f32, sin_p: f32) -> [f32; 3] {
    let nx = n[0] * cos_y + n[2] * sin_y;
    let ny_pre = n[1];
    let nz_pre = -n[0] * sin_y + n[2] * cos_y;
    let rny = ny_pre * cos_p - nz_pre * sin_p;
    let rnz = ny_pre * sin_p + nz_pre * cos_p;
    normalize3([nx, rny, rnz])
}

/// Three-point studio lighting (warm key + cool fill + Fresnel rim + hemisphere AO).
fn shade_studio_vertex(
    view_pos: [f32; 3],
    raw_normal: [f32; 3],
    face_normal: [f32; 3],
    cos_y: f32,
    sin_y: f32,
    cos_p: f32,
    sin_p: f32,
    untextured_base: Option<Color32>,
) -> Color32 {
    let n_src = if dot3(raw_normal, raw_normal) > 1e-6 {
        raw_normal
    } else {
        face_normal
    };
    let n = rotate_normal(n_src, cos_y, sin_y, cos_p, sin_p);
    let v = normalize3([-view_pos[0], -view_pos[1], -(4.8 + view_pos[2])]);

    // Vectors point from surface toward studio lights (camera sits on -Z).
    let l_key = normalize3([-0.46, 0.68, -0.72]);
    let l_fill = normalize3([0.78, 0.14, -0.50]);
    let l_rim = normalize3([0.44, 0.56, 0.70]);

    // Key light: soft-wrapped diffuse + Blinn-Phong studio softbox specular
    let diff_key = ((dot3(n, l_key) + 0.24) / 1.24).clamp(0.0, 1.0).powf(1.12);
    let h_key = normalize3([l_key[0] + v[0], l_key[1] + v[1], l_key[2] + v[2]]);
    let spec = dot3(n, h_key).max(0.0).powf(16.0) * 0.26;

    // Cool opposite fill light
    let diff_fill = ((dot3(n, l_fill) + 0.34) / 1.34).clamp(0.0, 1.0);

    // Fresnel rim highlight along silhouette edges
    let ndotv = dot3(n, v).clamp(0.0, 1.0);
    let fresnel = (1.0 - ndotv).powf(2.4);
    let rim_gate = ((dot3(n, l_rim) + 0.40) / 1.40).clamp(0.0, 1.0);
    let rim = fresnel * (0.16 + 0.34 * rim_gate);

    // Hemisphere ambient (studio ceiling vs floor bounce) + subtle vertical grounding AO
    let hemi = (n[1] * 0.5 + 0.5).clamp(0.0, 1.0);
    let amb_rgb = [
        0.24 + (0.37 - 0.24) * hemi,
        0.25 + (0.39 - 0.25) * hemi,
        0.30 + (0.44 - 0.30) * hemi,
    ];
    let ao = 0.86 + 0.14 * ((view_pos[1] + 1.0) * 0.5).clamp(0.0, 1.0);

    let key_tint = [1.04f32, 0.99f32, 0.92f32];
    let fill_tint = [0.82f32, 0.91f32, 1.08f32];
    let rim_tint = [0.92f32, 0.96f32, 1.06f32];

    let mut lr = amb_rgb[0] * ao + (0.62 * diff_key * key_tint[0]) + (0.24 * diff_fill * fill_tint[0]);
    let mut lg = amb_rgb[1] * ao + (0.62 * diff_key * key_tint[1]) + (0.24 * diff_fill * fill_tint[1]);
    let mut lb = amb_rgb[2] * ao + (0.62 * diff_key * key_tint[2]) + (0.24 * diff_fill * fill_tint[2]);

    let sr = spec * key_tint[0] + rim * rim_tint[0];
    let sg = spec * key_tint[1] + rim * rim_tint[1];
    let sb = spec * key_tint[2] + rim * rim_tint[2];

    if let Some(base) = untextured_base {
        let br = base.r() as f32 / 255.0;
        let bg = base.g() as f32 / 255.0;
        let bb = base.b() as f32 / 255.0;
        Color32::from_rgba_unmultiplied(
            ((br * lr + sr * 0.72).clamp(0.0, 1.0) * 255.0) as u8,
            ((bg * lg + sg * 0.72).clamp(0.0, 1.0) * 255.0) as u8,
            ((bb * lb + sb * 0.72).clamp(0.0, 1.0) * 255.0) as u8,
            base.a(),
        )
    } else {
        lr = (lr + sr).clamp(0.0, 1.0);
        lg = (lg + sg).clamp(0.0, 1.0);
        lb = (lb + sb).clamp(0.0, 1.0);
        Color32::from_rgba_unmultiplied(
            (lr * 255.0) as u8,
            (lg * 255.0) as u8,
            (lb * 255.0) as u8,
            255,
        )
    }
}

/// Render the 3D model into an egui Painter with GPU acceleration
pub fn render_avatar_3d_mesh(
    painter: &Painter,
    model: &Avatar3DModel,
    textures: &[LoadedTexture],
    yaw: f32,
    pitch: f32,
    zoom: f32,
    viewport_rect: Rect,
) {
    let center = viewport_rect.center();
    let aspect_scale = viewport_rect.height() * 0.49 * zoom;

    let cos_y = yaw.cos();
    let sin_y = yaw.sin();
    let cos_p = pitch.cos();
    let sin_p = pitch.sin();

    // Transform vertex
    let transform_pt = |p: [f32; 3]| -> ([f32; 3], Pos2, f32) {
        // Center and scale to [-1, 1]
        let px = (p[0] - model.center[0]) * model.scale;
        let py = (p[1] - model.center[1]) * model.scale;
        let pz = (p[2] - model.center[2]) * model.scale;

        // Rotate yaw around Y
        let rx = px * cos_y + pz * sin_y;
        let ry = py;
        let rz = -px * sin_y + pz * cos_y;

        // Rotate pitch around X
        let fx = rx;
        let fy = ry * cos_p - rz * sin_p;
        let fz = ry * sin_p + rz * cos_p;

        // Telephoto perspective distance matching Roblox avatar studio
        let dist = 4.8f32;
        let w = dist / (dist + fz);

        let sx = center.x + fx * w * aspect_scale;
        let sy = center.y - fy * w * aspect_scale + (viewport_rect.height() * 0.02);

        ([fx, fy, fz], Pos2::new(sx, sy), fz)
    };

    let mut projected: Vec<TransformedTriangle> = Vec::with_capacity(model.triangles.len());

    for tri in &model.triangles {
        let (t0, s0, z0) = transform_pt(tri.v[0].pos);
        let (t1, s1, z1) = transform_pt(tri.v[1].pos);
        let (t2, s2, z2) = transform_pt(tri.v[2].pos);

        // 2D Cross product backface culling
        let cross_2d = (s1.x - s0.x) * (s2.y - s0.y) - (s1.y - s0.y) * (s2.x - s0.x);
        if cross_2d <= 0.0 {
            continue; // Backface culled
        }

        let has_tex = tri
            .texture_index
            .and_then(|idx| textures.get(idx))
            .and_then(|t| t.handle.as_ref())
            .is_some();

        let vert_col_for = |vert: &AvatarVertex, vp: [f32; 3]| -> Color32 {
            let base = if has_tex {
                None
            } else if vert.color != Color32::WHITE {
                Some(vert.color)
            } else {
                Some(tri.base_color)
            };
            shade_studio_vertex(vp, vert.normal, tri.normal, cos_y, sin_y, cos_p, sin_p, base)
        };

        let col0 = vert_col_for(&tri.v[0], t0);
        let col1 = vert_col_for(&tri.v[1], t1);
        let col2 = vert_col_for(&tri.v[2], t2);

        // OBJ UV v is flipped relative to egui UV space
        let uv0 = Pos2::new(tri.v[0].uv[0], 1.0 - tri.v[0].uv[1]);
        let uv1 = Pos2::new(tri.v[1].uv[0], 1.0 - tri.v[1].uv[1]);
        let uv2 = Pos2::new(tri.v[2].uv[0], 1.0 - tri.v[2].uv[1]);

        let avg_z = (z0 + z1 + z2) / 3.0;

        projected.push(TransformedTriangle {
            p0: s0,
            p1: s1,
            p2: s2,
            uv0,
            uv1,
            uv2,
            col0,
            col1,
            col2,
            avg_z,
            tex_idx: tri.texture_index,
            _tri: tri,
        });
    }

    // Sort back-to-front (Painter's algorithm: higher Z is farther away)
    projected.sort_by(|a, b| {
        b.avg_z
            .partial_cmp(&a.avg_z)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Group triangles into textured egui Meshes
    // Pre-allocate meshes for each texture slot + 1 untextured mesh
    let mut mesh_tex: Vec<egui::Mesh> = Vec::new();
    for tex in textures {
        if let Some(h) = &tex.handle {
            mesh_tex.push(egui::Mesh::with_texture(h.id()));
        } else {
            mesh_tex.push(egui::Mesh::default());
        }
    }
    let mut mesh_default = egui::Mesh::default();

    for t in projected {
        let (is_textured, target_mesh) = if let Some(idx) = t.tex_idx {
            if idx < mesh_tex.len() && textures[idx].handle.is_some() {
                (true, &mut mesh_tex[idx])
            } else {
                (false, &mut mesh_default)
            }
        } else {
            (false, &mut mesh_default)
        };

        // For untextured triangles, always sample the solid white pixel in font atlas
        // Passing non-white UVs to mesh_default causes font atlas sampling (streaks/black faces)
        let (uv0, uv1, uv2) = if is_textured {
            (t.uv0, t.uv1, t.uv2)
        } else {
            (egui::epaint::WHITE_UV, egui::epaint::WHITE_UV, egui::epaint::WHITE_UV)
        };

        let base_idx = target_mesh.vertices.len() as u32;
        target_mesh.vertices.push(egui::epaint::Vertex {
            pos: t.p0,
            uv: uv0,
            color: t.col0,
        });
        target_mesh.vertices.push(egui::epaint::Vertex {
            pos: t.p1,
            uv: uv1,
            color: t.col1,
        });
        target_mesh.vertices.push(egui::epaint::Vertex {
            pos: t.p2,
            uv: uv2,
            color: t.col2,
        });
        target_mesh.indices.push(base_idx);
        target_mesh.indices.push(base_idx + 1);
        target_mesh.indices.push(base_idx + 2);
    }

    // Render meshes to painter
    for m in mesh_tex {
        if !m.indices.is_empty() {
            painter.add(Shape::Mesh(m.into()));
        }
    }
    if !mesh_default.indices.is_empty() {
        painter.add(Shape::Mesh(mesh_default.into()));
    }
}

/// Render the complete avatar preview viewport with studio lighting, floor shadow,
/// 2D/3D mode switch, camera reset, and click-and-drag 3D rotation grab.
pub fn render_avatar_viewport(
    ui: &mut egui::Ui,
    state: &mut Avatar3DState,
    avatar_2d_url: Option<&str>,
    viewport_size: Vec2,
    dark_mode: bool,
) {
    let (rect, resp) = ui.allocate_exact_size(viewport_size, egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    // 1. Studio backdrop gradient
    let top_col = if dark_mode {
        Color32::from_rgb(28, 33, 44)
    } else {
        // Soft pastel studio warm glow matching example photo
        Color32::from_rgb(246, 243, 232)
    };
    let bot_col = if dark_mode {
        Color32::from_rgb(14, 17, 24)
    } else {
        Color32::from_rgb(230, 233, 242)
    };

    let stroke = Stroke::new(
        1.0,
        if dark_mode {
            Color32::from_rgb(45, 52, 68)
        } else {
            Color32::from_rgb(215, 220, 230)
        },
    );

    // Rounded studio viewport frame
    painter.rect_filled(rect, 14.0, bot_col);
    painter.rect_stroke(rect, 14.0, stroke, egui::StrokeKind::Inside);

    // Vertical gradient backdrop
    let mut bg_mesh = egui::Mesh::default();
    bg_mesh.add_rect_with_uv(
        rect.shrink(1.0),
        Rect::from_min_max(egui::epaint::WHITE_UV, egui::epaint::WHITE_UV),
        Color32::WHITE,
    );
    // Tint vertices for studio gradient
    if bg_mesh.vertices.len() >= 4 {
        bg_mesh.vertices[0].color = top_col;
        bg_mesh.vertices[1].color = top_col;
        bg_mesh.vertices[2].color = bot_col;
        bg_mesh.vertices[3].color = bot_col;
    }
    painter.add(Shape::Mesh(bg_mesh.into()));

    // 2. Soft floor contact shadow under avatar feet
    let shadow_center = Pos2::new(rect.center().x, rect.bottom() - 32.0);
    let shadow_radius = Vec2::new(viewport_size.x * 0.38, 14.0);
    painter.add(Shape::ellipse_filled(
        shadow_center,
        shadow_radius,
        if dark_mode {
            Color32::from_black_alpha(80)
        } else {
            Color32::from_black_alpha(35)
        },
    ));

    // 3. Render 3D Model or 2D image
    let has_3d_model = state.model.is_some();

    // Prepare textures if needed
    for tex in &mut state.textures {
        if tex.handle.is_none() {
            tex.handle = Some(ui.ctx().load_texture(
                format!("rbx_avatar_tex_{}", tex.hash),
                tex.image.clone(),
                egui::TextureOptions::LINEAR,
            ));
        }
    }

    if state.is_3d && has_3d_model {
        let model = state.model.as_ref().unwrap();
        render_avatar_3d_mesh(
            &painter,
            model,
            &state.textures,
            state.yaw,
            state.pitch,
            state.zoom,
            rect,
        );
    } else {
        // Fallback to high-resolution 2D avatar preview
        if let Some(url) = avatar_2d_url {
            let img_size = Vec2::new(viewport_size.x - 16.0, viewport_size.y - 24.0);
            let img_rect = Rect::from_center_size(
                Pos2::new(rect.center().x, rect.center().y - 6.0),
                img_size,
            );
            let img = egui::Image::new(url)
                .fit_to_exact_size(img_size)
                .corner_radius(10);
            img.paint_at(ui, img_rect);
        }
    }

    // 4. Mouse Grab & Drag interaction
    if resp.dragged() {
        let delta = resp.drag_delta();
        state.yaw -= delta.x * 0.015;
        state.pitch = (state.pitch + delta.y * 0.015).clamp(-0.75, 0.75);
        // Automatically switch to 3D mode if dragged while in 2D and 3D model is loaded
        if !state.is_3d && has_3d_model {
            state.is_3d = true;
        }
    }

    if resp.hovered() {
        ui.ctx().set_cursor_icon(if resp.dragged() {
            egui::CursorIcon::Grabbing
        } else {
            egui::CursorIcon::Grab
        });

        // Zoom via scroll wheel
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.1 {
            state.zoom = (state.zoom * (1.0 + scroll * 0.0015)).clamp(0.65, 2.2);
        }
    }

    if resp.double_clicked() {
        state.yaw = DEFAULT_YAW;
        state.pitch = DEFAULT_PITCH;
        state.zoom = 1.0;
    }

    // 5. Floating Controls Overlay in top-left (Matching example photo)
    let overlay_pos = Pos2::new(rect.left() + 12.0, rect.top() + 12.0);
    let mut ui_builder = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(Rect::from_min_size(overlay_pos, Vec2::new(120.0, 160.0))),
    );

    ui_builder.vertical(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(0.0, 6.0);

        // 2D / 3D pill button group
        egui::Frame::new()
            .fill(if dark_mode {
                Color32::from_rgba_unmultiplied(18, 22, 30, 220)
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, 220)
            })
            .stroke(Stroke::new(
                1.0,
                if dark_mode { Color32::from_rgb(50, 58, 76) } else { Color32::from_rgb(205, 212, 224) },
            ))
            .corner_radius(16)
            .inner_margin(3)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;

                    // 2D toggle button
                    let is_2d = !state.is_3d;
                    let btn_2d = egui::Button::new(
                        egui::RichText::new("2D")
                            .size(11.5)
                            .strong()
                            .color(if is_2d {
                                Color32::WHITE
                            } else if dark_mode {
                                Color32::from_rgb(160, 170, 190)
                            } else {
                                Color32::from_rgb(80, 90, 110)
                            }),
                    )
                    .fill(if is_2d {
                        Color32::from_rgb(0, 180, 105)
                    } else {
                        Color32::TRANSPARENT
                    })
                    .corner_radius(12)
                    .min_size(Vec2::new(32.0, 24.0));
                    if ui.add(btn_2d).clicked() {
                        state.is_3d = false;
                    }

                    // 3D toggle button
                    let is_3d = state.is_3d;
                    let btn_3d = egui::Button::new(
                        egui::RichText::new("3D")
                            .size(11.5)
                            .strong()
                            .color(if is_3d {
                                Color32::WHITE
                            } else if dark_mode {
                                Color32::from_rgb(160, 170, 190)
                            } else {
                                Color32::from_rgb(80, 90, 110)
                            }),
                    )
                    .fill(if is_3d {
                        Color32::from_rgb(0, 180, 105)
                    } else {
                        Color32::TRANSPARENT
                    })
                    .corner_radius(12)
                    .min_size(Vec2::new(32.0, 24.0));
                    if ui.add(btn_3d).clicked() {
                        state.is_3d = true;
                    }
                });
            });

        // Reset camera button ↺
        let reset_btn = egui::Button::new(
            egui::RichText::new("↺")
                .size(15.0)
                .strong()
                .color(if dark_mode {
                    Color32::from_rgb(210, 220, 240)
                } else {
                    Color32::from_rgb(60, 70, 90)
                }),
        )
        .fill(if dark_mode {
            Color32::from_rgba_unmultiplied(18, 22, 30, 220)
        } else {
            Color32::from_rgba_unmultiplied(255, 255, 255, 220)
        })
        .stroke(Stroke::new(
            1.0,
            if dark_mode { Color32::from_rgb(50, 58, 76) } else { Color32::from_rgb(205, 212, 224) },
        ))
        .corner_radius(14)
        .min_size(Vec2::new(28.0, 28.0));

        if ui.add(reset_btn).on_hover_text("Reset Camera (Yaw & Pitch)").clicked() {
            state.yaw = DEFAULT_YAW;
            state.pitch = DEFAULT_PITCH;
            state.zoom = 1.0;
        }
    });

    // 6. Loading indicator or status overlay in top-right
    if state.loading {
        let loading_pos = Pos2::new(rect.right() - 84.0, rect.top() + 12.0);
        let mut ui_loading = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_min_size(loading_pos, Vec2::new(72.0, 26.0))),
        );
        egui::Frame::new()
            .fill(if dark_mode {
                Color32::from_rgba_unmultiplied(18, 22, 30, 220)
            } else {
                Color32::from_rgba_unmultiplied(255, 255, 255, 220)
            })
            .corner_radius(12)
            .inner_margin(4)
            .show(&mut ui_loading, |ui| {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(egui::RichText::new("3D").size(11.0).strong());
                });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::{shade_studio_vertex, Avatar3DState, DEFAULT_PITCH, DEFAULT_YAW};
    use eframe::egui::Color32;

    #[test]
    fn default_camera_faces_front_and_studio_lighting_has_contrast() {
        let st = Avatar3DState::default();
        assert_eq!(st.yaw, DEFAULT_YAW);
        assert_eq!(st.pitch, DEFAULT_PITCH);

        let key_lit = shade_studio_vertex(
            [0.0, 0.2, -0.3],
            [-0.3, 0.5, -0.8],
            [-0.3, 0.5, -0.8],
            1.0,
            0.0,
            1.0,
            0.0,
            None,
        );
        let shadow_side = shade_studio_vertex(
            [0.3, -0.2, 0.1],
            [0.85, -0.45, 0.25],
            [0.85, -0.45, 0.25],
            1.0,
            0.0,
            1.0,
            0.0,
            None,
        );
        assert!(key_lit.r() > shadow_side.r() + 45);

        let dark_lit = shade_studio_vertex(
            [-0.2, 0.4, -0.4],
            [-0.22, 0.34, -0.91],
            [-0.22, 0.34, -0.91],
            1.0,
            0.0,
            1.0,
            0.0,
            Some(Color32::from_rgb(20, 20, 22)),
        );
        assert!(dark_lit.r() > 50);
    }
}
