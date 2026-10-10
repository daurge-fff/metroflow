use crate::config::Fonts;
use crate::geom::Vec2;
use crate::render::scale;
use macroquad::prelude::*;

pub fn measure(fonts: &Fonts, text: &str, size: f32, bold: bool) -> Vec2 {
    let font = if bold { fonts.bold() } else { fonts.regular() };
    let (font_size, font_scale) = scale::font_flavor(size);
    let dims = measure_text(text, font, font_size, font_scale);
    Vec2::new(dims.width, dims.height.max(size))
}

pub fn draw(fonts: &Fonts, text: &str, top_left: Vec2, size: f32, color: Color, bold: bool) {
    let font = if bold { fonts.bold() } else { fonts.regular() };
    let (font_size, font_scale) = scale::font_flavor(size);
    let dims = measure_text(text, font, font_size, font_scale);
    let baseline = top_left.y + dims.offset_y;
    // Glyph-by-glyph: avoids macroquad's kerning, which ignores `font_scale`.
    let mut x = top_left.x;
    let mut buf = [0u8; 4];
    for ch in text.chars() {
        let glyph: &str = ch.encode_utf8(&mut buf);
        draw_text_ex(
            glyph,
            x,
            baseline,
            TextParams {
                font,
                font_size,
                font_scale,
                color,
                ..Default::default()
            },
        );
        x += measure_text(glyph, font, font_size, font_scale).width;
    }
}

pub fn draw_centered(fonts: &Fonts, text: &str, center: Vec2, size: f32, color: Color, bold: bool) {
    let dims = measure(fonts, text, size, bold);
    draw(
        fonts,
        text,
        Vec2::new(center.x - dims.x * 0.5, center.y - dims.y * 0.5),
        size,
        color,
        bold,
    );
}

pub fn draw_right(fonts: &Fonts, text: &str, right_top: Vec2, size: f32, color: Color, bold: bool) {
    let dims = measure(fonts, text, size, bold);
    draw(
        fonts,
        text,
        Vec2::new(right_top.x - dims.x, right_top.y),
        size,
        color,
        bold,
    );
}

pub fn draw_glyph(fonts: &Fonts, ch: char, center: Vec2, size: f32, color: Color) {
    let Some(font) = fonts.icons() else {
        return;
    };
    let mut buf = [0u8; 4];
    let text: &str = ch.encode_utf8(&mut buf);
    let (font_size, font_scale) = scale::font_flavor(size);
    let dims = measure_text(text, Some(font), font_size, font_scale);
    draw_text_ex(
        text,
        center.x - dims.width * 0.5,
        center.y - dims.height * 0.5 + dims.offset_y,
        TextParams {
            font: Some(font),
            font_size,
            font_scale,
            color,
            ..Default::default()
        },
    );
}

pub fn draw_with_shadow(
    fonts: &Fonts,
    text: &str,
    top_left: Vec2,
    size: f32,
    color: Color,
    shadow: Color,
    bold: bool,
) {
    draw(
        fonts,
        text,
        Vec2::new(top_left.x + 1.0, top_left.y + 1.0),
        size,
        shadow,
        bold,
    );
    draw(fonts, text, top_left, size, color, bold);
}
