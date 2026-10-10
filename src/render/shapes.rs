use crate::geom::Vec2;
use macroquad::prelude::*;

pub fn mq(v: Vec2) -> macroquad::prelude::Vec2 {
    macroquad::prelude::vec2(v.x, v.y)
}

pub fn round_rect_points(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    segments_per_corner: usize,
) -> Vec<macroquad::prelude::Vec2> {
    let r = radius.min(w * 0.5).min(h * 0.5).max(0.0);
    let segs = segments_per_corner.max(2);
    let mut points = Vec::with_capacity(segs * 4 + 4);
    let corners = [
        (x + w - r, y + r, -std::f32::consts::FRAC_PI_2),
        (x + w - r, y + h - r, 0.0),
        (x + r, y + h - r, std::f32::consts::FRAC_PI_2),
        (x + r, y + r, std::f32::consts::PI),
    ];
    for (cx, cy, start) in corners {
        for i in 0..=segs {
            let a = start + std::f32::consts::FRAC_PI_2 * i as f32 / segs as f32;
            points.push(macroquad::prelude::vec2(cx + a.cos() * r, cy + a.sin() * r));
        }
    }
    points
}

pub fn fill_round_rect(x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color) {
    if radius <= 0.5 {
        draw_rectangle(x, y, w, h, color);
        return;
    }
    let points = round_rect_points(x, y, w, h, radius, 6);
    fill_convex_polygon(&points, color);
}

pub fn stroke_round_rect(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    radius: f32,
    thickness: f32,
    color: Color,
) {
    if radius <= 0.5 {
        draw_rectangle_lines(x, y, w, h, thickness, color);
        return;
    }
    let points = round_rect_points(x, y, w, h, radius, 6);
    stroke_polygon(&points, thickness, color);
}

pub fn fill_convex_polygon(points: &[macroquad::prelude::Vec2], color: Color) {
    if points.len() < 3 {
        return;
    }
    let mut center = macroquad::prelude::vec2(0.0, 0.0);
    for p in points {
        center += *p;
    }
    center /= points.len() as f32;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        draw_triangle(center, a, b, color);
    }
}

pub fn stroke_polygon(points: &[macroquad::prelude::Vec2], thickness: f32, color: Color) {
    if points.len() < 2 {
        return;
    }
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        draw_line(a.x, a.y, b.x, b.y, thickness, color);
    }
    for p in points {
        draw_circle(p.x, p.y, thickness * 0.5, color);
    }
}

pub fn stroke_polyline(points: &[macroquad::prelude::Vec2], thickness: f32, color: Color) {
    if points.len() < 2 {
        return;
    }
    for seg in points.windows(2) {
        draw_line(seg[0].x, seg[0].y, seg[1].x, seg[1].y, thickness, color);
    }
    if thickness > 2.0 {
        for p in points {
            draw_circle(p.x, p.y, thickness * 0.5, color);
        }
    }
}

pub fn stroke_polyline_dashed(
    points: &[macroquad::prelude::Vec2],
    thickness: f32,
    color: Color,
    pattern: &[(f32, f32)],
) {
    if points.len() < 2 || pattern.is_empty() {
        return;
    }
    let draw_dot = |p: macroquad::prelude::Vec2| {
        draw_circle(p.x, p.y, thickness * 0.5, color);
    };
    let mut pattern_index = 0usize;
    let mut is_on = true;
    let mut remaining = pattern[0].0;
    for seg in points.windows(2) {
        let a = seg[0];
        let b = seg[1];
        let len = (b - a).length();
        if len <= f32::EPSILON {
            continue;
        }
        let dir = (b - a) / len;
        let mut travelled = 0.0;
        while travelled < len {
            let take = remaining.min(len - travelled);
            let from = a + dir * travelled;
            let to = a + dir * (travelled + take);
            if is_on {
                draw_line(from.x, from.y, to.x, to.y, thickness, color);
                if thickness > 2.5 {
                    draw_dot(from);
                    draw_dot(to);
                }
            }
            travelled += take;
            remaining -= take;
            if remaining <= 1e-3 {
                let pair = pattern[pattern_index % pattern.len()];
                is_on = !is_on;
                remaining = if is_on { pair.0 } else { pair.1 };
                pattern_index += 1;
                if remaining <= 0.0 {
                    is_on = !is_on;
                    remaining = if is_on { pair.0 } else { pair.1 };
                }
            }
        }
    }
}

fn rounded_rect_local(hx: f32, hy: f32, radius: f32, segs: usize) -> Vec<macroquad::prelude::Vec2> {
    let r = radius.min(hx).min(hy).max(0.0);
    let segs = segs.max(2);
    let mut pts = Vec::with_capacity(segs * 4 + 4);
    let corners = [
        (hx - r, -(hy - r), -std::f32::consts::FRAC_PI_2),
        (hx - r, hy - r, 0.0),
        (-(hx - r), hy - r, std::f32::consts::FRAC_PI_2),
        (-(hx - r), -(hy - r), std::f32::consts::PI),
    ];
    for (cx, cy, start) in corners {
        for i in 0..=segs {
            let a = start + std::f32::consts::FRAC_PI_2 * i as f32 / segs as f32;
            pts.push(macroquad::prelude::vec2(cx + a.cos() * r, cy + a.sin() * r));
        }
    }
    pts
}

pub fn fill_rounded_rotated_rect(
    center: macroquad::prelude::Vec2,
    size: macroquad::prelude::Vec2,
    angle: f32,
    radius: f32,
    color: Color,
) -> Vec<macroquad::prelude::Vec2> {
    let local = rounded_rect_local(size.x * 0.5, size.y * 0.5, radius, 6);
    let (s, c) = angle.sin_cos();
    let world: Vec<macroquad::prelude::Vec2> = local
        .iter()
        .map(|p| center + macroquad::prelude::vec2(p.x * c - p.y * s, p.x * s + p.y * c))
        .collect();
    fill_convex_polygon(&world, color);
    world
}

pub fn soft_glow(center: macroquad::prelude::Vec2, radius: f32, color: Color, layers: u32) {
    let layers = layers.max(1);
    for i in 0..layers {
        let t = i as f32 / layers as f32;
        let r = radius * (1.0 - t * 0.6);
        let a = color.a * (1.0 - t) * 0.04;
        draw_circle(
            center.x,
            center.y,
            r,
            Color::new(color.r, color.g, color.b, a),
        );
    }
}
