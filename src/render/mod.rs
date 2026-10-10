pub mod map;
pub mod scale;
pub mod shapes;
pub mod text;

use crate::geom::Vec2;
use crate::network::{LineId, StationId};
use macroquad::prelude::Color;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Pulse {
    pub pos: Vec2,
    pub color: Color,
    pub age: f32,
    pub life: f32,
    pub max_radius: f32,
}

pub struct RenderState {
    pub time: f32,
    pub selection: Option<StationId>,
    pub hover: Option<StationId>,
    pub pulses: Vec<Pulse>,
    pub line_load: HashMap<LineId, f32>,
}

impl Default for RenderState {
    fn default() -> Self {
        RenderState {
            time: 0.0,
            selection: None,
            hover: None,
            pulses: Vec::new(),
            line_load: HashMap::new(),
        }
    }
}

impl RenderState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        for pulse in &mut self.pulses {
            pulse.age += dt;
        }
        self.pulses.retain(|p| p.age < p.life);
    }

    pub fn push_pulse(&mut self, pos: Vec2, color: Color, reduced_motion: bool) {
        if reduced_motion {
            return;
        }
        self.pulses.push(Pulse {
            pos,
            color,
            age: 0.0,
            life: 0.55,
            max_radius: 14.0,
        });
    }
}
