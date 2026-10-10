/// Kinetic scroll with momentum and rubber-band overscroll. `offset` may sit
/// outside `[0, max]` (drawn as-is); it springs back when released.
pub struct Scroll {
    pub offset: f32,
    pub max: f32,
    velocity: f32,
    last: f32,
    dragging: bool,
}

impl Default for Scroll {
    fn default() -> Self {
        Self::new()
    }
}

impl Scroll {
    pub fn new() -> Self {
        Scroll {
            offset: 0.0,
            max: 0.0,
            velocity: 0.0,
            last: 0.0,
            dragging: false,
        }
    }

    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    pub fn start_drag(&mut self, y: f32) {
        self.dragging = true;
        self.last = y;
        self.velocity = 0.0;
    }

    pub fn drag_to(&mut self, y: f32, dt: f32) {
        if !self.dragging {
            return;
        }
        let dy = y - self.last;
        self.last = y;
        let over = self.offset < 0.0 || self.offset > self.max;
        let factor = if over { 0.38 } else { 1.0 };
        self.offset -= dy * factor;
        if dt > 1e-4 {
            let sample = -dy / dt;
            self.velocity = self.velocity * 0.55 + sample * 0.45;
            self.velocity = self.velocity.clamp(-7000.0, 7000.0);
        }
    }

    pub fn end_drag(&mut self) {
        self.dragging = false;
    }

    pub fn nudge(&mut self, delta: f32) {
        self.offset += delta;
        self.velocity = 0.0;
    }

    pub fn update(&mut self, dt: f32) {
        if self.dragging {
            return;
        }
        if self.velocity.abs() > 1.0 {
            self.offset += self.velocity * dt;
            self.velocity *= (-6.0 * dt).exp();
        } else {
            self.velocity = 0.0;
        }

        let k = 1.0 - (-16.0 * dt).exp();
        if self.offset < 0.0 {
            self.offset += (0.0 - self.offset) * k;
            self.velocity *= 0.5;
            if self.offset > -0.5 {
                self.offset = 0.0;
            }
        } else if self.offset > self.max {
            self.offset += (self.max - self.offset) * k;
            self.velocity *= 0.5;
            if self.offset < self.max + 0.5 {
                self.offset = self.max;
            }
        }
    }

    pub fn reset(&mut self) {
        self.offset = 0.0;
        self.velocity = 0.0;
        self.dragging = false;
    }
}
