use crate::geom::Vec2;
use crate::network::{LineId, Network, StationId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct TrainId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum TrainState {
    Moving,
    Dwelling { remaining: f32 },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Train {
    pub id: TrainId,
    pub line: LineId,
    pub segment: usize,
    pub direction: i8,
    pub progress: f32,
    pub speed: f32,
    pub state: TrainState,
    #[serde(default = "one")]
    pub cars: u32,
    pub capacity: u32,
    pub onboard: Vec<StationId>,
    #[serde(default)]
    pub dwell_total: f32,
}

fn one() -> u32 {
    1
}

#[derive(Clone, Copy, Debug)]
pub struct TrainPolicy {
    pub car_capacity: u32,
    pub cars_min: u32,
    pub cars_max: u32,
    pub auto: bool,
}

impl TrainPolicy {
    pub fn capacity(&self, cars: u32) -> u32 {
        cars.max(1) * self.car_capacity.max(1)
    }
}

impl Train {
    pub fn new(id: TrainId, line: LineId, cars: u32, car_capacity: u32) -> Self {
        let cars = cars.max(1);
        Train {
            id,
            line,
            segment: 0,
            direction: 1,
            progress: 0.0,
            speed: 150.0,
            state: TrainState::Moving,
            cars,
            capacity: cars * car_capacity.max(1),
            onboard: Vec::new(),
            dwell_total: 0.0,
        }
    }

    pub fn set_cars(&mut self, cars: u32, policy: &TrainPolicy) {
        let mut cars = cars.clamp(policy.cars_min.max(1), policy.cars_max.max(1));
        while self.onboard.len() as u32 > policy.capacity(cars) && cars < policy.cars_max {
            cars += 1;
        }
        self.cars = cars;
        self.capacity = policy.capacity(cars);
    }

    pub fn adjust_cars(&mut self, policy: &TrainPolicy) {
        if !policy.auto {
            return;
        }
        let load = self.onboard.len() as f32 / self.capacity.max(1) as f32;
        if load > 0.85 && self.cars < policy.cars_max {
            self.set_cars(self.cars + 1, policy);
        } else if load < 0.3 && self.cars > policy.cars_min {
            self.set_cars(self.cars - 1, policy);
        }
    }

    pub fn onboard_count(&self) -> usize {
        self.onboard.len()
    }

    pub fn position(&self, network: &Network) -> Option<Vec2> {
        let line = network.line(self.line)?;
        let (a_id, b_id) = line.segment_endpoints(self.segment)?;
        let a = network.station(a_id)?.pos;
        let b = network.station(b_id)?.pos;
        let t = if self.direction >= 0 {
            self.progress
        } else {
            1.0 - self.progress
        };
        Some(a.lerp(b, t.clamp(0.0, 1.0)))
    }

    pub fn heading(&self, network: &Network) -> f32 {
        let Some(line) = network.line(self.line) else {
            return 0.0;
        };
        let Some((a_id, b_id)) = line.segment_endpoints(self.segment) else {
            return 0.0;
        };
        let (Some(a), Some(b)) = (network.station(a_id), network.station(b_id)) else {
            return 0.0;
        };
        let dir = if self.direction >= 0 {
            (b.pos - a.pos).normalized()
        } else {
            (a.pos - b.pos).normalized()
        };
        dir.angle()
    }

    pub fn load_factor(&self) -> f32 {
        if self.capacity == 0 {
            0.0
        } else {
            self.onboard.len() as f32 / self.capacity as f32
        }
    }
}
