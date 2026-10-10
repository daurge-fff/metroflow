use crate::network::{LineId, Network, Routing, StationId, StationKind};
use crate::simulation::rng::Rng;
use crate::simulation::train::Train;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_WAITING: usize = 400;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct WaitingPassenger {
    pub destination: StationId,
    pub since: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ServiceResult {
    pub boarded: u32,
    pub alighted: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PassengerModel {
    pub waiting: BTreeMap<StationId, Vec<WaitingPassenger>>,
    pub served: BTreeMap<StationId, u64>,
    #[serde(default)]
    pub line_boarded: BTreeMap<LineId, u64>,
    pub spawn_interval: f32,
    pub total_spawned: u64,
    pub total_delivered: u64,
    pub wait_total: f64,
    pub wait_count: u64,
    #[serde(skip)]
    accumulator: f32,
}

impl Default for PassengerModel {
    fn default() -> Self {
        PassengerModel {
            waiting: BTreeMap::new(),
            served: BTreeMap::new(),
            line_boarded: BTreeMap::new(),
            spawn_interval: 0.2,
            total_spawned: 0,
            total_delivered: 0,
            wait_total: 0.0,
            wait_count: 0,
            accumulator: 0.0,
        }
    }
}

fn attractivity(kind: StationKind) -> f32 {
    match kind {
        StationKind::Regular => 1.0,
        StationKind::Interchange => 2.2,
        StationKind::Central => 3.5,
    }
}

impl PassengerModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn waiting_at(&self, id: StationId) -> u32 {
        self.waiting.get(&id).map_or(0, |v| v.len() as u32)
    }

    pub fn served_at(&self, id: StationId) -> u64 {
        self.served.get(&id).copied().unwrap_or(0)
    }

    pub fn total_waiting(&self) -> u64 {
        self.waiting.values().map(|v| v.len() as u64).sum()
    }

    pub fn average_wait(&self, id: StationId, now: f64) -> Option<f32> {
        let list = self.waiting.get(&id)?;
        if list.is_empty() {
            return None;
        }
        let sum: f64 = list.iter().map(|p| (now - p.since).max(0.0)).sum();
        Some((sum / list.len() as f64) as f32)
    }

    pub fn network_average_wait(&self, now: f64) -> Option<f32> {
        let mut sum = 0.0f64;
        let mut count = 0u64;
        for list in self.waiting.values() {
            for p in list {
                sum += (now - p.since).max(0.0);
                count += 1;
            }
        }
        if count == 0 {
            None
        } else {
            Some((sum / count as f64) as f32)
        }
    }

    pub fn average_served_wait(&self) -> Option<f32> {
        if self.wait_count == 0 {
            None
        } else {
            Some((self.wait_total / self.wait_count as f64) as f32)
        }
    }

    pub fn destination_breakdown(&self, id: StationId) -> Vec<(StationId, u32)> {
        let Some(list) = self.waiting.get(&id) else {
            return Vec::new();
        };
        let mut counts: BTreeMap<StationId, u32> = BTreeMap::new();
        for p in list {
            *counts.entry(p.destination).or_insert(0) += 1;
        }
        let mut result: Vec<(StationId, u32)> = counts.into_iter().collect();
        result.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        result
    }

    pub fn spawn(
        &mut self,
        dt: f32,
        network: &Network,
        routing: &Routing,
        rng: &mut Rng,
        now: f64,
    ) {
        if network.stations.is_empty() || self.spawn_interval <= 0.0 {
            return;
        }
        self.accumulator += dt;
        while self.accumulator >= self.spawn_interval {
            self.accumulator -= self.spawn_interval;
            let origin = weighted_station(network, rng);
            let origin_id = network.stations[origin].id;

            let candidates: Vec<(StationId, f32)> = network
                .stations
                .iter()
                .filter(|s| s.id != origin_id && routing.is_connected(origin_id, s.id))
                .map(|s| (s.id, attractivity(s.kind)))
                .collect();
            if candidates.is_empty() {
                continue;
            }
            let total: f32 = candidates.iter().map(|(_, w)| *w).sum();
            let mut pick = rng.unit() * total;
            let mut destination = candidates[candidates.len() - 1].0;
            for (id, w) in &candidates {
                pick -= *w;
                if pick <= 0.0 {
                    destination = *id;
                    break;
                }
            }

            let list = self.waiting.entry(origin_id).or_default();
            if list.len() < MAX_WAITING {
                list.push(WaitingPassenger {
                    destination,
                    since: now,
                });
                self.total_spawned += 1;
            }
        }
    }

    pub fn service(
        &mut self,
        station: StationId,
        next: StationId,
        routing: &Routing,
        train: &mut Train,
        now: f64,
    ) -> ServiceResult {
        let mut delivered = 0u32;
        let mut alighted = 0u32;
        let mut transfers: Vec<WaitingPassenger> = Vec::new();
        let mut stay: Vec<StationId> = Vec::with_capacity(train.onboard.len());

        for destination in train.onboard.drain(..) {
            if destination == station {
                delivered += 1;
                alighted += 1;
                continue;
            }
            if routing.ride_is_good(destination, station, train.line, next, Some(train.line)) {
                stay.push(destination);
            } else {
                transfers.push(WaitingPassenger {
                    destination,
                    since: now,
                });
                alighted += 1;
            }
        }
        train.onboard = stay;

        *self.served.entry(station).or_insert(0) += delivered as u64;
        self.total_delivered += delivered as u64;

        if !transfers.is_empty() {
            let list = self.waiting.entry(station).or_default();
            for p in transfers {
                if list.len() < MAX_WAITING {
                    list.push(p);
                }
            }
        }

        let mut boarded = 0u32;
        let mut space = train.capacity as usize - train.onboard.len();
        let mut remaining = Vec::new();
        if let Some(list) = self.waiting.remove(&station) {
            for passenger in list {
                if space > 0
                    && routing.ride_is_good(passenger.destination, station, train.line, next, None)
                {
                    train.onboard.push(passenger.destination);
                    self.wait_total += (now - passenger.since).max(0.0);
                    self.wait_count += 1;
                    boarded += 1;
                    space -= 1;
                } else {
                    remaining.push(passenger);
                }
            }
        }
        if !remaining.is_empty() {
            self.waiting.insert(station, remaining);
        }
        if boarded > 0 {
            *self.line_boarded.entry(train.line).or_insert(0) += boarded as u64;
        }

        ServiceResult { boarded, alighted }
    }
}

fn weighted_station(network: &Network, rng: &mut Rng) -> usize {
    let total: f32 = network.stations.iter().map(|s| attractivity(s.kind)).sum();
    if total <= 0.0 {
        return 0;
    }
    let mut pick = rng.unit() * total;
    for (i, s) in network.stations.iter().enumerate() {
        pick -= attractivity(s.kind);
        if pick <= 0.0 {
            return i;
        }
    }
    network.stations.len() - 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::{demo_network, LineId, Network, StationKind};
    use crate::simulation::train::TrainId;

    #[test]
    fn passengers_spawn_with_reachable_destination() {
        let net = demo_network();
        let routing = net.routing();
        let mut model = PassengerModel::new();
        let mut rng = Rng::new(1);
        model.spawn(20.0, &net, &routing, &mut rng, 0.0);
        assert!(model.total_waiting() > 0);
        for (origin, passengers) in &model.waiting {
            for p in passengers {
                assert_ne!(origin, &p.destination);
                assert!(routing.is_connected(*origin, p.destination));
            }
        }
    }

    #[test]
    fn board_only_when_helpful() {
        let mut net = Network::new("t");
        let a = net.add_station("A", crate::geom::Vec2::ZERO, StationKind::Regular);
        let b = net.add_station(
            "B",
            crate::geom::Vec2::new(200.0, 0.0),
            StationKind::Regular,
        );
        let c = net.add_station(
            "C",
            crate::geom::Vec2::new(400.0, 0.0),
            StationKind::Regular,
        );
        let line = net.add_line("L", 0);
        net.append_station(line, a).unwrap();
        net.append_station(line, b).unwrap();
        net.append_station(line, c).unwrap();
        let routing = net.routing();

        let mut model = PassengerModel::new();
        model.waiting.insert(
            b,
            vec![WaitingPassenger {
                destination: c,
                since: 0.0,
            }],
        );
        let mut train = Train::new(TrainId(0), LineId(0), 1, 50);
        let res = model.service(b, c, &routing, &mut train, 3.0);
        assert_eq!(res.boarded, 1);
        assert_eq!(train.onboard.len(), 1);
        assert!(model.average_served_wait().is_some());

        let mut model2 = PassengerModel::new();
        model2.waiting.insert(
            b,
            vec![WaitingPassenger {
                destination: a,
                since: 0.0,
            }],
        );
        let mut train2 = Train::new(TrainId(1), LineId(0), 1, 50);
        let res2 = model2.service(b, c, &routing, &mut train2, 0.0);
        assert_eq!(res2.boarded, 0);
    }

    #[test]
    fn delivers_to_destination() {
        let net = demo_network();
        let routing = net.routing();
        let mut model = PassengerModel::new();
        let destination = net.stations[1].id;
        let mut train = Train::new(TrainId(0), LineId(0), 1, 50);
        train.onboard.push(destination);
        let res = model.service(destination, destination, &routing, &mut train, 0.0);
        assert_eq!(res.alighted, 1);
        assert!(train.onboard.is_empty());
        assert_eq!(model.served_at(destination), 1);
    }

    #[test]
    fn breakdown_and_wait_time() {
        let net = demo_network();
        let mut model = PassengerModel::new();
        let station = net.stations[0].id;
        model.waiting.insert(
            station,
            vec![
                WaitingPassenger {
                    destination: net.stations[1].id,
                    since: 0.0,
                },
                WaitingPassenger {
                    destination: net.stations[1].id,
                    since: 4.0,
                },
                WaitingPassenger {
                    destination: net.stations[2].id,
                    since: 8.0,
                },
            ],
        );
        let breakdown = model.destination_breakdown(station);
        assert_eq!(breakdown[0], (net.stations[1].id, 2));
        let avg = model.average_wait(station, 10.0).unwrap();
        assert!(avg > 0.0);
    }
}
