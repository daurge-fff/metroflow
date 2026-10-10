pub mod clock;
pub mod passengers;
pub mod rng;
pub mod train;

pub use clock::SimClock;
pub use passengers::{PassengerModel, ServiceResult};
pub use rng::Rng;
pub use train::{Train, TrainId, TrainPolicy, TrainState};

use crate::network::{LineId, Network, Route, Routing, StationId};
use serde::{Deserialize, Serialize};

fn default_car_capacity() -> u32 {
    30
}
fn default_cars_min() -> u32 {
    2
}
fn default_cars_max() -> u32 {
    4
}
fn default_true() -> bool {
    true
}
fn default_trains_min() -> u32 {
    1
}
fn default_trains_max() -> u32 {
    3
}
fn default_flow_growth() -> f32 {
    0.0
}

#[derive(Clone, Copy, Debug)]
pub enum SimEvent {
    Arrive { station: StationId, line: LineId },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Simulation {
    pub clock: SimClock,
    pub trains: Vec<Train>,
    pub passengers: PassengerModel,
    pub rng: Rng,
    next_train_id: u32,
    #[serde(skip)]
    pub events: Vec<SimEvent>,
    #[serde(skip)]
    routing: Routing,
    #[serde(default = "default_car_capacity")]
    pub car_capacity: u32,
    #[serde(default = "default_cars_min")]
    pub cars_min: u32,
    #[serde(default = "default_cars_max")]
    pub cars_max: u32,
    #[serde(default = "default_true")]
    pub auto_cars: bool,
    #[serde(default = "default_trains_min")]
    pub trains_min: u32,
    #[serde(default = "default_trains_max")]
    pub trains_max: u32,
    #[serde(default = "default_true")]
    pub auto_trains: bool,
    #[serde(default = "default_flow_growth")]
    pub flow_growth: f32,
    /// Running cost, charged for cars above the minimum configuration.
    #[serde(default)]
    pub operational_cost: f64,
    #[serde(skip)]
    balance_timer: f32,
}

impl Simulation {
    pub fn new(network: &Network) -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x1234_5678_9ABC_DEF0);
        Self::new_with_seed(network, seed)
    }

    pub fn new_with_seed(network: &Network, seed: u64) -> Self {
        let mut sim = Simulation {
            clock: SimClock::default(),
            trains: Vec::new(),
            passengers: PassengerModel::new(),
            rng: Rng::new(seed),
            next_train_id: 0,
            events: Vec::new(),
            routing: network.routing(),
            car_capacity: default_car_capacity(),
            cars_min: default_cars_min(),
            cars_max: default_cars_max(),
            auto_cars: true,
            trains_min: default_trains_min(),
            trains_max: default_trains_max(),
            auto_trains: true,
            flow_growth: default_flow_growth(),
            operational_cost: 0.0,
            balance_timer: 0.0,
        };

        for line in &network.lines {
            let segs = line.segment_count();
            if segs == 0 {
                continue;
            }
            let count = sim.trains_min.max(1) as usize;
            let directions: &[i8] = if line.is_loop { &[1] } else { &[1, -1] };
            for &direction in directions {
                for i in 0..count {
                    let mut train = Train::new(
                        TrainId(sim.next_train_id),
                        line.id,
                        sim.cars_min,
                        sim.car_capacity,
                    );
                    sim.next_train_id += 1;
                    train.segment = (i * segs) / count;
                    train.direction = direction;
                    train.progress = ((i as f32 + 0.5) / count as f32).clamp(0.05, 0.95);
                    train.speed = line.base_speed;
                    sim.trains.push(train);
                }
            }
        }
        sim
    }

    fn spawn_train_on(&mut self, network: &Network, line: crate::network::LineId, direction: i8) {
        let Some(l) = network.line(line) else { return };
        let segs = l.segment_count();
        if segs == 0 {
            return;
        }
        let mut occupancy = vec![0u32; segs];
        for t in self.trains.iter().filter(|t| t.line == line) {
            if t.segment < segs {
                occupancy[t.segment] += 1;
            }
        }
        let segment = occupancy
            .iter()
            .enumerate()
            .min_by_key(|(_, c)| **c)
            .map(|(i, _)| i)
            .unwrap_or(0);
        let here = self
            .trains
            .iter()
            .filter(|t| t.line == line && t.segment == segment)
            .count();
        let mut train = Train::new(
            TrainId(self.next_train_id),
            line,
            self.cars_min.max(1),
            self.car_capacity.max(1),
        );
        self.next_train_id += 1;
        train.segment = segment;
        train.direction = if l.is_loop { 1 } else { direction };
        train.progress = (0.35 + 0.3 * here as f32).clamp(0.05, 0.9);
        train.speed = l.base_speed;
        self.trains.push(train);
    }

    /// People the line must move now: onboard plus a fair share of the
    /// passengers waiting at its stations.
    fn line_demand(&self, network: &Network, line: LineId) -> f32 {
        let onboard: u32 = self
            .trains
            .iter()
            .filter(|t| t.line == line)
            .map(|t| t.onboard_count() as u32)
            .sum();
        let mut waiting = 0.0f32;
        if let Some(l) = network.line(line) {
            for sid in &l.stations {
                let w = self
                    .passengers
                    .waiting
                    .get(sid)
                    .map(|v| v.len() as f32)
                    .unwrap_or(0.0);
                let served = network
                    .station(*sid)
                    .map(|s| s.lines.len().max(1) as f32)
                    .unwrap_or(1.0);
                waiting += w / served;
            }
        }
        onboard as f32 + waiting
    }

    pub fn balance_trains(&mut self, network: &Network) {
        let policy = self.train_policy();
        let cars_min = self.cars_min.max(1);
        let cars_max = self.cars_max.max(cars_min);
        let trains_min = self.trains_min.max(1) as usize;
        let trains_max = self.trains_max.max(self.trains_min.max(1)) as usize;

        // Normalise demand against the busiest line so consists differ.
        let reference = (cars_min * self.car_capacity.max(1) * self.trains_min.max(1) * 2) as f32;
        let demands: Vec<(LineId, f32)> = network
            .lines
            .iter()
            .map(|l| (l.id, self.line_demand(network, l.id)))
            .collect();
        let max_demand = demands.iter().map(|(_, d)| *d).fold(0.0f32, f32::max);
        let denom = max_demand.max(reference).max(1.0);

        for line in &network.lines {
            if line.segment_count() == 0 {
                continue;
            }
            let demand = demands
                .iter()
                .find(|(id, _)| *id == line.id)
                .map(|(_, d)| *d)
                .unwrap_or(0.0);
            let pressure = (demand / denom).clamp(0.0, 1.0);

            let target = if self.auto_trains {
                trains_min + ((trains_max - trains_min) as f32 * pressure).round() as usize
            } else {
                line.trains_override
                    .map(|t| (t as usize).clamp(1, 8))
                    .unwrap_or(trains_min)
            };
            // Count per line, not per direction (trains reverse mid-route).
            let dirs = if line.is_loop { 1 } else { 2 };
            let total_target = target * dirs;
            let count = self.trains.iter().filter(|t| t.line == line.id).count();
            if count < total_target {
                for _ in 0..(total_target - count) {
                    let forward = self
                        .trains
                        .iter()
                        .filter(|t| t.line == line.id && t.direction >= 0)
                        .count();
                    let backward = self
                        .trains
                        .iter()
                        .filter(|t| t.line == line.id && t.direction < 0)
                        .count();
                    let dir = if line.is_loop || backward >= forward {
                        1
                    } else {
                        -1
                    };
                    self.spawn_train_on(network, line.id, dir);
                }
            } else if count > total_target {
                let mut to_remove = count - total_target;
                let mut idx = self.trains.len();
                while idx > 0 && to_remove > 0 {
                    idx -= 1;
                    if self.trains[idx].line == line.id && self.trains[idx].onboard.is_empty() {
                        self.trains.remove(idx);
                        to_remove -= 1;
                    }
                }
            }

            let car_target = if self.auto_cars {
                cars_min + ((cars_max - cars_min) as f32 * pressure).round() as u32
            } else {
                line.cars_override
                    .map(|c| c.clamp(1, cars_max))
                    .unwrap_or(cars_min)
            };
            for train in self.trains.iter_mut().filter(|t| t.line == line.id) {
                train.set_cars(car_target, &policy);
            }
        }
    }

    pub fn train_policy(&self) -> TrainPolicy {
        TrainPolicy {
            car_capacity: self.car_capacity.max(1),
            cars_min: self.cars_min.max(1),
            cars_max: self.cars_max.max(self.cars_min.max(1)),
            auto: self.auto_cars,
        }
    }

    pub fn rebuild_routing(&mut self, network: &Network) {
        self.routing = network.routing();
    }

    pub fn apply_settings(&mut self, settings: &crate::config::Settings) {
        self.car_capacity = settings.car_capacity.max(1);
        self.cars_min = settings.cars_min.max(1);
        self.cars_max = settings.cars_max.max(self.cars_min);
        self.auto_cars = settings.auto_cars;
        self.trains_min = settings.trains_min.max(1);
        self.trains_max = settings.trains_max.max(self.trains_min);
        self.auto_trains = settings.auto_trains;
        self.flow_growth = settings.flow_growth;
        self.passengers.spawn_interval = 0.2 / settings.flow_level.multiplier();
        let policy = self.train_policy();
        for train in &mut self.trains {
            train.set_cars(train.cars, &policy);
        }
    }

    pub fn update(&mut self, network: &Network, real_dt: f32) {
        let real_dt = real_dt.clamp(0.0, 0.1);
        self.clock.tick(real_dt);
        let mut remaining = self.clock.sim_dt(real_dt);
        if remaining <= 0.0 {
            return;
        }
        let now = self.clock.elapsed;
        let total_dt = remaining;
        let growth = 1.0 + self.flow_growth * (self.clock.day().saturating_sub(1)) as f32;
        let spawn_multiplier = self.clock.spawn_multiplier() * growth;
        const MAX_STEP: f32 = 0.05;
        // Charge only for cars above the minimum configuration.
        let baseline_cars: u32 = network
            .lines
            .iter()
            .map(|l| {
                let dirs = if l.is_loop { 1 } else { 2 };
                self.cars_min.max(1) * self.trains_min.max(1) * dirs
            })
            .sum();
        let total_cars: u32 = self.trains.iter().map(|t| t.cars).sum();
        let waste_cars = total_cars.saturating_sub(baseline_cars);
        while remaining > 0.0 {
            let step = remaining.min(MAX_STEP);
            self.operational_cost += waste_cars as f64 * step as f64 * 0.5;
            self.passengers.spawn(
                step * spawn_multiplier,
                network,
                &self.routing,
                &mut self.rng,
                now,
            );
            for i in 0..self.trains.len() {
                step_train(
                    &mut self.trains[i],
                    network,
                    &self.routing,
                    step,
                    &mut self.passengers,
                    now,
                    &mut self.events,
                );
            }
            remaining -= step;
        }

        self.balance_timer -= total_dt;
        if self.balance_timer <= 0.0 {
            self.balance_trains(network);
            self.balance_timer = 2.0;
        }
    }

    pub fn reconcile(&mut self, network: &Network) {
        self.routing = network.routing();

        self.trains.retain(|t| {
            network
                .line(t.line)
                .map(|l| l.segment_count() > 0)
                .unwrap_or(false)
        });
        for train in &mut self.trains {
            if let Some(line) = network.line(train.line) {
                let segs = line.segment_count();
                if train.segment >= segs {
                    train.segment = segs.saturating_sub(1);
                    train.progress = train.progress.min(1.0);
                }
                train.speed = line.base_speed;
            }
            train
                .onboard
                .retain(|destination| network.station(*destination).is_some());
        }
        self.balance_trains(network);

        self.passengers
            .waiting
            .retain(|id, _| network.station(*id).is_some());
        for list in self.passengers.waiting.values_mut() {
            list.retain(|p| network.station(p.destination).is_some());
        }
        self.passengers
            .served
            .retain(|id, _| network.station(*id).is_some());
        self.passengers
            .line_boarded
            .retain(|id, _| network.line(*id).is_some());
    }

    pub fn line_boarded(&self, line: LineId) -> u64 {
        self.passengers
            .line_boarded
            .get(&line)
            .copied()
            .unwrap_or(0)
    }

    pub fn line_load_now(&self, line: LineId) -> f32 {
        let mut onboard = 0u32;
        let mut capacity = 0u32;
        for train in &self.trains {
            if train.line == line {
                onboard += train.onboard_count() as u32;
                capacity += train.capacity;
            }
        }
        if capacity == 0 {
            0.0
        } else {
            onboard as f32 / capacity as f32
        }
    }

    pub fn busiest_stations(&self, limit: usize) -> Vec<(StationId, u32)> {
        let mut list: Vec<(StationId, u32)> = self
            .passengers
            .waiting
            .iter()
            .map(|(id, v)| (*id, v.len() as u32))
            .filter(|(_, n)| *n > 0)
            .collect();
        list.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        list.truncate(limit);
        list
    }

    pub fn now(&self) -> f64 {
        self.clock.elapsed
    }

    pub fn plan(&self, network: &Network, from: StationId, to: StationId) -> Option<Route> {
        self.routing.plan(network, from, to)
    }

    pub fn remap_station(&mut self, from: StationId, into: StationId) {
        for train in &mut self.trains {
            for destination in &mut train.onboard {
                if *destination == from {
                    *destination = into;
                }
            }
        }
        if let Some(list) = self.passengers.waiting.remove(&from) {
            let entry = self.passengers.waiting.entry(into).or_default();
            entry.extend(list);
        }
        for list in self.passengers.waiting.values_mut() {
            for passenger in list.iter_mut() {
                if passenger.destination == from {
                    passenger.destination = into;
                }
            }
        }
        if let Some(served) = self.passengers.served.remove(&from) {
            *self.passengers.served.entry(into).or_insert(0) += served;
        }
    }

    pub fn waiting_at(&self, station: StationId) -> u32 {
        self.passengers.waiting_at(station)
    }

    pub fn total_waiting(&self) -> u64 {
        self.passengers.total_waiting()
    }

    pub fn total_onboard(&self) -> u64 {
        self.trains.iter().map(|t| t.onboard_count() as u64).sum()
    }

    pub fn total_capacity(&self) -> u64 {
        self.trains.iter().map(|t| t.capacity as u64).sum()
    }

    pub fn peak_load(&self) -> f32 {
        self.trains
            .iter()
            .map(|t| t.load_factor())
            .fold(0.0f32, f32::max)
    }

    pub fn average_load(&self) -> f32 {
        let cap = self.total_capacity();
        if cap == 0 {
            0.0
        } else {
            self.total_onboard() as f32 / cap as f32
        }
    }

    pub fn trains_serving(&self, network: &Network, station: StationId) -> usize {
        let Some(st) = network.station(station) else {
            return 0;
        };
        self.trains
            .iter()
            .filter(|t| st.lines.contains(&t.line))
            .count()
    }

    pub fn validate_against(&self, network: &Network) -> Result<(), String> {
        for train in &self.trains {
            if network.line(train.line).is_none() {
                return Err(format!(
                    "поезд {} ссылается на несуществующую линию",
                    train.id.0
                ));
            }
            let line = network.line(train.line).unwrap();
            let segs = line.segment_count();
            if segs > 0 && train.segment >= segs {
                return Err(format!("поезд {} имеет некорректный сегмент", train.id.0));
            }
            if train.onboard.len() as u32 > train.capacity {
                return Err(format!("поезд {} переполнен", train.id.0));
            }
            for destination in &train.onboard {
                if network.station(*destination).is_none() {
                    return Err(format!(
                        "поезд {} везёт пассажира на несуществующую станцию",
                        train.id.0
                    ));
                }
            }
            if !train.progress.is_finite() {
                return Err(format!("поезд {} имеет некорректный прогресс", train.id.0));
            }
        }
        for (station, destinations) in &self.passengers.waiting {
            if network.station(*station).is_none() {
                return Err(format!("пассажиры на несуществующей станции {station}"));
            }
            for passenger in destinations {
                if network.station(passenger.destination).is_none() {
                    return Err(format!(
                        "пассажир едет на несуществующую станцию {}",
                        passenger.destination
                    ));
                }
            }
        }
        Ok(())
    }
}

fn dwell_time(res: ServiceResult) -> f32 {
    (0.8 + res.boarded as f32 * 0.03 + res.alighted as f32 * 0.02).clamp(0.6, 5.0)
}

#[allow(clippy::too_many_arguments)]
fn step_train(
    train: &mut Train,
    network: &Network,
    routing: &Routing,
    dt: f32,
    passengers: &mut PassengerModel,
    now: f64,
    events: &mut Vec<SimEvent>,
) {
    let Some(line) = network.line(train.line) else {
        return;
    };
    let segs = line.segment_count();
    if segs == 0 {
        return;
    }
    if train.segment >= segs {
        train.segment = 0;
    }
    train.speed = line.base_speed;

    match train.state {
        TrainState::Moving => {
            let Some((a_id, b_id)) = line.segment_endpoints(train.segment) else {
                return;
            };
            let (Some(a), Some(b)) = (network.station(a_id), network.station(b_id)) else {
                return;
            };
            let len = a.pos.distance(b.pos);
            let arrived = if train.direction >= 0 { b_id } else { a_id };
            let arrived_end = if len <= 1e-3 {
                true
            } else {
                train.progress += train.speed * dt / len;
                train.progress >= 1.0
            };
            if arrived_end {
                train.progress = 1.0;
                let next = line
                    .station_after(train.segment, train.direction)
                    .unwrap_or(arrived);
                let res = passengers.service(arrived, next, routing, train, now);
                let dwell = dwell_time(res);
                train.dwell_total = dwell;
                train.state = TrainState::Dwelling { remaining: dwell };
                events.push(SimEvent::Arrive {
                    station: arrived,
                    line: line.id,
                });
            }
        }
        TrainState::Dwelling { remaining } => {
            let left = remaining - dt;
            if left <= 0.0 {
                let (segment, direction) = line.advance(train.segment, train.direction);
                train.segment = segment;
                train.direction = direction;
                train.progress = 0.0;
                train.state = TrainState::Moving;
            } else {
                train.state = TrainState::Dwelling { remaining: left };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::demo_network;

    #[test]
    fn spawns_trains_for_each_line() {
        let net = demo_network();
        let sim = Simulation::new_with_seed(&net, 1);
        assert!(!sim.trains.is_empty());
        for line in &net.lines {
            assert!(sim.trains.iter().any(|t| t.line == line.id));
        }
    }

    #[test]
    fn default_fleet_two_trains_two_cars_per_line() {
        let net = demo_network();
        let sim = Simulation::new_with_seed(&net, 1);
        for line in &net.lines {
            let trains: Vec<_> = sim.trains.iter().filter(|t| t.line == line.id).collect();
            assert!(trains.len() >= 2, "на линии должно быть минимум 2 поезда");
            for t in trains {
                assert!(t.cars >= 2, "состав должен быть минимум из 2 вагонов");
            }
        }
    }

    #[test]
    fn trains_run_in_both_directions() {
        let net = demo_network();
        let sim = Simulation::new_with_seed(&net, 1);
        for line in &net.lines {
            if line.is_loop {
                continue;
            }
            assert!(
                sim.trains
                    .iter()
                    .any(|t| t.line == line.id && t.direction == 1),
                "нет поезда в прямом направлении"
            );
            assert!(
                sim.trains
                    .iter()
                    .any(|t| t.line == line.id && t.direction == -1),
                "нет поезда в обратном направлении"
            );
        }
    }

    #[test]
    fn trains_move_over_time() {
        let net = demo_network();
        let mut sim = Simulation::new_with_seed(&net, 1);
        let before: Vec<_> = sim.trains.iter().map(|t| t.position(&net)).collect();
        for _ in 0..120 {
            sim.update(&net, 1.0 / 60.0);
        }
        let after: Vec<_> = sim.trains.iter().map(|t| t.position(&net)).collect();
        assert!(
            before.iter().zip(after.iter()).any(|(a, b)| a != b),
            "хотя бы один поезд должен сместиться"
        );
    }

    #[test]
    fn movement_is_frame_independent() {
        let net = demo_network();
        let mut a = Simulation::new_with_seed(&net, 5);
        let mut b = Simulation::new_with_seed(&net, 5);
        for _ in 0..120 {
            a.update(&net, 1.0 / 60.0);
        }
        for _ in 0..60 {
            b.update(&net, 1.0 / 30.0);
        }
        let pa = a.trains[0].position(&net).unwrap();
        let pb = b.trains[0].position(&net).unwrap();
        assert!(
            (pa - pb).length() < 5.0,
            "движение не должно зависеть от FPS"
        );
    }

    #[test]
    fn clock_pauses() {
        let net = demo_network();
        let mut sim = Simulation::new_with_seed(&net, 1);
        sim.clock.paused = true;
        let before = sim.trains[0].position(&net).unwrap();
        for _ in 0..60 {
            sim.update(&net, 1.0 / 60.0);
        }
        let after = sim.trains[0].position(&net).unwrap();
        assert!((before - after).length() < 1e-4);
    }

    #[test]
    fn validation_passes_for_fresh_sim() {
        let net = demo_network();
        let sim = Simulation::new_with_seed(&net, 1);
        assert!(sim.validate_against(&net).is_ok());
    }

    #[test]
    fn operational_cost_only_counts_excess() {
        let net = demo_network();
        let mut sim = Simulation::new_with_seed(&net, 1);
        sim.auto_cars = false;
        sim.auto_trains = false;
        for line in &mut sim.trains {
            line.cars = sim.cars_min.max(1);
        }
        for _ in 0..200 {
            sim.update(&net, 0.05);
        }
        assert_eq!(sim.operational_cost, 0.0);

        let policy = sim.train_policy();
        for train in &mut sim.trains {
            train.set_cars(8, &policy);
        }
        for _ in 0..50 {
            sim.update(&net, 0.05);
        }
        assert!(sim.operational_cost > 0.0);
    }

    #[test]
    fn trains_never_exceed_capacity() {
        let net = demo_network();
        let mut sim = Simulation::new_with_seed(&net, 9);
        for _ in 0..6000 {
            sim.update(&net, 1.0 / 60.0);
        }
        for t in &sim.trains {
            assert!(t.onboard.len() as u32 <= t.capacity);
        }
    }

    #[test]
    fn passengers_get_delivered_over_time() {
        let net = demo_network();
        let mut sim = Simulation::new_with_seed(&net, 3);
        sim.clock.scale = 4.0;
        for _ in 0..6000 {
            sim.update(&net, 1.0 / 60.0);
        }
        assert!(
            sim.passengers.total_delivered > 0,
            "часть пассажиров должна достичь цели"
        );
        assert!(sim.passengers.total_delivered >= sim.passengers.total_spawned / 20);
    }
}
