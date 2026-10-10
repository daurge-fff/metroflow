use crate::geom::Vec2;
use crate::network::{LineId, Network, StationId};
use std::collections::{BinaryHeap, HashMap};

pub const TRANSFER_PENALTY: f32 = 6.0;

const EPS: f32 = 1e-3;

struct Candidate {
    cost: f32,
    node: usize,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}
impl Eq for Candidate {}
impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.cost.total_cmp(&self.cost)
    }
}
impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug)]
pub struct RouteLeg {
    pub line: LineId,
    pub stations: Vec<StationId>,
}

#[derive(Clone, Debug)]
pub struct Route {
    pub from: StationId,
    pub to: StationId,
    pub legs: Vec<RouteLeg>,
    pub total_time: f32,
    pub transfers: usize,
}

impl Route {
    pub fn is_empty(&self) -> bool {
        self.legs.is_empty()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Routing {
    node_of: HashMap<(LineId, StationId), usize>,
    node_line: Vec<LineId>,
    station_nodes: HashMap<StationId, Vec<usize>>,
    station_index: HashMap<StationId, usize>,
    node_count: usize,
    cost: Vec<f32>,
    positions: HashMap<StationId, Vec2>,
    speeds: HashMap<LineId, f32>,
}

impl Routing {
    pub fn build(network: &Network) -> Self {
        let mut node_of = HashMap::new();
        let mut node_line = Vec::new();
        let mut station_nodes: HashMap<StationId, Vec<usize>> = HashMap::new();
        for line in &network.lines {
            for sid in &line.stations {
                let key = (line.id, *sid);
                if node_of.contains_key(&key) {
                    continue;
                }
                let idx = node_line.len();
                node_of.insert(key, idx);
                node_line.push(line.id);
                station_nodes.entry(*sid).or_default().push(idx);
            }
        }
        let node_count = node_line.len();

        let mut station_index = HashMap::new();
        for (i, s) in network.stations.iter().enumerate() {
            station_index.insert(s.id, i);
        }
        let station_count = network.stations.len();

        let positions: HashMap<StationId, Vec2> =
            network.stations.iter().map(|s| (s.id, s.pos)).collect();
        let speeds: HashMap<LineId, f32> = network
            .lines
            .iter()
            .map(|l| (l.id, l.base_speed.max(1.0)))
            .collect();

        let mut edges: Vec<Vec<(usize, f32)>> = vec![Vec::new(); node_count];
        let time_of = |a: StationId, b: StationId, line: LineId| -> f32 {
            let pa = positions.get(&a).copied().unwrap_or(Vec2::ZERO);
            let pb = positions.get(&b).copied().unwrap_or(Vec2::ZERO);
            let speed = speeds.get(&line).copied().unwrap_or(150.0);
            (pa.distance(pb) / speed).max(EPS)
        };
        for line in &network.lines {
            for i in 0..line.segment_count() {
                let Some((a, b)) = line.segment_endpoints(i) else {
                    continue;
                };
                let (Some(&ua), Some(&ub)) =
                    (node_of.get(&(line.id, a)), node_of.get(&(line.id, b)))
                else {
                    continue;
                };
                let t = time_of(a, b, line.id);
                edges[ua].push((ub, t));
                edges[ub].push((ua, t));
            }
        }
        for nodes in station_nodes.values() {
            for &i in nodes {
                for &j in nodes {
                    if i != j {
                        edges[i].push((j, TRANSFER_PENALTY));
                    }
                }
            }
        }

        let mut reversed: Vec<Vec<(usize, f32)>> = vec![Vec::new(); node_count];
        for (u, list) in edges.iter().enumerate() {
            for &(v, c) in list {
                reversed[v].push((u, c));
            }
        }

        let mut cost = vec![f32::INFINITY; station_count * node_count];
        for station in &network.stations {
            let Some(&di) = station_index.get(&station.id) else {
                continue;
            };
            let mut dist = vec![f32::INFINITY; node_count];
            let mut heap = BinaryHeap::new();
            if let Some(nodes) = station_nodes.get(&station.id) {
                for &n in nodes {
                    dist[n] = 0.0;
                    heap.push(Candidate { cost: 0.0, node: n });
                }
            }
            while let Some(Candidate { cost: c, node: u }) = heap.pop() {
                if c > dist[u] + EPS {
                    continue;
                }
                for &(w, edge) in &reversed[u] {
                    let nc = c + edge;
                    if nc < dist[w] - EPS {
                        dist[w] = nc;
                        heap.push(Candidate { cost: nc, node: w });
                    }
                }
            }
            for n in 0..node_count {
                cost[di * node_count + n] = dist[n];
            }
        }

        Routing {
            node_of,
            node_line,
            station_nodes,
            station_index,
            node_count,
            cost,
            positions,
            speeds,
        }
    }

    pub fn travel_time(&self, line: LineId, a: StationId, b: StationId) -> f32 {
        let pa = self.positions.get(&a).copied().unwrap_or(Vec2::ZERO);
        let pb = self.positions.get(&b).copied().unwrap_or(Vec2::ZERO);
        let speed = self.speeds.get(&line).copied().unwrap_or(150.0);
        (pa.distance(pb) / speed).max(EPS)
    }

    pub fn is_connected(&self, from: StationId, to: StationId) -> bool {
        self.cost_station(from, to).is_some()
    }

    fn node_index(&self, line: LineId, station: StationId) -> Option<usize> {
        self.node_of.get(&(line, station)).copied()
    }

    pub fn cost_node(&self, line: LineId, station: StationId, dest: StationId) -> Option<f32> {
        let di = *self.station_index.get(&dest)?;
        let node = self.node_index(line, station)?;
        let value = self.cost[di * self.node_count + node];
        if value.is_finite() {
            Some(value)
        } else {
            None
        }
    }

    pub fn cost_station(&self, station: StationId, dest: StationId) -> Option<f32> {
        let nodes = self.station_nodes.get(&station)?;
        let mut best = f32::INFINITY;
        for &n in nodes {
            let di = *self.station_index.get(&dest)?;
            let value = self.cost[di * self.node_count + n];
            if value < best {
                best = value;
            }
        }
        if best.is_finite() {
            Some(best)
        } else {
            None
        }
    }

    pub fn best_other_line_cost(
        &self,
        station: StationId,
        dest: StationId,
        exclude: LineId,
    ) -> f32 {
        let mut best = f32::INFINITY;
        if let (Some(nodes), Some(&di)) = (
            self.station_nodes.get(&station),
            self.station_index.get(&dest),
        ) {
            for &n in nodes {
                if self.node_line[n] == exclude {
                    continue;
                }
                let value = self.cost[di * self.node_count + n];
                if value < best {
                    best = value;
                }
            }
        }
        best
    }

    pub fn ride_is_good(
        &self,
        dest: StationId,
        station: StationId,
        line: LineId,
        next: StationId,
        current: Option<LineId>,
    ) -> bool {
        let Some(next_cost) = self.cost_node(line, next, dest) else {
            return false;
        };
        let ride = self.travel_time(line, station, next) + next_cost;
        match current {
            None => {
                let best = self.cost_station(station, dest).unwrap_or(f32::INFINITY);
                ride <= best + EPS
            }
            Some(cur) if cur == line => {
                let transfer = TRANSFER_PENALTY + self.best_other_line_cost(station, dest, line);
                ride <= transfer + EPS
            }
            Some(_) => false,
        }
    }

    fn line_neighbors(
        &self,
        network: &Network,
        line: LineId,
        station: StationId,
    ) -> Vec<StationId> {
        let Some(line) = network.line(line) else {
            return Vec::new();
        };
        let mut result = Vec::new();
        let n = line.stations.len();
        for i in 0..n {
            if line.stations[i] != station {
                continue;
            }
            let prev = if i == 0 {
                if line.is_loop {
                    Some(line.stations[n - 1])
                } else {
                    None
                }
            } else {
                Some(line.stations[i - 1])
            };
            let next = if i + 1 < n {
                Some(line.stations[i + 1])
            } else if line.is_loop {
                Some(line.stations[0])
            } else {
                None
            };
            for candidate in [prev, next].into_iter().flatten() {
                if candidate != station && !result.contains(&candidate) {
                    result.push(candidate);
                }
            }
        }
        result
    }

    pub fn plan(&self, network: &Network, from: StationId, to: StationId) -> Option<Route> {
        if from == to {
            return Some(Route {
                from,
                to,
                legs: Vec::new(),
                total_time: 0.0,
                transfers: 0,
            });
        }
        if !self.station_index.contains_key(&from) || !self.station_index.contains_key(&to) {
            return None;
        }

        let mut legs: Vec<RouteLeg> = Vec::new();
        let mut station = from;
        let mut current_line: Option<LineId> = None;
        let mut total_time = 0.0f32;
        let mut transfers = 0usize;
        let mut guard = 0usize;
        let limit = self.node_count * 4 + 16;

        while station != to {
            guard += 1;
            if guard > limit {
                return None;
            }
            match current_line {
                None => {
                    let nodes = self.station_nodes.get(&station)?;
                    let mut best: Option<(LineId, f32)> = None;
                    for &n in nodes {
                        let line = self.node_line[n];
                        if let Some(c) = self.cost_node(line, station, to) {
                            if best.is_none_or(|(_, bc)| c < bc) {
                                best = Some((line, c));
                            }
                        }
                    }
                    let (line, _) = best?;
                    current_line = Some(line);
                    legs.push(RouteLeg {
                        line,
                        stations: vec![station],
                    });
                }
                Some(line) => {
                    let transfer_cost =
                        TRANSFER_PENALTY + self.best_other_line_cost(station, to, line);
                    let mut best_ride = f32::INFINITY;
                    let mut best_next = None;
                    for neighbor in self.line_neighbors(network, line, station) {
                        if let Some(c) = self.cost_node(line, neighbor, to) {
                            let cost = self.travel_time(line, station, neighbor) + c;
                            if cost < best_ride {
                                best_ride = cost;
                                best_next = Some(neighbor);
                            }
                        }
                    }
                    match best_next {
                        None => {
                            current_line = None;
                            transfers += 1;
                        }
                        Some(_) if transfer_cost < best_ride - EPS => {
                            current_line = None;
                            transfers += 1;
                        }
                        Some(next) => {
                            total_time += self.travel_time(line, station, next);
                            if let Some(leg) = legs.last_mut() {
                                leg.stations.push(next);
                            }
                            station = next;
                        }
                    }
                }
            }
        }

        Some(Route {
            from,
            to,
            legs,
            total_time,
            transfers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::demo_network;

    #[test]
    fn demo_network_is_fully_connected() {
        let net = demo_network();
        let routing = Routing::build(&net);
        let first = net.stations[0].id;
        for s in &net.stations {
            assert!(routing.cost_station(first, s.id).is_some());
        }
    }

    #[test]
    fn plan_without_transfers_stays_on_one_line() {
        let net = demo_network();
        let routing = Routing::build(&net);
        let line = &net.lines[0];
        let route = routing
            .plan(
                &net,
                line.stations[0],
                line.stations[line.stations.len() - 1],
            )
            .unwrap();
        assert_eq!(route.transfers, 0);
        assert_eq!(route.legs.len(), 1);
    }

    #[test]
    fn plan_prefers_fewer_transfers() {
        let net = demo_network();
        let routing = Routing::build(&net);
        let a = net.stations[0].id;
        let b = net.stations[net.stations.len() - 1].id;
        if let Some(route) = routing.plan(&net, a, b) {
            for pair in route.legs.windows(2) {
                assert_eq!(pair[0].stations.last(), pair[1].stations.first());
            }
            assert_eq!(route.transfers, route.legs.len().saturating_sub(1));
        }
    }

    #[test]
    fn disconnected_has_no_cost() {
        let mut net = Network::new("t");
        let a = net.add_station("A", Vec2::ZERO, crate::network::StationKind::Regular);
        let b = net.add_station(
            "B",
            Vec2::new(100.0, 0.0),
            crate::network::StationKind::Regular,
        );
        let routing = Routing::build(&net);
        assert!(routing.cost_station(a, b).is_none());
        assert!(routing.plan(&net, a, b).is_none());
    }
}
