use crate::geom::{Rect, Vec2};

#[derive(Clone, Debug)]
pub struct Camera {
    pub target: Vec2,
    pub zoom: f32,
    pub target_zoom: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
    pub viewport: Rect,
    pub default_target: Vec2,
    pub default_zoom: f32,
    pub momentum: Vec2,
    zoom_anchor: Option<(Vec2, Vec2)>,
}

impl Camera {
    pub fn new(viewport: Rect) -> Self {
        Camera {
            target: Vec2::ZERO,
            zoom: 1.0,
            target_zoom: 1.0,
            min_zoom: 0.25,
            max_zoom: 3.0,
            viewport,
            default_target: Vec2::ZERO,
            default_zoom: 1.0,
            momentum: Vec2::ZERO,
            zoom_anchor: None,
        }
    }

    pub fn world_to_screen(&self, world: Vec2) -> Vec2 {
        (world - self.target) * self.zoom + self.viewport.center()
    }

    pub fn world_from_screen(&self, screen: Vec2) -> Vec2 {
        (screen - self.viewport.center()) / self.zoom + self.target
    }

    pub fn pan_by_screen(&mut self, delta: Vec2) {
        self.target = self.target - delta / self.zoom;
        self.zoom_anchor = None;
    }

    pub fn zoom_by(&mut self, factor: f32, screen_anchor: Vec2) {
        let anchor_world = self.world_from_screen(screen_anchor);
        let new_target = (self.target_zoom * factor).clamp(self.min_zoom, self.max_zoom);
        if (new_target - self.target_zoom).abs() > f32::EPSILON {
            self.target_zoom = new_target;
            self.zoom_anchor = Some((screen_anchor, anchor_world));
        }
    }

    pub fn update(&mut self, dt: f32, reduced_motion: bool) {
        let rate = if reduced_motion { 1.0 } else { 14.0 };
        let t = 1.0 - (-rate * dt).exp();
        self.zoom += (self.target_zoom - self.zoom) * t;
        if (self.target_zoom - self.zoom).abs() < 1e-3 {
            self.zoom = self.target_zoom;
        }
        if let Some((screen_anchor, world_anchor)) = self.zoom_anchor {
            let now_world = self.world_from_screen(screen_anchor);
            self.target = self.target + (world_anchor - now_world);
            if (self.zoom - self.target_zoom).abs() < 1e-3 {
                self.zoom_anchor = None;
            }
        }

        if self.momentum.length_squared() > 4.0 {
            self.target = self.target - self.momentum * dt / self.zoom;
            let damp = (-(if reduced_motion { 14.0 } else { 5.0 }) * dt).exp();
            self.momentum = self.momentum * damp;
            if self.momentum.length() < 12.0 {
                self.momentum = Vec2::ZERO;
            }
        }
    }

    pub fn reset(&mut self) {
        self.target = self.default_target;
        self.target_zoom = self.default_zoom;
        self.zoom = self.default_zoom;
        self.zoom_anchor = None;
        self.momentum = Vec2::ZERO;
    }

    pub fn center(&mut self) {
        self.target = self.default_target;
        self.zoom_anchor = None;
        self.momentum = Vec2::ZERO;
    }

    pub fn fit(&mut self, bounds: Option<Rect>) {
        let Some(bounds) = bounds else {
            self.default_target = Vec2::ZERO;
            self.default_zoom = 1.0;
            self.reset();
            return;
        };
        if !bounds.pos.is_finite() || !bounds.size.is_finite() {
            self.default_target = Vec2::ZERO;
            self.default_zoom = 1.0;
            self.reset();
            return;
        }
        let margins = 120.0;
        let world_w = (bounds.size.x + margins * 2.0).max(1.0);
        let world_h = (bounds.size.y + margins * 2.0).max(1.0);
        let vw = self.viewport.size.x.max(1.0);
        let vh = self.viewport.size.y.max(1.0);
        let fit_zoom = (vw / world_w).min(vh / world_h);
        let center = bounds.center();
        self.default_target = center;
        self.default_zoom = fit_zoom.clamp(self.min_zoom, self.max_zoom);
        self.reset();
    }

    pub fn clamp_to(&mut self, bounds: Option<Rect>) {
        let Some(bounds) = bounds else { return };
        let half = self.viewport.size / (2.0 * self.zoom);
        let pad = 160.0;
        let min_x = bounds.pos.x - pad + half.x.min(bounds.size.x * 0.5);
        let max_x = bounds.right() + pad - half.x.min(bounds.size.x * 0.5);
        let min_y = bounds.pos.y - pad + half.y.min(bounds.size.y * 0.5);
        let max_y = bounds.bottom() + pad - half.y.min(bounds.size.y * 0.5);
        if min_x <= max_x {
            self.target.x = self.target.x.clamp(min_x, max_x);
        }
        if min_y <= max_y {
            self.target.y = self.target.y.clamp(min_y, max_y);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cam() -> Camera {
        let mut c = Camera::new(Rect::new(0.0, 0.0, 800.0, 600.0));
        c.target = Vec2::new(100.0, 100.0);
        c.zoom = 1.5;
        c.target_zoom = 1.5;
        c
    }

    #[test]
    fn world_screen_roundtrip() {
        let c = cam();
        let p = Vec2::new(250.0, 330.0);
        let screen = c.world_to_screen(p);
        let back = c.world_from_screen(screen);
        assert!((back.x - p.x).abs() < 1e-3 && (back.y - p.y).abs() < 1e-3);
    }

    #[test]
    fn pan_moves_target_by_scaled_delta() {
        let mut c = cam();
        let before = c.target;
        c.pan_by_screen(Vec2::new(15.0, -30.0));
        assert!((c.target.x - (before.x - 10.0)).abs() < 1e-3);
        assert!((c.target.y - (before.y + 20.0)).abs() < 1e-3);
    }

    #[test]
    fn zoom_is_clamped() {
        let mut c = cam();
        c.zoom_by(100.0, Vec2::new(0.0, 0.0));
        assert!(c.target_zoom <= c.max_zoom);
        c.zoom_by(0.0001, Vec2::new(0.0, 0.0));
        assert!(c.target_zoom >= c.min_zoom);
    }
}
