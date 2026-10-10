use crate::config::Fonts;
use crate::geom::Rect;
use crate::render::{shapes, text};
use macroquad::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Play,
    Pause,
    Plus,
    Minus,
    Fit,
    Center,
    Tag,
    Grid,
    Motion,
    Save,
    Load,
    Menu,
    Close,
    Person,
    Edit,
    Delete,
    Undo,
    Redo,
    Palette,
    AddCircle,
    Route,
    Settings,
    Layers,
    Check,
    Heat,
    Flow,
    Help,
    Chart,
    Clock,
    Back,
}

impl Icon {
    pub fn codepoint(self) -> char {
        match self {
            Icon::Play => '\u{e037}',
            Icon::Pause => '\u{e034}',
            Icon::Plus => '\u{e145}',
            Icon::Minus => '\u{e15b}',
            Icon::Fit => '\u{e5d0}',
            Icon::Center => '\u{e3b4}',
            Icon::Tag => '\u{e892}',
            Icon::Grid => '\u{e3ec}',
            Icon::Motion => '\u{e71c}',
            Icon::Save => '\u{e161}',
            Icon::Load => '\u{e2c6}',
            Icon::Menu => '\u{e5d2}',
            Icon::Close => '\u{e5cd}',
            Icon::Person => '\u{e7fd}',
            Icon::Edit => '\u{e3c9}',
            Icon::Delete => '\u{e872}',
            Icon::Undo => '\u{e166}',
            Icon::Redo => '\u{e15a}',
            Icon::Palette => '\u{e40a}',
            Icon::AddCircle => '\u{e148}',
            Icon::Route => '\u{eacd}',
            Icon::Settings => '\u{e8b8}',
            Icon::Layers => '\u{e53b}',
            Icon::Check => '\u{e5ca}',
            Icon::Heat => '\u{e80e}',
            Icon::Flow => '\u{e922}',
            Icon::Help => '\u{e8fd}',
            Icon::Chart => '\u{e6e1}',
            Icon::Clock => '\u{e8b5}',
            Icon::Back => '\u{e5c4}',
        }
    }
}

pub fn draw(fonts: &Fonts, icon: Icon, rect: Rect, color: Color) {
    let size = rect.size.x.min(rect.size.y);
    text::draw_glyph(fonts, icon.codepoint(), rect.center(), size, color);
}

pub fn line_chip(center_rect: Rect, color: Color, style: crate::config::palette::LineStyle) {
    shapes::fill_round_rect(
        center_rect.pos.x,
        center_rect.pos.y,
        center_rect.size.x,
        center_rect.size.y,
        center_rect.size.y * 0.5,
        color,
    );
    if style != crate::config::palette::LineStyle::Solid {
        let c = center_rect.center();
        draw_line(
            c.x - center_rect.size.x * 0.28,
            c.y,
            c.x + center_rect.size.x * 0.28,
            c.y,
            1.5,
            Color::new(0.0, 0.0, 0.0, 0.35),
        );
    }
}
