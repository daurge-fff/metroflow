use crate::config::palette::{with_alpha, LINE_COLORS};
use crate::config::Settings;
use crate::geom::{Rect, Vec2};
use crate::render::{shapes, text};
use crate::ui::icons::{self, Icon};
use crate::ui::layout::Layout;
use crate::ui::widgets::{ButtonStyle, UiContext};
use crate::ui::UiAction;
use macroquad::prelude::*;

pub const DEVELOPER: &str = "German Vitiaz";
pub const HANDLE: &str = "@daurge";
pub const GITHUB_URL: &str = "https://github.com/daurge-fff";
pub const TELEGRAM_URL: &str = "https://t.me/daurge";

/// Game features shown on the start screen (ru, en).
const FEATURES: &[(&str, &str)] = &[
    ("Карты и редактор сети", "Maps & network editor"),
    ("Маршруты и пересадки", "Routes & transfers"),
    ("Поезда и пассажиры", "Trains & passengers"),
    ("Аналитика в реальном времени", "Real-time analytics"),
    ("Свои цвета линий", "Custom line colors"),
    ("Авто-состав по нагрузке", "Auto-consist by load"),
];

/// How the simulation works (ru, en) — short, honest formulas.
const MECHANICS: &[(&str, &str)] = &[
    (
        "Нагрузка линии = пассажиры в поездах + ожидающие на её станциях (на пересадках делится между линиями).",
        "Line load = passengers onboard + waiting at its stations (split between lines at interchanges).",
    ),
    (
        "Авто-состав: вагоны = мин + (макс − мин) × давление нагрузки.",
        "Auto-consist: cars = min + (max − min) × demand pressure.",
    ),
    (
        "Авто-поезда: поезда = мин + (макс − мин) × давление нагрузки.",
        "Auto-trains: trains = min + (max − min) × demand pressure.",
    ),
    (
        "Сложность задаёт поток: интервал спавна = 0.2 / множитель (Спокойный 0.6 · Обычный 1 · Людно 1.5 · Час пик 2.2).",
        "Difficulty sets flow: spawn interval = 0.2 / multiplier (Calm 0.6 · Normal 1 · Busy 1.5 · Rush 2.2).",
    ),
    (
        "Издержки растут за вагоны сверх минимальной конфигурации — за лишние составы платишь.",
        "Cost grows for cars above the minimum configuration — extra consists are not free.",
    ),
    (
        "Маршрут — самый быстрый по времени с учётом штрафа за пересадку (эскалаторы).",
        "Route — fastest by time including a transfer penalty (escalators).",
    ),
    (
        "Пассажир садится только в поезд, идущий к его пересадке или пункту назначения.",
        "A passenger boards only a train heading toward their transfer or destination.",
    ),
    (
        "Сутки = 24 игровых часа; пассажиропоток зависит от часа (утро/вечер) и растёт по дням.",
        "A day = 24 game hours; flow depends on the hour (morning/evening) and grows over days.",
    ),
];

pub fn draw_background(ui: &mut UiContext, layout: &Layout) {
    let w = layout.window.x;
    let h = layout.window.y;
    draw_rectangle(0.0, 0.0, w, h, ui.palette.bg_top);
    let bands = 24;
    for i in 0..bands {
        let t = i as f32 / bands as f32;
        let y = t * h;
        let color = crate::config::palette::mix(ui.palette.bg_top, ui.palette.bg_bottom, t);
        draw_rectangle(0.0, y, w, h / bands as f32 + 1.0, color);
    }

    let ribbons = 5;
    for i in 0..ribbons {
        let color = LINE_COLORS[i % LINE_COLORS.len()];
        let base_y = h * (0.18 + 0.16 * i as f32);
        let phase = ui.time * (0.25 + i as f32 * 0.05) + i as f32 * 1.7;
        let mut pts = Vec::new();
        let steps = 40;
        for s in 0..=steps {
            let x = w * s as f32 / steps as f32;
            let y = base_y + (x * 0.006 + phase).sin() * 26.0;
            pts.push(macroquad::prelude::vec2(x, y));
        }
        shapes::stroke_polyline(&pts, 6.0, with_alpha(color, 0.18));
    }
    draw_rectangle(0.0, 0.0, w, h, with_alpha(ui.palette.bg_top, 0.35));
}

/// A rounded card with an icon + title. Returns the content box (x, y, width).
fn card(ui: &mut UiContext, rect: Rect, title: &str, icon: Icon) -> (f32, f32, f32) {
    shapes::fill_round_rect(
        rect.pos.x + 2.0,
        rect.pos.y + 4.0,
        rect.size.x,
        rect.size.y,
        14.0,
        Color::new(0.0, 0.0, 0.0, 0.22),
    );
    shapes::fill_round_rect(
        rect.pos.x,
        rect.pos.y,
        rect.size.x,
        rect.size.y,
        14.0,
        ui.palette.panel,
    );
    shapes::stroke_round_rect(
        rect.pos.x,
        rect.pos.y,
        rect.size.x,
        rect.size.y,
        14.0,
        1.5,
        ui.palette.border,
    );
    let ix = rect.pos.x + 18.0;
    icons::draw(
        ui.fonts,
        icon,
        Rect::new(ix, rect.pos.y + 15.0, 20.0, 20.0),
        ui.palette.accent,
    );
    text::draw(
        ui.fonts,
        title,
        Vec2::new(ix + 30.0, rect.pos.y + 16.0),
        17.0,
        ui.palette.text_primary,
        true,
    );
    (rect.pos.x + 18.0, rect.pos.y + 50.0, rect.size.x - 36.0)
}

#[allow(clippy::too_many_arguments)]
fn slider_row(
    ui: &mut UiContext,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value_text: &str,
    value: f32,
    min: f32,
    max: f32,
    make: impl Fn(f32) -> UiAction,
) {
    text::draw(
        ui.fonts,
        label,
        Vec2::new(x, y + 2.0),
        14.0,
        ui.palette.text_primary,
        false,
    );
    text::draw_right(
        ui.fonts,
        value_text,
        Vec2::new(x + w, y + 2.0),
        14.0,
        ui.palette.accent_strong,
        true,
    );
    ui.slider(Rect::new(x, y + 22.0, w, 22.0), value, min, max, make);
}

fn wrap_lines(ui: &UiContext, s: &str, max_w: f32, size: f32) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in s.split_whitespace() {
        let cand = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && text::measure(ui.fonts, &cand, size, false).x > max_w {
            out.push(line);
            line = word.to_string();
        } else {
            line = cand;
        }
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

pub fn draw_main_menu(ui: &mut UiContext, layout: &Layout, has_save: bool) {
    draw_background(ui, layout);
    let w = layout.window.x;
    let h = layout.window.y;

    let wide = w >= 900.0;
    let seg_w = 168.0;
    let (bx, col_w, bottom) = if wide {
        let split = w * 0.50;
        draw_hero(ui, Rect::new(0.0, 0.0, split, h));
        let col_w = (w - split - 64.0).min(440.0);
        let bx = split + ((w - split) - col_w) * 0.5;
        (bx, col_w, draw_menu_buttons(ui, bx, col_w, has_save, h))
    } else {
        draw_hero(ui, Rect::new(0.0, 0.0, w, h * 0.40));
        let col_w = (w - 48.0).min(440.0);
        let bx = (w - col_w) * 0.5;
        (bx, col_w, draw_menu_buttons(ui, bx, col_w, has_save, h))
    };

    let lang_labels: Vec<&str> = crate::config::i18n::Lang::ALL
        .iter()
        .map(|l| l.label())
        .collect();
    let seg = Rect::new(
        bx + (col_w - seg_w) * 0.5,
        (bottom + 12.0).min(h - 44.0),
        seg_w,
        34.0,
    );
    ui.segmented(seg, &lang_labels, ui.lang.index(), UiAction::SetLanguage);
    draw_footer(ui, layout);
}

fn draw_hero(ui: &mut UiContext, rect: Rect) {
    let compact = rect.size.y < 520.0;
    let cx = rect.center().x;
    let er = if compact { 44.0 } else { 62.0 };
    let mut y = rect.pos.y + if compact { 34.0 } else { rect.size.y * 0.16 };
    draw_logo(ui, Vec2::new(cx, y + er), er);
    y += er * 2.0 + 40.0;

    let title = if compact { 46.0 } else { 62.0 };
    text::draw_centered(
        ui.fonts,
        "MetroFlow",
        Vec2::new(cx, y),
        title,
        ui.palette.text_primary,
        true,
    );
    y += title * 0.8 + 6.0;
    text::draw_centered(
        ui.fonts,
        ui.tr(
            "Минималистичный симулятор метрополитена",
            "A minimalist metro simulator",
        ),
        Vec2::new(cx, y),
        if compact { 14.0 } else { 17.0 },
        ui.palette.text_secondary,
        false,
    );
    y += if compact { 30.0 } else { 40.0 };

    draw_hero_train(ui, rect, y);
}

fn tri(t: f32) -> f32 {
    let t = t.fract();
    if t < 0.5 {
        t * 2.0
    } else {
        2.0 - t * 2.0
    }
}

fn train_glyph(ui: &UiContext, x: f32, y: f32, w: f32, color: Color) {
    let h = w * 0.46;
    shapes::fill_round_rect(
        x + 1.0,
        y + 2.0,
        w,
        h,
        h * 0.28,
        Color::new(0.0, 0.0, 0.0, 0.25),
    );
    shapes::fill_round_rect(x, y, w, h, h * 0.28, color);
    let win = with_alpha(ui.palette.on_accent, 0.82);
    let n = 3;
    for i in 0..n {
        let wx = x + w * 0.14 + i as f32 * w * 0.26;
        shapes::fill_round_rect(wx, y + h * 0.22, w * 0.17, h * 0.34, 2.0, win);
    }
    draw_circle(x + w * 0.22, y + h, h * 0.13, ui.palette.text_secondary);
    draw_circle(x + w * 0.78, y + h, h * 0.13, ui.palette.text_secondary);
    draw_circle(x + w - 3.0, y + h * 0.32, 2.5, ui.palette.warning);
}

fn train_on(
    ui: &UiContext,
    a: macroquad::prelude::Vec2,
    b: macroquad::prelude::Vec2,
    t: f32,
    w: f32,
    color: Color,
) {
    let center = a + (b - a) * t;
    let angle = (b.y - a.y).atan2(b.x - a.x);
    let h = w * 0.46;
    shapes::fill_rounded_rotated_rect(
        center,
        macroquad::prelude::vec2(w, h),
        angle,
        h * 0.28,
        color,
    );
    let (s, c) = angle.sin_cos();
    let win = with_alpha(ui.palette.on_accent, 0.82);
    for i in 0..3 {
        let off = (i as f32 - 1.0) * w * 0.27;
        draw_circle(center.x + off * c, center.y + off * s, h * 0.16, win);
    }
}

/// Animated metro scene: two lines crossing at an interchange, with trains.
fn draw_hero_train(ui: &mut UiContext, rect: Rect, y: f32) {
    let cx = rect.center().x;
    let track_w = (rect.size.x * 0.76).min(480.0);
    let x0 = cx - track_w * 0.5;
    let ty = y + 24.0;
    let color = with_alpha(ui.palette.border, 0.95);

    let a = macroquad::prelude::vec2(x0 + track_w * 0.18, ty + 44.0);
    let b = macroquad::prelude::vec2(x0 + track_w * 0.66, ty - 44.0);
    let cross_x = (a.x + b.x) * 0.5;

    // tracks
    shapes::stroke_polyline(&[a, b], 6.0, color);
    shapes::fill_round_rect(x0, ty - 3.0, track_w, 6.0, 3.0, color);

    // stations on the horizontal line (skip the one nearest the interchange)
    for i in 0..=4 {
        let sx = x0 + track_w * i as f32 / 4.0;
        if (sx - cross_x).abs() < 30.0 {
            continue;
        }
        draw_circle(sx, ty, 6.0, ui.palette.station_fill);
        draw_circle_lines(sx, ty, 6.0, 2.0, ui.palette.station_border);
    }
    for p in [a, b] {
        draw_circle(p.x, p.y, 6.0, ui.palette.station_fill);
        draw_circle_lines(p.x, p.y, 6.0, 2.0, ui.palette.station_border);
    }
    // interchange at the crossing
    draw_circle(cross_x, ty, 12.0, ui.palette.station_border);
    draw_circle(cross_x, ty, 9.5, ui.palette.station_fill);
    draw_circle(cross_x, ty, 4.0, ui.palette.accent);

    let period = 9.0;
    let w = 52.0;
    // two trains shuttle on the horizontal line, opposite phases
    let run = track_w + w;
    let t1 = tri(ui.time / period);
    let t2 = tri(ui.time / period + 0.5);
    train_glyph(ui, x0 - w + run * t1, ty - 21.0, w, ui.palette.accent);
    train_glyph(ui, x0 - w + run * t2, ty - 21.0, w, ui.palette.success);
    // diagonal train, rotated along the line
    let td = tri(ui.time / period + 0.25);
    train_on(ui, a, b, td, w * 0.9, ui.palette.warning);
}

fn draw_menu_buttons(ui: &mut UiContext, x: f32, w: f32, has_save: bool, h: f32) -> f32 {
    let top = 26.0;
    let footer = 96.0;
    let gap = 10.0;
    let avail = (h - top - footer).max(120.0);
    let rows = 4.0;
    let bh = ((avail - (rows - 1.0) * gap) / rows).clamp(36.0, 62.0);
    let block = rows * bh + (rows - 1.0) * gap;
    let y0 = top + (avail - block).max(0.0) * 0.5;

    let panel = Rect::new(x - 22.0, y0 - 20.0, w + 44.0, block + 40.0);
    shapes::fill_round_rect(
        panel.pos.x + 2.0,
        panel.pos.y + 6.0,
        panel.size.x,
        panel.size.y,
        20.0,
        Color::new(0.0, 0.0, 0.0, 0.18),
    );
    shapes::fill_round_rect(
        panel.pos.x,
        panel.pos.y,
        panel.size.x,
        panel.size.y,
        20.0,
        with_alpha(ui.palette.panel, 0.72),
    );
    shapes::stroke_round_rect(
        panel.pos.x,
        panel.pos.y,
        panel.size.x,
        panel.size.y,
        20.0,
        1.5,
        with_alpha(ui.palette.border, 0.9),
    );

    let half = (w - gap) * 0.5;
    let y1 = y0 + bh + gap;
    let y2 = y1 + bh + gap;
    let y3 = y2 + bh + gap;

    ui.icon_text_button(
        Rect::new(x, y0, w, bh),
        Icon::Play,
        ui.tr("Новая симуляция", "New simulation"),
        ButtonStyle::Primary,
        true,
        UiAction::OpenSetup,
    );
    ui.icon_text_button(
        Rect::new(x, y1, half, bh),
        Icon::Edit,
        ui.tr("Своя карта", "Custom map"),
        ButtonStyle::Secondary,
        true,
        UiAction::NewEmpty,
    );
    ui.icon_text_button(
        Rect::new(x + half + gap, y1, half, bh),
        Icon::Load,
        ui.tr("Продолжить", "Continue"),
        ButtonStyle::Secondary,
        has_save,
        UiAction::LoadGame,
    );
    ui.icon_text_button(
        Rect::new(x, y2, half, bh),
        Icon::Settings,
        ui.tr("Настройки", "Settings"),
        ButtonStyle::Ghost,
        true,
        UiAction::OpenSettings,
    );
    ui.icon_text_button(
        Rect::new(x + half + gap, y2, half, bh),
        Icon::Help,
        ui.tr("О проекте", "About"),
        ButtonStyle::Ghost,
        true,
        UiAction::OpenAbout,
    );
    ui.icon_text_button(
        Rect::new(x, y3, w, bh),
        Icon::Close,
        ui.tr("Выход", "Quit"),
        ButtonStyle::Ghost,
        true,
        UiAction::Quit,
    );
    y3 + bh
}

fn draw_footer(ui: &mut UiContext, layout: &Layout) {
    let w = layout.window.x;
    let h = layout.window.y;
    let ver = env!("CARGO_PKG_VERSION");
    text::draw_centered(
        ui.fonts,
        &ui.tf(
            "{a} ({h}) · v{v}",
            "{a} ({h}) · v{v}",
            &[("a", DEVELOPER), ("h", HANDLE), ("v", ver)],
        ),
        Vec2::new(w * 0.5, h - 30.0),
        12.0,
        ui.palette.text_muted,
        false,
    );
}

/// Modern metro logo: a bold "M" line with stations and a little train.
fn draw_logo(ui: &UiContext, center: Vec2, size: f32) {
    let s = size;
    shapes::soft_glow(
        macroquad::prelude::vec2(center.x, center.y),
        s * 1.7,
        ui.palette.accent,
        8,
    );
    let pts = [
        Vec2::new(center.x - s * 0.72, center.y + s * 0.72),
        Vec2::new(center.x - s * 0.72, center.y - s * 0.72),
        Vec2::new(center.x, center.y + s * 0.1),
        Vec2::new(center.x + s * 0.72, center.y - s * 0.72),
        Vec2::new(center.x + s * 0.72, center.y + s * 0.72),
    ];
    let mq: Vec<macroquad::prelude::Vec2> = pts.iter().map(|p| shapes::mq(*p)).collect();
    shapes::stroke_polyline(&mq, s * 0.40, ui.palette.track_casing);
    shapes::stroke_polyline(&mq, s * 0.26, ui.palette.accent);
    for i in [1usize, 2, 3] {
        let c = pts[i];
        draw_circle(c.x, c.y, s * 0.16, ui.palette.station_fill);
        draw_circle_lines(c.x, c.y, s * 0.16, s * 0.06, ui.palette.station_border);
    }
    draw_circle(pts[2].x, pts[2].y, s * 0.07, ui.palette.accent);
    // little train on the right leg
    let tc = Rect::new(pts[4].x - s * 0.20, center.y + s * 0.18, s * 0.40, s * 0.22);
    shapes::fill_round_rect(
        tc.pos.x,
        tc.pos.y,
        tc.size.x,
        tc.size.y,
        s * 0.06,
        ui.palette.warning,
    );
    let win = with_alpha(ui.palette.on_accent, 0.8);
    for i in 0..2 {
        shapes::fill_round_rect(
            tc.pos.x + s * 0.06 + i as f32 * s * 0.12,
            tc.pos.y + s * 0.05,
            s * 0.08,
            s * 0.10,
            s * 0.02,
            win,
        );
    }
}

pub fn draw_settings(ui: &mut UiContext, layout: &Layout, settings: &Settings, scroll: f32) -> f32 {
    draw_background(ui, layout);
    let w = layout.window.x;
    let h = layout.window.y;
    draw_rectangle(0.0, 0.0, w, h, with_alpha(ui.palette.bg_top, 0.72));

    let inner_w = (w - 48.0).min(880.0);
    let x = (w - inner_w) * 0.5;
    let mut y = 88.0 - scroll;
    let gap = 14.0;
    let on = ui.tr("Вкл", "On");
    let off = ui.tr("Выкл", "Off");

    // ── Interface ───────────────────────────────────────────────
    let iface_h = 50.0 + 4.0 * 40.0 + 54.0 + 14.0;
    let (cx, mut cy, cw) = card(
        ui,
        Rect::new(x, y, inner_w, iface_h),
        ui.tr("Интерфейс", "Interface"),
        Icon::Settings,
    );
    let half = (cw - 16.0) * 0.5;
    let toggles = [
        (
            ui.tr("Подписи станций", "Station labels"),
            settings.show_labels,
            UiAction::ToggleLabels,
        ),
        (
            ui.tr("Сетка карты", "Map grid"),
            settings.show_grid,
            UiAction::ToggleGrid,
        ),
        (
            ui.tr("Пассажиры у станций", "Waiting passengers"),
            settings.show_waiting,
            UiAction::ToggleWaiting,
        ),
        (
            ui.tr("Полные анимации", "Full animations"),
            !settings.reduced_motion,
            UiAction::ToggleMotion,
        ),
        (
            ui.tr("Нагрузка станций", "Station load"),
            settings.show_heat,
            UiAction::ToggleHeat,
        ),
        (
            ui.tr("Поток по линиям", "Line flow"),
            settings.show_flow,
            UiAction::ToggleFlow,
        ),
        (
            ui.tr("Автосохранение", "Autosave"),
            settings.autosave,
            UiAction::ToggleAutosave,
        ),
    ];
    for (i, (label, value, action)) in toggles.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let tx = cx + col as f32 * (half + 16.0);
        let ty = cy + row as f32 * 40.0;
        text::draw(
            ui.fonts,
            label,
            Vec2::new(tx, ty + 7.0),
            14.0,
            ui.palette.text_primary,
            false,
        );
        let tg = Rect::new(tx + half - 64.0, ty, 64.0, 30.0);
        ui.toggle(
            tg,
            if *value { on } else { off },
            *value,
            true,
            action.clone(),
        );
    }
    cy += toggles.len().div_ceil(2) as f32 * 40.0 + 8.0;
    let effective = if settings.ui_scale > 0.0 {
        settings.ui_scale
    } else {
        crate::config::settings::auto_ui_scale()
    };
    slider_row(
        ui,
        cx,
        cy,
        cw,
        ui.tr("Масштаб интерфейса", "UI scale"),
        &format!("{}%", (effective * 100.0).round() as u32),
        effective,
        0.8,
        3.0,
        UiAction::SetUiScale,
    );
    y += iface_h + gap;

    // ── Consist ─────────────────────────────────────────────────
    let consist_h = 50.0 + 3.0 * 48.0 + 42.0 + 14.0;
    let (cx, mut cy, cw) = card(
        ui,
        Rect::new(x, y, inner_w, consist_h),
        ui.tr("Состав поезда", "Train consist"),
        Icon::Layers,
    );
    slider_row(
        ui,
        cx,
        cy,
        cw,
        ui.tr("Вагонов, мин", "Cars, min"),
        &settings.cars_min.to_string(),
        settings.cars_min as f32,
        1.0,
        6.0,
        |v| UiAction::SetCarsMin(v.round() as u32),
    );
    cy += 48.0;
    slider_row(
        ui,
        cx,
        cy,
        cw,
        ui.tr("Вагонов, макс", "Cars, max"),
        &settings.cars_max.to_string(),
        settings.cars_max as f32,
        1.0,
        8.0,
        |v| UiAction::SetCarsMax(v.round() as u32),
    );
    cy += 48.0;
    slider_row(
        ui,
        cx,
        cy,
        cw,
        ui.tr("Вместимость вагона", "Car capacity"),
        &settings.car_capacity.to_string(),
        settings.car_capacity as f32,
        10.0,
        120.0,
        |v| UiAction::SetCarCapacity((v / 5.0).round() as u32 * 5),
    );
    cy += 48.0;
    text::draw(
        ui.fonts,
        ui.tr("Авто-состав по нагрузке", "Auto-consist by load"),
        Vec2::new(cx, cy + 7.0),
        14.0,
        ui.palette.text_primary,
        false,
    );
    ui.toggle(
        Rect::new(cx + cw - 64.0, cy, 64.0, 30.0),
        if settings.auto_cars { on } else { off },
        settings.auto_cars,
        true,
        UiAction::ToggleAutoCars,
    );
    y += consist_h + gap;

    // ── Trains ──────────────────────────────────────────────────
    let trains_h = 50.0 + 2.0 * 48.0 + 42.0 + 14.0;
    let (cx, mut cy, cw) = card(
        ui,
        Rect::new(x, y, inner_w, trains_h),
        ui.tr("Поезда", "Trains"),
        Icon::Route,
    );
    slider_row(
        ui,
        cx,
        cy,
        cw,
        ui.tr("Поездов на направление, мин", "Trains per direction, min"),
        &settings.trains_min.to_string(),
        settings.trains_min as f32,
        1.0,
        6.0,
        |v| UiAction::SetTrainsMin(v.round() as u32),
    );
    cy += 48.0;
    slider_row(
        ui,
        cx,
        cy,
        cw,
        ui.tr("Поездов на направление, макс", "Trains per direction, max"),
        &settings.trains_max.to_string(),
        settings.trains_max as f32,
        1.0,
        8.0,
        |v| UiAction::SetTrainsMax(v.round() as u32),
    );
    cy += 48.0;
    text::draw(
        ui.fonts,
        ui.tr("Авто-поезда по загрузке", "Auto-trains by load"),
        Vec2::new(cx, cy + 7.0),
        14.0,
        ui.palette.text_primary,
        false,
    );
    ui.toggle(
        Rect::new(cx + cw - 64.0, cy, 64.0, 30.0),
        if settings.auto_trains { on } else { off },
        settings.auto_trains,
        true,
        UiAction::ToggleAutoTrains,
    );
    y += trains_h + gap;

    // ── Difficulty ──────────────────────────────────────────────
    let diff_h = 50.0 + 46.0 + 48.0 + 14.0;
    let (cx, mut cy, cw) = card(
        ui,
        Rect::new(x, y, inner_w, diff_h),
        ui.tr("Пассажиропоток (сложность)", "Passenger flow (difficulty)"),
        Icon::Chart,
    );
    let labels: Vec<&str> = crate::config::settings::FlowLevel::ALL
        .iter()
        .map(|l| match ui.lang {
            crate::config::i18n::Lang::Ru => l.label(),
            crate::config::i18n::Lang::En => l.label_en(),
        })
        .collect();
    ui.segmented(
        Rect::new(cx, cy, cw, 40.0),
        &labels,
        settings.flow_level.index(),
        UiAction::SetFlow,
    );
    cy += 46.0;
    slider_row(
        ui,
        cx,
        cy,
        cw,
        ui.tr("Рост потока за день", "Flow growth per day"),
        &format!("{}%", (settings.flow_growth * 100.0).round() as u32),
        settings.flow_growth * 100.0,
        0.0,
        100.0,
        |v| UiAction::SetFlowGrowth(v / 100.0),
    );
    y += diff_h + gap;

    // ── Theme ───────────────────────────────────────────────────
    let theme_h = 50.0 + 44.0 + 14.0;
    let (cx, cy, cw) = card(
        ui,
        Rect::new(x, y, inner_w, theme_h),
        ui.tr("Тема оформления", "Theme"),
        Icon::Palette,
    );
    let theme_labels: Vec<&str> = crate::config::palette::Theme::ALL
        .iter()
        .map(|t| match ui.lang {
            crate::config::i18n::Lang::Ru => t.label(),
            crate::config::i18n::Lang::En => t.label_en(),
        })
        .collect();
    ui.segmented(
        Rect::new(cx, cy, cw, 40.0),
        &theme_labels,
        settings.theme.index(),
        UiAction::SetTheme,
    );
    y += theme_h + 8.0;

    // ── Language ────────────────────────────────────────────────
    let lang_h = 50.0 + 44.0 + 14.0;
    let (cx, cy, cw) = card(
        ui,
        Rect::new(x, y, inner_w, lang_h),
        ui.tr("Язык", "Language"),
        Icon::Tag,
    );
    let lang_labels: Vec<&str> = crate::config::i18n::Lang::ALL
        .iter()
        .map(|l| l.label())
        .collect();
    ui.segmented(
        Rect::new(cx, cy, cw, 40.0),
        &lang_labels,
        settings.language.index(),
        UiAction::SetLanguage,
    );
    y += lang_h + 8.0;

    let content_h = y - (88.0 - scroll);

    draw_rectangle(0.0, 0.0, w, 64.0, ui.palette.panel);
    text::draw(
        ui.fonts,
        ui.tr("Настройки", "Settings"),
        Vec2::new(x, 20.0),
        26.0,
        ui.palette.text_primary,
        true,
    );
    let back_rect = Rect::new(x + inner_w - 170.0, 16.0, 170.0, 34.0);
    ui.icon_text_button(
        back_rect,
        Icon::Back,
        ui.tr("К игре", "Back"),
        ButtonStyle::Secondary,
        true,
        UiAction::CloseSettings,
    );
    draw_line(0.0, 64.0, w, 64.0, 1.5, ui.palette.border);
    content_h
}

pub fn draw_about(ui: &mut UiContext, layout: &Layout, scroll: f32) -> f32 {
    draw_background(ui, layout);
    let w = layout.window.x;
    let h = layout.window.y;
    draw_rectangle(0.0, 0.0, w, h, with_alpha(ui.palette.bg_top, 0.72));

    let inner_w = (w - 48.0).min(760.0);
    let x = (w - inner_w) * 0.5;
    let mut y = 92.0 - scroll;
    let gap = 14.0;

    // Hero
    let hero_h = 150.0;
    shapes::fill_round_rect(x, y, inner_w, hero_h, 14.0, ui.palette.panel);
    shapes::stroke_round_rect(x, y, inner_w, hero_h, 14.0, 1.5, ui.palette.border);
    draw_logo(ui, Vec2::new(x + 70.0, y + hero_h * 0.5), 40.0);
    text::draw(
        ui.fonts,
        "MetroFlow",
        Vec2::new(x + 140.0, y + 30.0),
        34.0,
        ui.palette.text_primary,
        true,
    );
    text::draw(
        ui.fonts,
        ui.tr(
            "Минималистичный симулятор метрополитена",
            "A minimalist metro simulator",
        ),
        Vec2::new(x + 140.0, y + 66.0),
        14.0,
        ui.palette.text_secondary,
        false,
    );
    text::draw(
        ui.fonts,
        &ui.tf(
            "Версия {v}",
            "Version {v}",
            &[("v", env!("CARGO_PKG_VERSION"))],
        ),
        Vec2::new(x + 140.0, y + 92.0),
        13.0,
        ui.palette.text_muted,
        false,
    );
    y += hero_h + gap;

    // Author
    let (cx, cy, cw) = card(
        ui,
        Rect::new(x, y, inner_w, 120.0),
        ui.tr("Разработчик", "Developer"),
        Icon::Person,
    );
    text::draw(
        ui.fonts,
        &format!("{DEVELOPER}  {HANDLE}"),
        Vec2::new(cx, cy),
        20.0,
        ui.palette.text_primary,
        true,
    );
    text::draw(
        ui.fonts,
        ui.tr(
            "Связь и обновления — GitHub и Telegram.",
            "Reach out and updates — GitHub and Telegram.",
        ),
        Vec2::new(cx, cy + 30.0),
        13.0,
        ui.palette.text_muted,
        false,
    );
    let bw = 150.0;
    ui.action_button(
        Rect::new(cx + cw - bw * 2.0 - 10.0, cy - 2.0, bw, 38.0),
        ui.tr("GitHub", "GitHub"),
        ButtonStyle::Primary,
        true,
        UiAction::OpenLink(GITHUB_URL.to_string()),
    );
    ui.action_button(
        Rect::new(cx + cw - bw, cy - 2.0, bw, 38.0),
        ui.tr("Telegram", "Telegram"),
        ButtonStyle::Secondary,
        true,
        UiAction::OpenLink(TELEGRAM_URL.to_string()),
    );
    y += 120.0 + gap;

    // Features (+ License) + How it works
    let wide = inner_w >= 720.0;
    let col_gap = 14.0;
    let col_w = if wide {
        (inner_w - col_gap) * 0.5
    } else {
        inner_w
    };
    let feat_h = 50.0 + FEATURES.len() as f32 * 22.0 + 14.0;

    let mut m_lines: Vec<(bool, String)> = Vec::new();
    let mw = col_w - 48.0;
    for (ru, en) in MECHANICS {
        let mut first = true;
        for l in wrap_lines(ui, ui.tr(ru, en), mw, 12.0) {
            m_lines.push((first, l));
            first = false;
        }
    }
    let mec_h = 50.0 + m_lines.len() as f32 * 17.0 + 14.0;
    // Equal-height columns: the license card absorbs the leftover space.
    let min_lic = 140.0;
    let col_h = if wide {
        mec_h.max(feat_h + gap + min_lic)
    } else {
        mec_h
    };
    let lic_h = (col_h - feat_h - gap).max(min_lic);

    let (fcx, mut fcy, _) = card(
        ui,
        Rect::new(x, y, col_w, feat_h),
        ui.tr("Возможности", "Features"),
        Icon::Check,
    );
    for (ru, en) in FEATURES {
        draw_circle(fcx + 4.0, fcy + 7.0, 3.0, ui.palette.accent);
        text::draw(
            ui.fonts,
            ui.tr(ru, en),
            Vec2::new(fcx + 16.0, fcy),
            14.0,
            ui.palette.text_secondary,
            false,
        );
        fcy += 22.0;
    }

    let (lcx, lcy, _) = card(
        ui,
        Rect::new(x, y + feat_h + gap, col_w, lic_h),
        ui.tr("Лицензия", "License"),
        Icon::Check,
    );
    text::draw(
        ui.fonts,
        "PolyForm Noncommercial 1.0.0",
        Vec2::new(lcx, lcy),
        15.0,
        ui.palette.text_primary,
        true,
    );
    text::draw(
        ui.fonts,
        ui.tr(
            "Исходники открыты. Коммерческое использование запрещено.",
            "Source available. Commercial use is not permitted.",
        ),
        Vec2::new(lcx, lcy + 22.0),
        12.0,
        ui.palette.text_muted,
        false,
    );
    text::draw(
        ui.fonts,
        "polyformproject.org/licenses/noncommercial/1.0.0",
        Vec2::new(lcx, lcy + 40.0),
        11.0,
        ui.palette.accent_strong,
        false,
    );
    text::draw(
        ui.fonts,
        ui.tr(
            "Copyright (c) 2026 German Vitiaz",
            "Copyright (c) 2026 German Vitiaz",
        ),
        Vec2::new(lcx, lcy + 58.0),
        12.0,
        ui.palette.text_muted,
        false,
    );

    let mx = if wide { x + col_w + col_gap } else { x };
    let my = if wide {
        y
    } else {
        y + feat_h + gap + lic_h + gap
    };
    let mcard_h = if wide { col_h } else { mec_h };
    let (mcx, mut mcy, _) = card(
        ui,
        Rect::new(mx, my, col_w, mcard_h),
        ui.tr("Как это считается", "How it works"),
        Icon::Chart,
    );
    for (first, line) in &m_lines {
        if *first {
            draw_circle(mcx + 4.0, mcy + 6.0, 3.0, ui.palette.accent);
        }
        text::draw(
            ui.fonts,
            line,
            Vec2::new(mcx + 16.0, mcy),
            12.0,
            ui.palette.text_secondary,
            false,
        );
        mcy += 17.0;
    }

    let left_h = feat_h + gap + lic_h;
    y += if wide {
        left_h.max(mec_h)
    } else {
        left_h + gap + mec_h
    };
    y += gap;

    let content_h = y - (92.0 - scroll);

    draw_rectangle(0.0, 0.0, w, 64.0, ui.palette.panel);
    text::draw(
        ui.fonts,
        ui.tr("О проекте", "About"),
        Vec2::new(x, 20.0),
        26.0,
        ui.palette.text_primary,
        true,
    );
    ui.icon_text_button(
        Rect::new(x + inner_w - 170.0, 16.0, 170.0, 34.0),
        Icon::Back,
        ui.tr("Назад", "Back"),
        ButtonStyle::Secondary,
        true,
        UiAction::CloseAbout,
    );
    draw_line(0.0, 64.0, w, 64.0, 1.5, ui.palette.border);
    content_h
}

pub fn draw_new_setup(
    ui: &mut UiContext,
    layout: &Layout,
    params: &crate::network::GenerateParams,
    settings: &Settings,
    scroll: f32,
) -> f32 {
    draw_background(ui, layout);
    let w = layout.window.x;
    let h = layout.window.y;
    draw_rectangle(0.0, 0.0, w, h, with_alpha(ui.palette.bg_top, 0.72));

    let inner_w = (w - 48.0).min(760.0);
    let x = (w - inner_w) * 0.5;
    let mut y = 88.0 - scroll;

    let (cx, mut cy, cw) = card(
        ui,
        Rect::new(x, y, inner_w, 50.0 + 3.0 * 44.0 + 34.0 + 14.0),
        ui.tr("Карта", "Map"),
        Icon::Grid,
    );
    stepper(
        ui,
        cx,
        cy,
        cw,
        ui.tr("Станций", "Stations"),
        params.stations,
        UiAction::AdjustStations(-1),
        UiAction::AdjustStations(1),
    );
    cy += 44.0;
    stepper(
        ui,
        cx,
        cy,
        cw,
        ui.tr("Веток (линий)", "Lines"),
        params.lines,
        UiAction::AdjustLines(-1),
        UiAction::AdjustLines(1),
    );
    cy += 44.0;
    stepper(
        ui,
        cx,
        cy,
        cw,
        ui.tr("Узловых станций (пересадки)", "Interchanges (hubs)"),
        params.hubs,
        UiAction::AdjustHubs(-1),
        UiAction::AdjustHubs(1),
    );
    cy += 40.0;
    let max_hubs = if params.lines >= 2 {
        params.stations.saturating_sub(2).min(params.lines)
    } else {
        0
    };
    text::draw(
        ui.fonts,
        &ui.tf(
            "Узлы: макс {n} при {l} линиях и {s} станциях.",
            "Hubs: max {n} for {l} lines and {s} stations.",
            &[
                ("n", &max_hubs.to_string()),
                ("l", &params.lines.to_string()),
                ("s", &params.stations.to_string()),
            ],
        ),
        Vec2::new(cx, cy),
        12.0,
        ui.palette.text_muted,
        false,
    );
    y += 50.0 + 3.0 * 44.0 + 34.0 + 14.0 + 14.0;

    let (cx, mut cy, cw) = card(
        ui,
        Rect::new(x, y, inner_w, 50.0 + 2.0 * 44.0 + 14.0),
        ui.tr("Поезда", "Trains"),
        Icon::Route,
    );
    stepper(
        ui,
        cx,
        cy,
        cw,
        ui.tr("Поездов в одну сторону, мин", "Trains per direction, min"),
        settings.trains_min,
        UiAction::AdjustTrainsMin(-1),
        UiAction::AdjustTrainsMin(1),
    );
    cy += 44.0;
    stepper(
        ui,
        cx,
        cy,
        cw,
        ui.tr("Поездов в одну сторону, макс", "Trains per direction, max"),
        settings.trains_max,
        UiAction::AdjustTrainsMax(-1),
        UiAction::AdjustTrainsMax(1),
    );
    y += 50.0 + 2.0 * 44.0 + 14.0 + 16.0;

    let gap = 12.0;
    let bw = (inner_w - gap * 2.0) / 3.0;
    let cancel = Rect::new(x, y, bw, 46.0);
    let random = Rect::new(x + bw + gap, y, bw, 46.0);
    let create = Rect::new(x + (bw + gap) * 2.0, y, bw, 46.0);
    ui.action_button(
        cancel,
        ui.tr("Отмена", "Cancel"),
        ButtonStyle::Ghost,
        true,
        UiAction::CloseSetup,
    );
    ui.action_button(
        random,
        ui.tr("Случайная", "Random"),
        ButtonStyle::Secondary,
        true,
        UiAction::GenerateRandom,
    );
    ui.action_button(
        create,
        ui.tr("Создать", "Create"),
        ButtonStyle::Primary,
        true,
        UiAction::GenerateMap,
    );
    y += 46.0;

    let content_h = y - (88.0 - scroll);

    draw_rectangle(0.0, 0.0, w, 70.0, ui.palette.panel);
    text::draw(
        ui.fonts,
        ui.tr("Новая симуляция", "New simulation"),
        Vec2::new(x, 18.0),
        26.0,
        ui.palette.text_primary,
        true,
    );
    text::draw(
        ui.fonts,
        ui.tr(
            "Уникальная карта генерируется по параметрам.",
            "A unique map is generated from these parameters.",
        ),
        Vec2::new(x, 48.0),
        13.0,
        ui.palette.text_muted,
        false,
    );
    draw_line(0.0, 70.0, w, 70.0, 1.5, ui.palette.border);
    content_h
}

#[allow(clippy::too_many_arguments)]
fn stepper(
    ui: &mut UiContext,
    x: f32,
    y: f32,
    w: f32,
    label: &str,
    value: u32,
    dec: UiAction,
    inc: UiAction,
) {
    text::draw(
        ui.fonts,
        label,
        Vec2::new(x, y + 6.0),
        14.0,
        ui.palette.text_primary,
        false,
    );
    let box_x = x + w - 118.0;
    let minus = Rect::new(box_x, y, 34.0, 30.0);
    let plus = Rect::new(box_x + 84.0, y, 34.0, 30.0);
    ui.icon_button(minus, Icon::Minus, false, true, dec);
    ui.icon_button(plus, Icon::Plus, false, true, inc);
    text::draw_centered(
        ui.fonts,
        &value.to_string(),
        Vec2::new(box_x + 59.0, y + 15.0),
        18.0,
        ui.palette.text_primary,
        true,
    );
}
