use macroquad::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};

static SCALE: AtomicU32 = AtomicU32::new(0);
static DESIGN_W: AtomicU32 = AtomicU32::new(0);
static DESIGN_H: AtomicU32 = AtomicU32::new(0);

pub fn set(scale: f32, design_w: f32, design_h: f32) {
    SCALE.store(scale.to_bits(), Ordering::Relaxed);
    DESIGN_W.store(design_w.to_bits(), Ordering::Relaxed);
    DESIGN_H.store(design_h.to_bits(), Ordering::Relaxed);
}

pub fn scale() -> f32 {
    f32::from_bits(SCALE.load(Ordering::Relaxed))
}

pub fn factor() -> f32 {
    scale() * screen_dpi_scale()
}

/// Maps a design-space font size to a (raster size, scale) pair that keeps
/// glyphs 1:1 with the UI camera. `measure` uses the same pair, so layout is
/// unaffected.
pub fn font_flavor(size: f32) -> (u16, f32) {
    let s = scale();
    if (s - 1.0).abs() < 1e-3 || s <= 0.0 {
        return (size.max(1.0) as u16, 1.0);
    }
    let px = (size * s).ceil().clamp(1.0, u16::MAX as f32);
    (px as u16, size / px)
}

pub fn ui_camera() -> Camera2D {
    let dw = f32::from_bits(DESIGN_W.load(Ordering::Relaxed)).max(1.0);
    let dh = f32::from_bits(DESIGN_H.load(Ordering::Relaxed)).max(1.0);
    Camera2D {
        target: vec2(dw * 0.5, dh * 0.5),
        zoom: vec2(2.0 / dw, 2.0 / dh),
        ..Default::default()
    }
}

pub fn active() -> bool {
    (scale() - 1.0).abs() > 1e-3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_flavor_is_crisp_and_size_preserving() {
        set(1.0, 100.0, 100.0);
        assert_eq!(font_flavor(14.0), (14, 1.0));

        set(1.4, 100.0, 100.0);
        let (font_size, font_scale) = font_flavor(14.0);
        assert!(font_size > 14);
        let visual = font_size as f32 * font_scale;
        assert!((visual - 14.0).abs() < 1.0);

        set(1.0, 100.0, 100.0);
    }
}
