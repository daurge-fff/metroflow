use crate::config::i18n::Lang;
use crate::config::{Fonts, Palette};
use crate::geom::{Rect, Vec2};
use crate::render::{shapes, text};
use crate::ui::icons::{self, Icon};
use crate::ui::UiAction;
use macroquad::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonStyle {
    Primary,
    Secondary,
    Ghost,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Error,
}

pub struct Toast {
    pub text: String,
    pub kind: ToastKind,
    pub age: f32,
    pub life: f32,
}

impl Toast {
    pub fn new(text: impl Into<String>, kind: ToastKind) -> Self {
        Toast {
            text: text.into(),
            kind,
            age: 0.0,
            life: if kind == ToastKind::Error { 5.0 } else { 3.0 },
        }
    }

    pub fn update(&mut self, dt: f32) -> bool {
        self.age += dt;
        self.age < self.life
    }
}

pub struct UiContext<'a> {
    pub fonts: &'a Fonts,
    pub palette: &'a Palette,
    pub mouse: Vec2,
    pub pressed: bool,
    pub down: bool,
    pub released: bool,
    pub time: f32,
    pub lang: Lang,
    pub actions: Vec<UiAction>,
    pub pointer_over_ui: bool,
    pub slider_rects: Vec<Rect>,
    pub slider_grab: bool,
}

impl<'a> UiContext<'a> {
    pub fn new(fonts: &'a Fonts, palette: &'a Palette, time: f32, lang: Lang, scale: f32) -> Self {
        let (mx, my) = mouse_position();
        let scale = if scale > 0.0 { scale } else { 1.0 };
        UiContext {
            fonts,
            palette,
            mouse: Vec2::new(mx / scale, my / scale),
            pressed: is_mouse_button_pressed(MouseButton::Left),
            down: is_mouse_button_down(MouseButton::Left),
            released: is_mouse_button_released(MouseButton::Left),
            time,
            lang,
            actions: Vec::new(),
            pointer_over_ui: false,
            slider_rects: Vec::new(),
            slider_grab: false,
        }
    }

    pub fn tr(&self, ru: &'static str, en: &'static str) -> &'static str {
        match self.lang {
            Lang::Ru => ru,
            Lang::En => en,
        }
    }

    pub fn tf(&self, ru: &str, en: &str, args: &[(&str, &str)]) -> String {
        let mut out = match self.lang {
            Lang::Ru => ru.to_string(),
            Lang::En => en.to_string(),
        };
        for (key, value) in args {
            out = out.replace(&format!("{{{key}}}"), value);
        }
        out
    }

    pub fn take_actions(&mut self) -> Vec<UiAction> {
        std::mem::take(&mut self.actions)
    }

    pub fn emit(&mut self, action: UiAction) {
        self.actions.push(action);
    }

    fn hovered(&mut self, rect: Rect) -> bool {
        let hit = rect.contains(self.mouse);
        if hit {
            self.pointer_over_ui = true;
        }
        hit
    }

    pub fn note_region(&mut self, rect: Rect) {
        if rect.contains(self.mouse) {
            self.pointer_over_ui = true;
        }
    }

    pub fn panel(&mut self, rect: Rect) {
        self.note_region(rect);
        shapes::fill_round_rect(
            rect.pos.x + 2.0,
            rect.pos.y + 4.0,
            rect.size.x,
            rect.size.y,
            12.0,
            Color::new(0.0, 0.0, 0.0, 0.28),
        );
        shapes::fill_round_rect(
            rect.pos.x,
            rect.pos.y,
            rect.size.x,
            rect.size.y,
            12.0,
            self.palette.panel,
        );
        shapes::stroke_round_rect(
            rect.pos.x,
            rect.pos.y,
            rect.size.x,
            rect.size.y,
            12.0,
            1.5,
            self.palette.border,
        );
    }

    pub fn divider(&self, rect: Rect, color: Color) {
        draw_line(rect.pos.x, rect.pos.y, rect.right(), rect.pos.y, 1.0, color);
    }

    pub fn button(&mut self, rect: Rect, label: &str, style: ButtonStyle, enabled: bool) -> bool {
        let hovered = self.hovered(rect) && enabled;
        let clicked = hovered && self.pressed;
        let (fill, border, base_text) = match style {
            ButtonStyle::Primary => (self.palette.accent, None, self.palette.on_accent),
            ButtonStyle::Secondary => (
                self.palette.panel_raised,
                Some(self.palette.border),
                self.palette.text_primary,
            ),
            ButtonStyle::Ghost => (
                Color::new(0.0, 0.0, 0.0, 0.0),
                Some(self.palette.border),
                self.palette.text_secondary,
            ),
        };
        let mut fill = fill;
        if !enabled {
            fill.a *= 0.4;
        } else if clicked {
            fill = crate::config::palette::shade(fill, 0.82);
        } else if hovered {
            fill = crate::config::palette::mix(fill, Color::new(1.0, 1.0, 1.0, fill.a), 0.12);
        }
        if fill.a > 0.0 {
            shapes::fill_round_rect(rect.pos.x, rect.pos.y, rect.size.x, rect.size.y, 9.0, fill);
        }
        if let Some(border) = border {
            let bc = if hovered {
                self.palette.border_strong
            } else {
                border
            };
            shapes::stroke_round_rect(
                rect.pos.x,
                rect.pos.y,
                rect.size.x,
                rect.size.y,
                9.0,
                1.5,
                bc,
            );
        }
        let text_color = if enabled {
            base_text
        } else {
            crate::config::palette::with_alpha(base_text, 0.45)
        };
        let size = (rect.size.y * 0.4).clamp(13.0, 17.0);
        text::draw_centered(self.fonts, label, rect.center(), size, text_color, false);
        clicked
    }

    pub fn action_button(
        &mut self,
        rect: Rect,
        label: &str,
        style: ButtonStyle,
        enabled: bool,
        action: UiAction,
    ) -> bool {
        let clicked = self.button(rect, label, style, enabled);
        if clicked {
            let _ = label;
            self.emit(action);
        }
        clicked
    }

    pub fn icon_text_button(
        &mut self,
        rect: Rect,
        icon: Icon,
        label: &str,
        style: ButtonStyle,
        enabled: bool,
        action: UiAction,
    ) -> bool {
        let hovered = self.hovered(rect) && enabled;
        let clicked = hovered && self.pressed;
        let (mut fill, border, base_text) = match style {
            ButtonStyle::Primary => (self.palette.accent, None, self.palette.on_accent),
            ButtonStyle::Secondary => (
                self.palette.panel_raised,
                Some(self.palette.border),
                self.palette.text_primary,
            ),
            ButtonStyle::Ghost => (
                Color::new(0.0, 0.0, 0.0, 0.0),
                Some(self.palette.border),
                self.palette.text_secondary,
            ),
        };
        if !enabled {
            fill.a *= 0.4;
        } else if clicked {
            fill = crate::config::palette::shade(fill, 0.82);
        } else if hovered {
            fill = crate::config::palette::mix(fill, Color::new(1.0, 1.0, 1.0, fill.a), 0.12);
        }
        if fill.a > 0.0 {
            shapes::fill_round_rect(rect.pos.x, rect.pos.y, rect.size.x, rect.size.y, 9.0, fill);
        }
        if let Some(border) = border {
            let bc = if hovered {
                self.palette.border_strong
            } else {
                border
            };
            shapes::stroke_round_rect(
                rect.pos.x,
                rect.pos.y,
                rect.size.x,
                rect.size.y,
                9.0,
                1.5,
                bc,
            );
        }
        let text_color = if enabled {
            base_text
        } else {
            crate::config::palette::with_alpha(base_text, 0.45)
        };
        let icon_size = (rect.size.y - 16.0).clamp(14.0, 22.0);
        let text_size = 14.0;
        let dims = text::measure(self.fonts, label, text_size, false);
        let gap = 8.0;
        let total = icon_size + gap + dims.x;
        let start = rect.center().x - total * 0.5;
        icons::draw(
            self.fonts,
            icon,
            Rect::new(
                start,
                rect.center().y - icon_size * 0.5,
                icon_size,
                icon_size,
            ),
            text_color,
        );
        text::draw(
            self.fonts,
            label,
            Vec2::new(start + icon_size + gap, rect.center().y - dims.y * 0.5),
            text_size,
            text_color,
            false,
        );
        if clicked {
            self.emit(action);
        }
        clicked
    }

    pub fn list_row(&mut self, rect: Rect, selected: bool) -> bool {
        let hovered = self.hovered(rect);
        let clicked = hovered && self.pressed;
        if selected {
            shapes::fill_round_rect(
                rect.pos.x,
                rect.pos.y,
                rect.size.x,
                rect.size.y,
                8.0,
                crate::config::palette::with_alpha(self.palette.accent, 0.18),
            );
            shapes::stroke_round_rect(
                rect.pos.x,
                rect.pos.y,
                rect.size.x,
                rect.size.y,
                8.0,
                1.5,
                self.palette.accent,
            );
        } else if hovered {
            shapes::fill_round_rect(
                rect.pos.x,
                rect.pos.y,
                rect.size.x,
                rect.size.y,
                8.0,
                self.palette.panel_raised,
            );
        }
        clicked
    }

    pub fn toggle(
        &mut self,
        rect: Rect,
        label: &str,
        on: bool,
        enabled: bool,
        action: UiAction,
    ) -> bool {
        let hovered = self.hovered(rect) && enabled;
        let clicked = hovered && self.pressed;
        let fill = if on {
            crate::config::palette::with_alpha(
                self.palette.accent,
                if hovered { 0.32 } else { 0.22 },
            )
        } else if hovered {
            self.palette.panel_raised
        } else {
            Color::new(0.0, 0.0, 0.0, 0.0)
        };
        if fill.a > 0.0 {
            shapes::fill_round_rect(rect.pos.x, rect.pos.y, rect.size.x, rect.size.y, 9.0, fill);
        }
        shapes::stroke_round_rect(
            rect.pos.x,
            rect.pos.y,
            rect.size.x,
            rect.size.y,
            9.0,
            1.5,
            if on {
                self.palette.accent
            } else {
                self.palette.border
            },
        );
        let text_color = if on {
            self.palette.accent_strong
        } else {
            self.palette.text_secondary
        };
        let size = (rect.size.y * 0.36).clamp(12.0, 15.0);
        text::draw_centered(self.fonts, label, rect.center(), size, text_color, false);
        if clicked {
            self.emit(action);
        }
        clicked
    }

    /// Slider; only reacts to presses that started on it (`slider_grab`).
    pub fn slider(
        &mut self,
        rect: Rect,
        value: f32,
        min: f32,
        max: f32,
        make: impl Fn(f32) -> UiAction,
    ) {
        self.slider_rects.push(rect);
        if rect.contains(self.mouse) {
            self.pointer_over_ui = true;
        }
        let cy = rect.center().y;
        let track_h = 6.0;
        let span = (max - min).max(1e-6);
        let t = ((value - min) / span).clamp(0.0, 1.0);
        shapes::fill_round_rect(
            rect.pos.x,
            cy - track_h * 0.5,
            rect.size.x,
            track_h,
            3.0,
            self.palette.panel_alt,
        );
        shapes::fill_round_rect(
            rect.pos.x,
            cy - track_h * 0.5,
            (rect.size.x * t).max(track_h),
            track_h,
            3.0,
            self.palette.accent,
        );
        let kx = rect.pos.x + rect.size.x * t;
        draw_circle(kx, cy, 11.0, self.palette.accent_strong);
        draw_circle_lines(kx, cy, 11.0, 2.5, self.palette.map_bg);

        if self.slider_grab
            && self.down
            && self.mouse.x >= rect.pos.x - 16.0
            && self.mouse.x <= rect.right() + 16.0
            && (self.mouse.y - cy).abs() <= 30.0
        {
            let nt = ((self.mouse.x - rect.pos.x) / rect.size.x).clamp(0.0, 1.0);
            let nv = min + nt * span;
            if (nv - value).abs() > 1e-4 {
                self.emit(make(nv));
            }
        }
    }

    pub fn icon_button(
        &mut self,
        rect: Rect,
        icon: Icon,
        toggled: bool,
        enabled: bool,
        action: UiAction,
    ) -> bool {
        let hovered = self.hovered(rect) && enabled;
        let clicked = hovered && self.pressed;
        let fill = if toggled {
            crate::config::palette::with_alpha(self.palette.accent, 0.22)
        } else if hovered {
            self.palette.panel_raised
        } else {
            Color::new(0.0, 0.0, 0.0, 0.0)
        };
        if fill.a > 0.0 {
            shapes::fill_round_rect(rect.pos.x, rect.pos.y, rect.size.x, rect.size.y, 9.0, fill);
        }
        shapes::stroke_round_rect(
            rect.pos.x,
            rect.pos.y,
            rect.size.x,
            rect.size.y,
            9.0,
            1.5,
            if toggled {
                self.palette.accent
            } else {
                self.palette.border
            },
        );
        let color = if !enabled {
            crate::config::palette::with_alpha(self.palette.text_muted, 0.5)
        } else if toggled {
            self.palette.accent_strong
        } else if hovered {
            self.palette.text_primary
        } else {
            self.palette.text_secondary
        };
        let inner = Rect::new(
            rect.pos.x + rect.size.x * 0.18,
            rect.pos.y + rect.size.y * 0.18,
            rect.size.x * 0.64,
            rect.size.y * 0.64,
        );
        icons::draw(self.fonts, icon, inner, color);
        if clicked {
            self.emit(action);
        }
        clicked
    }

    pub fn segmented(
        &mut self,
        rect: Rect,
        labels: &[&str],
        selected: usize,
        make: impl Fn(usize) -> UiAction,
    ) {
        self.note_region(rect);
        shapes::fill_round_rect(
            rect.pos.x,
            rect.pos.y,
            rect.size.x,
            rect.size.y,
            9.0,
            self.palette.panel_alt,
        );
        let n = labels.len().max(1);
        let seg_w = rect.size.x / n as f32;
        for (i, label) in labels.iter().enumerate() {
            let seg = Rect::new(
                rect.pos.x + i as f32 * seg_w + 3.0,
                rect.pos.y + 3.0,
                seg_w - 6.0,
                rect.size.y - 6.0,
            );
            let active = i == selected;
            let hovered = seg.contains(self.mouse);
            if hovered {
                self.pointer_over_ui = true;
            }
            if active {
                shapes::fill_round_rect(
                    seg.pos.x,
                    seg.pos.y,
                    seg.size.x,
                    seg.size.y,
                    7.0,
                    self.palette.accent,
                );
            } else if hovered {
                shapes::fill_round_rect(
                    seg.pos.x,
                    seg.pos.y,
                    seg.size.x,
                    seg.size.y,
                    7.0,
                    self.palette.panel_raised,
                );
            }
            let color = if active {
                self.palette.on_accent
            } else {
                self.palette.text_secondary
            };
            text::draw_centered(self.fonts, label, seg.center(), 14.0, color, active);
            if hovered && self.pressed && !active {
                self.emit(make(i));
            }
        }
    }
}

pub fn draw_toast(fonts: &Fonts, palette: &Palette, toast: &Toast, screen: Vec2) {
    let fade = {
        let remain = toast.life - toast.age;
        (toast.age.min(0.3) / 0.3).min(remain / 0.5).clamp(0.0, 1.0)
    };
    let (accent, icon) = match toast.kind {
        ToastKind::Success => (palette.success, Icon::Save),
        ToastKind::Error => (palette.danger, Icon::Close),
    };
    let size = 15.0;
    let dims = text::measure(fonts, &toast.text, size, false);
    let w = dims.x + 74.0;
    let h = 46.0;
    let x = screen.x * 0.5 - w * 0.5;
    let y = screen.y - h - 28.0;
    shapes::fill_round_rect(
        x,
        y,
        w,
        h,
        12.0,
        crate::config::palette::with_alpha(palette.panel, fade),
    );
    shapes::stroke_round_rect(
        x,
        y,
        w,
        h,
        12.0,
        1.5,
        crate::config::palette::with_alpha(accent, fade),
    );
    icons::draw(
        fonts,
        icon,
        Rect::new(x + 14.0, y + h * 0.5 - 11.0, 22.0, 22.0),
        crate::config::palette::with_alpha(accent, fade),
    );
    text::draw(
        fonts,
        &toast.text,
        Vec2::new(x + 46.0, y + h * 0.5 - dims.y * 0.5),
        size,
        crate::config::palette::with_alpha(palette.text_primary, fade),
        false,
    );
}
