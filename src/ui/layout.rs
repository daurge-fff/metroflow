use crate::geom::{Rect, Vec2};

pub struct Layout {
    pub window: Vec2,
    pub top_bar: Rect,
    pub side_panel: Rect,
    pub map_viewport: Rect,
    pub controls: Rect,
}

impl Layout {
    pub fn compute(width: f32, height: f32) -> Self {
        let top_h = 66.0;

        let mut panel_w = (width * 0.26).clamp(292.0, 372.0);
        if width < 900.0 {
            panel_w = 272.0;
        }
        if width < 720.0 {
            panel_w = (width * 0.42).clamp(220.0, 272.0);
        }
        panel_w = panel_w.min(width * 0.5);

        let map_w = (width - panel_w).max(200.0);

        let controls_w = 232.0;
        let controls_h = 178.0;
        let controls = Rect::new(
            16.0,
            (height - controls_h - 16.0).max(top_h + 8.0),
            controls_w,
            controls_h,
        );

        Layout {
            window: Vec2::new(width, height),
            top_bar: Rect::new(0.0, 0.0, width, top_h),
            side_panel: Rect::new(width - panel_w, top_h, panel_w, height - top_h),
            map_viewport: Rect::new(0.0, top_h, map_w, height - top_h),
            controls,
        }
    }
}
