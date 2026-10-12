use ab_glyph::{Font, FontVec, Glyph, PxScale, ScaleFont, point};

/// Pixel format: little-endian `0xFFRRGGBB` (softbuffer).
pub const BG: u32 = 0xFF1E1E1E;
pub const FG: u32 = 0xFF00E6E6;

const FONT_DIR: &str = r"C:\Windows\Fonts";
const MONO_CHAIN: [&str; 2] = [r"consola.ttf", r"lucon.ttf"];
const CJK_CHAIN: [&str; 3] = [r"msyh.ttc", r"msyh.ttf", r"simhei.ttf"];

pub struct Fonts {
    pub mono: FontVec,
    pub cjk: FontVec,
}

/// Try each candidate path in order (absolute paths as-is, others under
/// FONT_DIR); first loadable wins.
pub fn load_font(candidates: &[&str]) -> Option<FontVec> {
    for rel in candidates {
        let path = if std::path::Path::new(rel).is_absolute() {
            rel.to_string()
        } else {
            format!("{FONT_DIR}\\{rel}")
        };
        let Ok(data) = std::fs::read(path) else {
            continue;
        };
        if let Ok(font) = FontVec::try_from_vec_and_index(data, 0) {
            return Some(font);
        }
    }
    None
}

/// Load both font families with their fallback chains (spec D2).
/// Returns `None` when either chain fails completely.
pub fn load_fonts() -> Option<Fonts> {
    Some(Fonts {
        mono: load_font(&MONO_CHAIN)?,
        cjk: load_font(&CJK_CHAIN)?,
    })
}

/// (width, height) of `text` rendered at `px` pixel size.
pub fn measure(text: &str, font: &FontVec, px: f32) -> (f32, f32) {
    let scaled = font.as_scaled(px);
    let mut width = 0.0;
    for ch in text.chars() {
        width += scaled.h_advance(font.glyph_id(ch));
    }
    let height = scaled.ascent() - scaled.descent();
    (width, height.max(0.0))
}

/// Alpha-blend `src` over `dst` by coverage `a` (per channel, alpha kept at 0xFF).
fn blend(dst: u32, src: u32, a: f32) -> u32 {
    let ch = |x: u32, s: u32| f32::from(((x >> s) & 0xFF) as u8);
    let l = |d: u32, s: u32, sh: u32| {
        let v = ch(d, sh) * (1.0 - a) + ch(s, sh) * a;
        ((v as u32) & 0xFF) << sh
    };
    l(dst, src, 16) | l(dst, src, 8) | l(dst, src, 0) | (0xFF << 24)
}

/// Rasterize `text` into `pix` (stride pixels wide). `x`/`y` is the top-left
/// corner of the text box. Out-of-buffer pixels are skipped.
pub fn draw_text(
    pix: &mut [u32],
    stride: u32,
    text: &str,
    font: &FontVec,
    px: f32,
    color: u32,
    x: i32,
    y: i32,
) {
    let scaled = font.as_scaled(px);
    let mut caret = point(x as f32, y as f32 + scaled.ascent());
    for ch in text.chars() {
        let gid = font.glyph_id(ch);
        let glyph = Glyph {
            id: gid,
            scale: PxScale::from(px),
            position: caret,
        };
        if let Some(outlined) = font.outline_glyph(glyph) {
            let bounds = outlined.px_bounds();
            outlined.draw(|gx, gy, v| {
                if v <= 0.0 {
                    return;
                }
                let ix = bounds.min.x as i64 + gx as i64;
                let iy = bounds.min.y as i64 + gy as i64;
                if ix < 0 || iy < 0 {
                    return;
                }
                let (ux, uy) = (ix as u32, iy as u32);
                let height = (pix.len() / stride as usize) as u32;
                if ux >= stride || uy >= height {
                    return;
                }
                let idx = (uy * stride + ux) as usize;
                pix[idx] = blend(pix[idx], color, v);
            });
        }
        caret.x += scaled.h_advance(gid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_chain_skips_missing_files() {
        // First path does not exist, second does: the chain must recover.
        let f = load_font(&["Z:\\nope.ttf", "C:\\Windows\\Fonts\\consola.ttf"]);
        assert!(f.is_some());
    }

    #[test]
    fn fallback_chain_returns_none_when_all_missing() {
        assert!(load_font(&["Z:\\nope1.ttf", "Z:\\nope2.ttf"]).is_none());
    }

    #[test]
    fn load_fonts_finds_both_on_windows() {
        assert!(load_fonts().is_some());
    }

    #[test]
    fn measure_nonzero() {
        let fonts = load_fonts().unwrap();
        assert!(measure("00:00", &fonts.mono, 150.0).0 > 0.0);
        assert!(measure("00:00", &fonts.mono, 150.0).1 > 0.0);
        assert!(measure("Monday", &fonts.cjk, 20.0).0 > 0.0);
        assert!(measure("Monday", &fonts.cjk, 20.0).1 > 0.0);
    }

    #[test]
    fn draw_text_paints_foreground_pixels() {
        let fonts = load_fonts().unwrap();
        let w = 200u32;
        let h = 60u32;
        let mut pix = vec![BG; (w * h) as usize];
        draw_text(&mut pix, w, "0", &fonts.mono, 50.0, FG, 0, 0);
        let fg_count = pix.iter().filter(|&&p| p == FG).count();
        assert!(fg_count > 0, "expected foreground pixels, found {fg_count}");
        // Every painted pixel must be FG or an antialiasing blend of BG/FG,
        // i.e. nothing outside the BG..FG channel range.
        assert!(pix.iter().all(|&p| p == BG || is_bg_fg_blend(p)));
    }

    #[test]
    fn draw_cjk_date_text_at_bottom_position() {
        // Replicates main.rs draw() for the date line at a bottom position.
        let fonts = load_fonts().unwrap();
        let (w, h) = (1536u32, 864u32);
        let date_px = 37.5f32;
        let date_s = "12/10/2026";
        let (dw, dh) = measure(date_s, &fonts.cjk, date_px);
        let margin = 12i32;
        let date_y = h as i32 - dh as i32 - margin;
        let mut pix = vec![BG; (w * h) as usize];
        draw_text(&mut pix, w, date_s, &fonts.cjk, date_px, FG, center_x(w as i32, dw as i32), date_y);
        let fg_count = pix.iter().filter(|&&p| p == FG).count();
        assert!(fg_count > 0, "date text produced no FG pixels at date_y={date_y}");
        // All painted rows must lie inside the buffer, at or below the box top.
        let mut min_row = h as usize;
        for y in 0..h as usize {
            let row = &pix[y * w as usize..(y + 1) * w as usize];
            if row.iter().any(|&p| p == FG) {
                min_row = y;
                break;
            }
        }
        assert!(
            min_row >= date_y as usize,
            "painted row {min_row} is above box top {date_y}"
        );
    }

    fn center_x(win_w: i32, text_w: i32) -> i32 {
        (win_w - text_w).max(0) / 2
    }

    fn is_bg_fg_blend(p: u32) -> bool {
        if (p >> 24) != 0xFF {
            return false;
        }
        let ch = |x: u32, s: u32| (x >> s) & 0xFF;
        let in_range = |a: u32, b: u32, v: u32| v >= a.min(b) && v <= a.max(b);
        in_range(ch(BG, 16), ch(FG, 16), ch(p, 16))
            && in_range(ch(BG, 8), ch(FG, 8), ch(p, 8))
            && in_range(ch(BG, 0), ch(FG, 0), ch(p, 0))
    }
}
