use std::io::Cursor;
use eframe::egui::ColorImage;
use matroska_demuxer::{Frame, MatroskaFile, TrackType};
use vp8_decoder::Decoder;

#[derive(Clone)]
pub struct VideoFrame {
    pub timestamp_ms: u64,
    pub image: ColorImage,
}

#[allow(dead_code)]
pub struct DecodedPreviewVideo {
    pub frames: Vec<VideoFrame>,
    pub duration_ms: u64,
    pub width: usize,
    pub height: usize,
}

impl DecodedPreviewVideo {
    pub fn frame_at(&self, elapsed_ms: u64) -> Option<&VideoFrame> {
        if self.frames.is_empty() {
            return None;
        }
        if self.duration_ms == 0 {
            return self.frames.first();
        }
        let pos = elapsed_ms % self.duration_ms;
        let idx = self.frames.partition_point(|f| f.timestamp_ms <= pos);
        if idx == 0 {
            self.frames.first()
        } else {
            self.frames.get(idx - 1)
        }
    }
}

pub fn yuv_to_rgba_downscaled(
    pic: &vp8_decoder::Picture<'_>,
    max_w: usize,
) -> (usize, usize, Vec<u8>) {
    let src_w = pic.width as usize;
    let src_h = pic.height as usize;
    if src_w == 0 || src_h == 0 {
        return (0, 0, Vec::new());
    }

    let (dst_w, dst_h) = if src_w > max_w {
        let w = max_w;
        let h = ((src_h as f32 * (w as f32 / src_w as f32)).round() as usize).max(1);
        (w, h)
    } else {
        (src_w, src_h)
    };

    let mut rgba = vec![255u8; dst_w * dst_h * 4];

    for dy in 0..dst_h {
        let sy = (dy * src_h) / dst_h;
        let sy_uv = sy / 2;
        let row_offset = dy * dst_w * 4;

        for dx in 0..dst_w {
            let sx = (dx * src_w) / dst_w;
            let sx_uv = sx / 2;

            let y_idx = sy * pic.y_stride + sx;
            let uv_idx = sy_uv * pic.uv_stride + sx_uv;

            let y_val = if y_idx < pic.y.len() { pic.y[y_idx] as i32 } else { 0 };
            let u_val = if uv_idx < pic.u.len() { pic.u[uv_idx] as i32 - 128 } else { 0 };
            let v_val = if uv_idx < pic.v.len() { pic.v[uv_idx] as i32 - 128 } else { 0 };

            // Full-range YUV -> RGB
            let r = (y_val + ((359 * v_val) >> 8)).clamp(0, 255) as u8;
            let g = (y_val - ((88 * u_val + 183 * v_val) >> 8)).clamp(0, 255) as u8;
            let b = (y_val + ((454 * u_val) >> 8)).clamp(0, 255) as u8;

            let px_offset = row_offset + dx * 4;
            rgba[px_offset] = r;
            rgba[px_offset + 1] = g;
            rgba[px_offset + 2] = b;
            rgba[px_offset + 3] = 255;
        }
    }

    (dst_w, dst_h, rgba)
}

pub fn decode_webm_vp8(webm_bytes: &[u8], max_width: usize) -> Result<DecodedPreviewVideo, String> {
    if webm_bytes.is_empty() {
        return Err("Empty WebM bytes".to_string());
    }

    let cursor = Cursor::new(webm_bytes);
    let mut matroska = MatroskaFile::open(cursor)
        .map_err(|e| format!("Could not demux WebM: {e}"))?;

    let video_track = matroska
        .tracks()
        .iter()
        .find(|t| t.track_type() == TrackType::Video)
        .map(|t| t.track_number().get())
        .ok_or_else(|| "No video track found in WebM container".to_string())?;

    let mut decoder = Decoder::new();
    let mut raw_frame = Frame::default();

    let mut frames = Vec::new();
    let mut last_saved_ts = None;
    let min_frame_interval_ms = 45; // ~22 fps max to save memory and CPU
    let max_frames = 120; // ~6-8 seconds max preview

    let mut out_w = 0;
    let mut out_h = 0;

    while let Ok(true) = matroska.next_frame(&mut raw_frame) {
        if raw_frame.track != video_track {
            continue;
        }

        let ts_ms = raw_frame.timestamp / 1_000_000;
        if let Some(last_ts) = last_saved_ts {
            if ts_ms.saturating_sub(last_ts) < min_frame_interval_ms && frames.len() < max_frames {
                // Still decode to keep VP8 reference frame state correct, but don't store picture
                let _ = decoder.decode(&raw_frame.data);
                continue;
            }
        }

        match decoder.decode(&raw_frame.data) {
            Ok(Some(picture)) => {
                let (w, h, rgba) = yuv_to_rgba_downscaled(&picture, max_width);
                if w > 0 && h > 0 {
                    out_w = w;
                    out_h = h;
                    let color_img = ColorImage::from_rgba_unmultiplied([w, h], &rgba);
                    frames.push(VideoFrame {
                        timestamp_ms: ts_ms,
                        image: color_img,
                    });
                    last_saved_ts = Some(ts_ms);
                    if frames.len() >= max_frames {
                        break;
                    }
                }
            }
            Ok(None) => {}
            Err(_) => {
                // Skip corrupted/unsupported packet without crashing
            }
        }
    }

    if frames.is_empty() {
        return Err("No video frames could be decoded from WebM".to_string());
    }

    let first_ts = frames.first().map(|f| f.timestamp_ms).unwrap_or(0);
    // Normalize timestamps to start at 0
    if first_ts > 0 {
        for f in &mut frames {
            f.timestamp_ms = f.timestamp_ms.saturating_sub(first_ts);
        }
    }

    let last_ts = frames.last().map(|f| f.timestamp_ms).unwrap_or(0);
    let duration_ms = (last_ts + 66).max(500);

    Ok(DecodedPreviewVideo {
        frames,
        duration_ms,
        width: out_w,
        height: out_h,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::Color32;

    #[test]
    fn test_yuv_to_rgba_downscaled() {
        let empty_pic = vp8_decoder::Picture {
            width: 0,
            height: 0,
            y: &[],
            u: &[],
            v: &[],
            y_stride: 0,
            uv_stride: 0,
        };
        let (w, h, rgba) = yuv_to_rgba_downscaled(&empty_pic, 320);
        assert_eq!(w, 0);
        assert_eq!(h, 0);
        assert!(rgba.is_empty());
    }

    #[test]
    fn test_empty_webm_decode() {
        let res = decode_webm_vp8(&[], 320);
        assert!(res.is_err());
    }

    #[test]
    fn test_frame_at_lookup() {
        let frame1 = VideoFrame {
            timestamp_ms: 0,
            image: ColorImage::new([1, 1], vec![Color32::BLACK]),
        };
        let frame2 = VideoFrame {
            timestamp_ms: 500,
            image: ColorImage::new([1, 1], vec![Color32::WHITE]),
        };
        let video = DecodedPreviewVideo {
            frames: vec![frame1, frame2],
            duration_ms: 1000,
            width: 1,
            height: 1,
        };

        assert_eq!(video.frame_at(0).unwrap().timestamp_ms, 0);
        assert_eq!(video.frame_at(250).unwrap().timestamp_ms, 0);
        assert_eq!(video.frame_at(500).unwrap().timestamp_ms, 500);
        assert_eq!(video.frame_at(800).unwrap().timestamp_ms, 500);
        // Loops after 1000ms:
        assert_eq!(video.frame_at(1250).unwrap().timestamp_ms, 0);
    }
}
