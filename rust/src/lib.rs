pub mod render;
pub mod single;
pub mod timefmt;

/// Horizontal offset that centers a `text_w` wide box in a `win_w` wide
/// window; never negative (oversized text pins to the left edge).
pub fn center_offset(win_w: i32, text_w: i32) -> i32 {
    (win_w - text_w).max(0) / 2
}

/// Scale a point size by a DPI scale factor.
pub fn scaled(px: f32, s: f64) -> f32 {
    px * s as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_offset_centers() {
        assert_eq!(center_offset(1000, 300), 350);
    }

    #[test]
    fn center_offset_never_negative() {
        assert_eq!(center_offset(100, 101), 0);
        assert_eq!(center_offset(100, 300), 0);
    }

    #[test]
    fn scaled_multiplies_by_dpi_factor() {
        assert_eq!(scaled(150.0, 1.5), 225.0);
        assert_eq!(scaled(20.0, 1.0), 20.0);
        assert_eq!(scaled(30.0, 2.0), 60.0);
    }
}
