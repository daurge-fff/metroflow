use crate::camera::Camera;
use crate::config::palette::{mix, shade, with_alpha, Palette};
use crate::config::{Fonts, Settings};
use crate::geom::Vec2 as GVec2;
use crate::network::{LineId, Network, StationId, StationKind};
use crate::render::shapes::{self, stroke_polygon};
use crate::render::{text, RenderState};
use crate::simulation::Simulation;
use macroquad::prelude::*;

pub struct MapContext<'a> {
    pub network: &'a Network,
    pub sim: &'a Simulation,
    pub camera: &'a Camera,
    pub palette: &'a Palette,
    pub fonts: &'a Fonts,
    pub settings: &'a Settings,
    pub state: &'a RenderState,
    pub active_line: Option<LineId>,
    pub daylight: f32,
    pub route: Option<&'a crate::network::Route>,
    pub cursor: GVec2,
}

const TRACK_WIDTH: f32 = 9.0;
const TRACK_CASING_WIDTH: f32 = 15.0;
const LABEL_ZOOM_THRESHOLD: f32 = 0.5;
pub const ALERT_THRESHOLD: u32 = 24;

pub fn draw(ctx: &MapContext) {
    draw_background(ctx);
    if ctx.settings.show_grid {
        draw_grid(ctx);
    }
    draw_tracks(ctx);
    draw_flow(ctx);
    draw_route(ctx);
    draw_station_markers(ctx);
    draw_waiting(ctx);
    draw_labels(ctx);
    draw_alerts(ctx);
    draw_trains(ctx);
    draw_pulses(ctx);
    draw_hover_tooltip(ctx);
}

fn world_to_screen(ctx: &MapContext, world: GVec2) -> macroquad::prelude::Vec2 {
    shapes::mq(ctx.camera.world_to_screen(world))
}

fn draw_background(ctx: &MapContext) {
    let vp = ctx.camera.viewport;
    let night = shade(ctx.palette.map_bg, 0.42);
    let bg = mix(night, ctx.palette.map_bg, ctx.daylight.clamp(0.0, 1.0));
    draw_rectangle(vp.pos.x, vp.pos.y, vp.size.x, vp.size.y, bg);
}

fn draw_grid(ctx: &MapContext) {
    let vp = ctx.camera.viewport;
    let tl = ctx.camera.world_from_screen(vp.pos);
    let br = ctx
        .camera
        .world_from_screen(GVec2::new(vp.right(), vp.bottom()));
    if !tl.is_finite() || !br.is_finite() {
        return;
    }

    let mut spacing = 100.0;
    while spacing * ctx.camera.zoom < 24.0 && spacing < 3200.0 {
        spacing *= 2.0;
    }

    let start_x = (tl.x / spacing).floor() * spacing;
    let start_y = (tl.y / spacing).floor() * spacing;
    let dot_r = (1.6 * ctx.camera.zoom).clamp(1.0, 2.4);

    let mut y = start_y;
    while y <= br.y {
        let mut x = start_x;
        while x <= br.x {
            let s = world_to_screen(ctx, GVec2::new(x, y));
            let grid = with_alpha(
                ctx.palette.map_grid,
                ctx.palette.map_grid.a * (0.45 + 0.55 * ctx.daylight.clamp(0.0, 1.0)),
            );
            draw_circle(s.x, s.y, dot_r, grid);
            x += spacing;
        }
        y += spacing;
    }
}

fn line_screen_points(
    ctx: &MapContext,
    line: &crate::network::Line,
) -> Vec<macroquad::prelude::Vec2> {
    let mut pts: Vec<macroquad::prelude::Vec2> = line
        .stations
        .iter()
        .filter_map(|id| ctx.network.station(*id))
        .map(|s| world_to_screen(ctx, s.pos))
        .collect();
    if line.is_loop && pts.len() >= 2 {
        if let Some(first) = ctx
            .network
            .station(line.stations[0])
            .map(|s| world_to_screen(ctx, s.pos))
        {
            if pts.first() != Some(&first) {
                pts.push(first);
            }
        }
    }
    pts
}

fn track_width(ctx: &MapContext, line: &crate::network::Line) -> f32 {
    if !ctx.settings.show_flow {
        return TRACK_WIDTH;
    }
    let load = ctx.state.line_load.get(&line.id).copied().unwrap_or(0.0);
    TRACK_WIDTH + (load * 0.45).min(7.0)
}

fn draw_tracks(ctx: &MapContext) {
    for line in &ctx.network.lines {
        let pts = line_screen_points(ctx, line);
        let width = track_width(ctx, line) + 6.0;
        shapes::stroke_polyline(&pts, width, ctx.palette.track_casing);
    }
    if let Some(active) = ctx.active_line {
        if let Some(line) = ctx.network.line(active) {
            let pts = line_screen_points(ctx, line);
            shapes::stroke_polyline(
                &pts,
                TRACK_CASING_WIDTH + 10.0,
                with_alpha(ctx.palette.accent, 0.30),
            );
        }
    }
    for line in &ctx.network.lines {
        let pts = line_screen_points(ctx, line);
        let color = Color::from_rgba(line.color[0], line.color[1], line.color[2], 255);
        let width = track_width(ctx, line);
        match line.style {
            crate::config::palette::LineStyle::Solid => {
                shapes::stroke_polyline(&pts, width, color);
            }
            style => {
                let pattern = scale_pattern(style.dash_pattern(), width);
                shapes::stroke_polyline_dashed(&pts, width, color, &pattern);
            }
        }
    }
}

fn draw_flow(ctx: &MapContext) {
    if !ctx.settings.show_flow {
        return;
    }
    let color = |line: &crate::network::Line| {
        let base = Color::from_rgba(line.color[0], line.color[1], line.color[2], 255);
        mix(base, Color::new(1.0, 1.0, 1.0, 1.0), 0.4)
    };
    for line in &ctx.network.lines {
        let c = color(line);
        for i in 0..line.segment_count() {
            let Some((a, b)) = line.segment_endpoints(i) else {
                continue;
            };
            let (Some(sa), Some(sb)) = (ctx.network.station(a), ctx.network.station(b)) else {
                continue;
            };
            let pa = world_to_screen(ctx, sa.pos);
            let pb = world_to_screen(ctx, sb.pos);
            let d = pb - pa;
            let len = d.length();
            if len < 12.0 {
                continue;
            }
            let u = d / len;
            let perp = macroquad::prelude::vec2(-u.y, u.x);
            let spacing = 46.0;
            let mut t = (ctx.state.time * 48.0) % spacing;
            while t < len {
                let p = pa + u * t;
                let sz = 4.5;
                let p1 = p + perp * sz;
                let p3 = p - perp * sz;
                let tip = p + u * (sz * 1.5);
                draw_line(p1.x, p1.y, tip.x, tip.y, 1.6, c);
                draw_line(tip.x, tip.y, p3.x, p3.y, 1.6, c);
                t += spacing;
            }
        }
    }
}

fn draw_route(ctx: &MapContext) {
    let Some(route) = ctx.route else {
        return;
    };
    let glow = with_alpha(ctx.palette.accent_strong, 0.5);
    for (i, leg) in route.legs.iter().enumerate() {
        let pts: Vec<macroquad::prelude::Vec2> = leg
            .stations
            .iter()
            .filter_map(|id| ctx.network.station(*id))
            .map(|s| world_to_screen(ctx, s.pos))
            .collect();
        shapes::stroke_polyline(&pts, TRACK_WIDTH * 0.55, glow);
        for p in &pts {
            draw_circle(p.x, p.y, 3.0, ctx.palette.accent_strong);
        }
        if i + 1 < route.legs.len() {
            if let Some(last) = pts.last() {
                draw_circle_lines(last.x, last.y, 8.0, 2.5, ctx.palette.warning);
            }
        }
    }
}

fn scale_pattern(pattern: &[(f32, f32)], width: f32) -> Vec<(f32, f32)> {
    let k = (width / 9.0).clamp(0.8, 1.6);
    pattern.iter().map(|(a, b)| (a * k, b * k)).collect()
}

fn station_radius(kind: StationKind) -> f32 {
    match kind {
        StationKind::Regular => 8.0,
        StationKind::Interchange => 9.5,
        StationKind::Central => 12.0,
    }
}

fn draw_station_markers(ctx: &MapContext) {
    for station in &ctx.network.stations {
        let center = world_to_screen(ctx, station.pos);
        let r = station_radius(station.kind);
        draw_ellipse(
            center.x,
            center.y + r * 0.7,
            r * 1.7,
            r * 0.7,
            0.0,
            Color::new(0.0, 0.0, 0.0, 0.18),
        );
        if ctx.settings.show_heat {
            let waiting = ctx.sim.passengers.waiting_at(station.id) as f32;
            let heat = (waiting / ALERT_THRESHOLD as f32).clamp(0.0, 1.0);
            if heat > 0.02 {
                let color = if heat < 0.5 {
                    mix(ctx.palette.success, ctx.palette.warning, heat * 2.0)
                } else {
                    mix(ctx.palette.warning, ctx.palette.danger, (heat - 0.5) * 2.0)
                };
                shapes::soft_glow(center, r + 20.0, color, 7);
                draw_circle(center.x, center.y, r + 3.0, with_alpha(color, 0.22 * heat));
            }
        }
        if ctx.state.selection == Some(station.id) {
            let pulse = selection_pulse(ctx, r);
            draw_circle_lines(center.x, center.y, pulse, 2.5, ctx.palette.accent);
        } else if ctx.state.hover == Some(station.id) {
            draw_circle_lines(
                center.x,
                center.y,
                r + 7.0,
                2.0,
                with_alpha(ctx.palette.accent, 0.8),
            );
        }
    }
    for station in &ctx.network.stations {
        let center = world_to_screen(ctx, station.pos);
        let r = station_radius(station.kind);
        match station.kind {
            StationKind::Regular => {
                draw_circle(
                    center.x,
                    center.y,
                    r + 2.5,
                    with_alpha(ctx.palette.station_border, 0.18),
                );
                draw_circle(center.x, center.y, r, ctx.palette.station_fill);
                draw_circle_lines(center.x, center.y, r, 3.0, ctx.palette.station_border);
                if station.lines.len() == 1 {
                    if let Some(line) = ctx.network.line(station.lines[0]) {
                        let color =
                            Color::from_rgba(line.color[0], line.color[1], line.color[2], 255);
                        draw_circle(center.x, center.y, r * 0.4, color);
                    }
                }
            }
            StationKind::Interchange => {
                shapes::fill_round_rect(
                    center.x - r,
                    center.y - r,
                    r * 2.0,
                    r * 2.0,
                    4.0,
                    ctx.palette.station_fill,
                );
                shapes::stroke_round_rect(
                    center.x - r,
                    center.y - r,
                    r * 2.0,
                    r * 2.0,
                    4.0,
                    3.0,
                    ctx.palette.station_border,
                );
            }
            StationKind::Central => {
                shapes::soft_glow(center, r + 18.0, ctx.palette.accent, 6);
                draw_circle(center.x, center.y, r, ctx.palette.station_fill);
                draw_circle_lines(center.x, center.y, r, 4.0, ctx.palette.station_border);
                draw_circle_lines(center.x, center.y, r * 0.5, 2.0, ctx.palette.accent);
            }
        }
    }

    for station in &ctx.network.stations {
        if station.lines.len() < 2 {
            continue;
        }
        let center = world_to_screen(ctx, station.pos);
        let r = station_radius(station.kind);
        let n = station.lines.len() as f32;
        for (i, line_id) in station.lines.iter().enumerate() {
            if let Some(line) = ctx.network.line(*line_id) {
                let color = Color::from_rgba(line.color[0], line.color[1], line.color[2], 255);
                let angle = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / n;
                let p = macroquad::prelude::vec2(
                    center.x + angle.cos() * (r + 6.0),
                    center.y + angle.sin() * (r + 6.0),
                );
                draw_circle(p.x, p.y, 2.6, color);
            }
        }
    }
}

fn draw_hover_tooltip(ctx: &MapContext) {
    let Some(id) = ctx.state.hover else {
        return;
    };
    let Some(station) = ctx.network.station(id) else {
        return;
    };
    let lines: Vec<String> = station
        .lines
        .iter()
        .filter_map(|l| ctx.network.line(*l))
        .map(|l| l.name.clone())
        .collect();
    let waiting = ctx.sim.passengers.waiting_at(id);
    let avg = ctx
        .sim
        .passengers
        .average_wait(id, ctx.sim.now())
        .map(|w| format!("{w:.0} с"))
        .unwrap_or_else(|| "—".to_string());

    let line2 = format!("Ожидают: {waiting} · Среднее: {avg}");
    let line3 = if lines.is_empty() {
        String::new()
    } else {
        lines.join(" · ")
    };
    let title_size = 14.0;
    let small_size = 12.0;
    let w = text::measure(ctx.fonts, &station.name, title_size, true)
        .x
        .max(text::measure(ctx.fonts, &line2, small_size, false).x)
        .max(text::measure(ctx.fonts, &line3, small_size, false).x)
        + 24.0;
    let h = if line3.is_empty() { 52.0 } else { 68.0 };
    let pos = macroquad::prelude::vec2(ctx.cursor.x + 16.0, ctx.cursor.y + 16.0);
    shapes::fill_round_rect(pos.x, pos.y, w, h, 8.0, with_alpha(ctx.palette.panel, 0.97));
    shapes::stroke_round_rect(pos.x, pos.y, w, h, 8.0, 1.5, ctx.palette.border_strong);
    text::draw(
        ctx.fonts,
        &station.name,
        GVec2::new(pos.x + 12.0, pos.y + 8.0),
        title_size,
        ctx.palette.text_primary,
        true,
    );
    text::draw(
        ctx.fonts,
        &line2,
        GVec2::new(pos.x + 12.0, pos.y + 28.0),
        small_size,
        ctx.palette.text_secondary,
        false,
    );
    if !line3.is_empty() {
        text::draw(
            ctx.fonts,
            &line3,
            GVec2::new(pos.x + 12.0, pos.y + 46.0),
            small_size,
            ctx.palette.accent_strong,
            false,
        );
    }
}

fn selection_pulse(ctx: &MapContext, base_r: f32) -> f32 {
    if ctx.settings.reduced_motion {
        return base_r + 8.0;
    }
    let t = (ctx.state.time * 2.0).sin() * 0.5 + 0.5;
    base_r + 7.0 + t * 5.0
}

fn draw_waiting(ctx: &MapContext) {
    if !ctx.settings.show_waiting {
        return;
    }
    for station in &ctx.network.stations {
        let waiting = ctx.sim.passengers.waiting_at(station.id);
        if waiting == 0 {
            continue;
        }
        let shown = waiting.min(10) as usize;
        let center = world_to_screen(ctx, station.pos);
        let r = station_radius(station.kind) + 7.0;
        let start = -std::f32::consts::FRAC_PI_2 - 0.5;
        for i in 0..shown {
            let angle = start + i as f32 * 0.22;
            let p =
                macroquad::prelude::vec2(center.x + angle.cos() * r, center.y - angle.sin() * r);
            draw_circle(p.x, p.y, 2.4, ctx.palette.station_waiting);
        }
        if waiting > 10 {
            let label = format!("+{}", waiting - 10);
            text::draw_with_shadow(
                ctx.fonts,
                &label,
                GVec2::new(center.x + r + 2.0, center.y - 14.0),
                12.0,
                ctx.palette.station_waiting,
                ctx.palette.map_bg,
                false,
            );
        }
    }
}

fn draw_labels(ctx: &MapContext) {
    if !ctx.settings.show_labels || ctx.camera.zoom < LABEL_ZOOM_THRESHOLD {
        return;
    }
    for station in &ctx.network.stations {
        let center = world_to_screen(ctx, station.pos);
        let r = station_radius(station.kind);
        let size = ctx.camera.zoom.clamp(0.7, 1.2) * 15.0;
        let bold = matches!(station.kind, StationKind::Central);
        let dims = text::measure(ctx.fonts, &station.name, size, bold);
        let top_left = GVec2::new(center.x - dims.x * 0.5, center.y - r - dims.y - 6.0);
        text::draw_with_shadow(
            ctx.fonts,
            &station.name,
            top_left,
            size,
            ctx.palette.text_primary,
            with_alpha(ctx.palette.map_bg, 0.9),
            bold,
        );
    }
}

fn draw_alerts(ctx: &MapContext) {
    for station in &ctx.network.stations {
        let waiting = ctx.sim.passengers.waiting_at(station.id);
        if waiting < ALERT_THRESHOLD {
            continue;
        }
        let center = world_to_screen(ctx, station.pos);
        let r = station_radius(station.kind);
        let pulse = if ctx.settings.reduced_motion {
            0.0
        } else {
            ((ctx.state.time * 3.0).sin() * 0.5 + 0.5) * 4.0
        };
        draw_circle_lines(
            center.x,
            center.y,
            r + 6.0 + pulse,
            2.5,
            with_alpha(ctx.palette.danger, 0.85),
        );
        let bx = center.x + r + 6.0;
        let by = center.y - r - 6.0;
        draw_circle(bx, by, 9.0, ctx.palette.danger);
        text::draw_centered(
            ctx.fonts,
            "!",
            GVec2::new(bx, by),
            15.0,
            Color::new(1.0, 1.0, 1.0, 1.0),
            true,
        );
    }
}

fn draw_trains(ctx: &MapContext) {
    let scale = ctx.camera.zoom.clamp(0.55, 1.35);
    for train in &ctx.sim.trains {
        let (Some(world), Some(line)) = (train.position(ctx.network), ctx.network.line(train.line))
        else {
            continue;
        };
        let center = world_to_screen(ctx, world);
        let angle = train.heading(ctx.network);
        let base = Color::from_rgba(line.color[0], line.color[1], line.color[2], 255);
        let load = train.load_factor();
        let body = crate::config::palette::mix(base, Color::new(1.0, 1.0, 1.0, 1.0), load * 0.18);

        let (s, c) = angle.sin_cos();
        let place = |offx: f32, offy: f32| {
            macroquad::prelude::vec2(
                center.x + offx * c - offy * s,
                center.y + offx * s + offy * c,
            )
        };

        let cars = train.cars.max(1);
        let car_len = 17.0 * scale;
        let gap = 3.0 * scale;
        let width = 12.0 * scale;
        let radius = width * 0.42;
        let car_size = macroquad::prelude::vec2(car_len, width);
        let offsets: Vec<f32> = (0..cars)
            .map(|i| (i as f32 - (cars as f32 - 1.0) * 0.5) * (car_len + gap))
            .collect();

        for &offset in &offsets {
            let cc = place(offset, 0.0);
            shapes::fill_rounded_rotated_rect(
                macroquad::prelude::vec2(cc.x, cc.y + 2.5),
                car_size,
                angle,
                radius,
                Color::new(0.0, 0.0, 0.0, 0.25),
            );
        }

        for pair in offsets.windows(2) {
            let a = place(pair[0] + car_len * 0.5, 0.0);
            let b = place(pair[1] - car_len * 0.5, 0.0);
            draw_line(a.x, a.y, b.x, b.y, 3.0, shade(body, 0.6));
        }

        for &offset in &offsets {
            let car_center = place(offset, 0.0);
            let points =
                shapes::fill_rounded_rotated_rect(car_center, car_size, angle, radius, body);
            stroke_polygon(&points, 1.8, ctx.palette.station_border);

            let stripe = macroquad::prelude::vec2(car_len * 0.62, width * 0.14);
            shapes::fill_rounded_rotated_rect(
                place(offset, -width * 0.26),
                stripe,
                angle,
                stripe.y * 0.5,
                with_alpha(Color::new(1.0, 1.0, 1.0, 1.0), 0.35),
            );

            for wx in [-car_len * 0.24, car_len * 0.24] {
                let wp = place(offset + wx, width * 0.08);
                draw_circle(
                    wp.x,
                    wp.y,
                    1.3 * scale,
                    with_alpha(ctx.palette.track_casing, 0.55),
                );
            }
        }

        let front_offset = offsets.last().copied().unwrap_or(0.0);
        let nose = place(front_offset + car_len * 0.5 - radius * 0.6, 0.0);
        draw_circle(
            nose.x,
            nose.y,
            radius * 0.5,
            with_alpha(Color::new(1.0, 1.0, 1.0, 1.0), 0.9),
        );

        if let crate::simulation::TrainState::Dwelling { remaining } = train.state {
            let progress = if train.dwell_total > 0.0 {
                (1.0 - remaining / train.dwell_total).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let base = macroquad::prelude::vec2(center.x, center.y - width * 0.9 - 9.0);
            for i in 0..3 {
                let dx = base.x - 8.0 + i as f32 * 8.0;
                let filled = (i as f32 + 0.5) / 3.0 <= progress;
                let color = if filled {
                    ctx.palette.accent_strong
                } else {
                    with_alpha(ctx.palette.border, 0.8)
                };
                draw_circle(dx, base.y, 2.2, color);
            }
        }
    }
}

fn draw_pulses(ctx: &MapContext) {
    for pulse in &ctx.state.pulses {
        let t = (pulse.age / pulse.life).clamp(0.0, 1.0);
        let r = pulse.max_radius * (0.3 + 0.7 * t);
        let alpha = (1.0 - t) * 0.7;
        let center = world_to_screen(ctx, pulse.pos);
        draw_circle_lines(
            center.x,
            center.y,
            6.0 + r,
            2.5,
            with_alpha(pulse.color, alpha),
        );
    }
}

pub fn station_at_screen(
    network: &Network,
    camera: &Camera,
    screen: GVec2,
    tolerance: f32,
) -> Option<StationId> {
    let mut best: Option<(f32, StationId)> = None;
    for station in &network.stations {
        let s = camera.world_to_screen(station.pos);
        let d = s.distance(screen);
        let radius = station_radius(station.kind) + tolerance;
        if d <= radius && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, station.id));
        }
    }
    best.map(|(_, id)| id)
}
