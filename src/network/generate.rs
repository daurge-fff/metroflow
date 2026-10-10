use crate::config::i18n::Lang;
use crate::geom::Vec2;
use crate::network::{Network, StationId, StationKind};

#[derive(Clone, Copy, Debug)]
pub struct GenerateParams {
    pub seed: u64,
    pub stations: u32,
    pub lines: u32,
    pub hubs: u32,
}

const MIN_STATIONS: u32 = 4;
const MAX_STATIONS: u32 = 40;
const MIN_LINES: u32 = 1;
const MAX_LINES: u32 = 8;

impl GenerateParams {
    pub fn sanitized(self) -> Self {
        let stations = self.stations.clamp(MIN_STATIONS, MAX_STATIONS);
        let max_lines = MAX_LINES.min(stations - 1).max(MIN_LINES);
        let lines = self.lines.clamp(MIN_LINES, max_lines);
        let max_hubs = if lines >= 2 {
            stations.saturating_sub(2).min(lines)
        } else {
            0
        };
        let hubs = if max_hubs >= 1 {
            self.hubs.clamp(1, max_hubs)
        } else {
            0
        };
        GenerateParams {
            seed: self.seed,
            stations,
            lines,
            hubs,
        }
    }

    pub fn random(seed: u64) -> Self {
        let mut rng = Rng::new(seed ^ 0x9E37_79B9_7F4A_7C15);
        let stations = rng.range_u32(8, 18);
        let lines = rng.range_u32(2, 4.min(stations.saturating_sub(1)));
        let hubs = rng.range_u32(
            if lines >= 2 { 1 } else { 0 },
            lines.min(stations.saturating_sub(2)),
        );
        GenerateParams {
            seed,
            stations,
            lines,
            hubs,
        }
    }
}

pub fn generate(params: GenerateParams, lang: Lang) -> Network {
    let (net_name, station_fmt, line_fmt) = match lang {
        Lang::Ru => ("Случайная сеть", "Станция {}", "Линия {}"),
        Lang::En => ("Random network", "Station {}", "Line {}"),
    };
    let p = params.sanitized();
    let mut rng = Rng::new(p.seed);
    let n = p.stations as usize;
    let l = p.lines as usize;
    let hub_count = (p.hubs as usize).max(1).min(n.saturating_sub(1));

    let mut positions: Vec<Vec2> = Vec::with_capacity(n);
    positions.push(Vec2::ZERO);
    for k in 1..hub_count {
        let a = std::f32::consts::TAU * k as f32 / hub_count as f32;
        positions.push(snap(Vec2::new(a.cos() * 330.0, a.sin() * 330.0)));
    }
    let outer = n.saturating_sub(hub_count).max(1);
    for k in 0..outer {
        let ring = 560.0 + (k % 2) as f32 * 110.0 + rng.range(-50.0, 50.0);
        let a = std::f32::consts::TAU * k as f32 / outer as f32 + rng.range(-0.18, 0.18);
        positions.push(snap(Vec2::new(a.cos() * ring, a.sin() * ring)));
    }

    let mut net = Network::new(net_name);
    let ids: Vec<StationId> = positions
        .iter()
        .enumerate()
        .map(|(i, p)| net.add_station(named(station_fmt, i + 1), *p, StationKind::Regular))
        .collect();

    let outer_indices: Vec<usize> = (hub_count..n).collect();
    let mut groups: Vec<Vec<usize>> = vec![Vec::new(); l];
    for (k, idx) in outer_indices.iter().enumerate() {
        groups[k % l].push(*idx);
    }

    for (i, group) in groups.iter().enumerate() {
        let line = net.add_line(named(line_fmt, i + 1), i);
        let start_hub = i % hub_count.max(1);
        let end_hub = (i + 1) % hub_count.max(1);
        let mut seq: Vec<StationId> = Vec::new();
        seq.push(ids[start_hub]);
        let mut group = group.clone();
        group.sort_by(|a, b| {
            let aa = angle_of(positions[*a]);
            let bb = angle_of(positions[*b]);
            aa.partial_cmp(&bb).unwrap_or(std::cmp::Ordering::Equal)
        });
        for idx in group {
            seq.push(ids[idx]);
        }
        if hub_count >= 2 {
            seq.push(ids[end_hub]);
        }
        for s in seq {
            let _ = net.append_station(line, s);
        }
        if let Some(lref) = net.line_mut(line) {
            lref.base_speed = 150.0 - i as f32 * 5.0;
        }
    }

    net.recompute_station_kinds();
    net
}

pub fn expand(network: &mut Network, seed: u64, count: u32, lang: Lang) -> bool {
    if network.stations.is_empty() {
        return false;
    }
    let (station_fmt, line_fmt) = match lang {
        Lang::Ru => ("Станция {}", "Линия {}"),
        Lang::En => ("Station {}", "Line {}"),
    };
    let count = count.clamp(1, 6) as usize;
    let mut rng = Rng::new(seed);
    let anchor_index = (rng.next_u64() as usize) % network.stations.len();
    let anchor = network.stations[anchor_index].clone();
    let anchor_pos = anchor.pos;
    let anchor_id = anchor.id;

    let center = network.bounds().map(|b| b.center()).unwrap_or(Vec2::ZERO);
    let mut dir = (anchor_pos - center).normalized();
    if dir.length() < 0.1 {
        dir = Vec2::new(1.0, 0.0);
    }
    let angle = rng.range(-0.7, 0.7);
    let (s, c) = angle.sin_cos();
    dir = Vec2::new(dir.x * c - dir.y * s, dir.x * s + dir.y * c);

    let mut new_ids = Vec::new();
    let mut pos = anchor_pos;
    for _ in 0..count {
        pos = pos + dir * (175.0 + rng.range(-25.0, 25.0));
        let mut guard = 0;
        while network
            .stations
            .iter()
            .any(|existing| existing.pos.distance(pos) < 130.0)
            && guard < 8
        {
            pos = pos + dir * 130.0;
            guard += 1;
        }
        let name = named(station_fmt, network.stations.len() + 1);
        let id = network.add_station(name, snap(pos), StationKind::Regular);
        new_ids.push(id);
    }

    let color_index = network.lines.len();
    let line = network.add_line(named(line_fmt, network.lines.len() + 1), color_index);
    let _ = network.append_station(line, anchor_id);
    for id in new_ids {
        let _ = network.append_station(line, id);
    }
    if let Some(l) = network.line_mut(line) {
        l.base_speed = 145.0;
    }
    network.recompute_station_kinds();
    true
}

fn named(fmt: &str, n: usize) -> String {
    fmt.replacen("{}", &n.to_string(), 1)
}

fn angle_of(p: Vec2) -> f32 {
    p.y.atan2(p.x)
}

fn snap(v: Vec2) -> Vec2 {
    const STEP: f32 = 20.0;
    Vec2::new((v.x / STEP).round() * STEP, (v.y / STEP).round() * STEP)
}

struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Rng {
            state: if seed == 0 {
                0x2545_F491_4F6C_DD1D
            } else {
                seed
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn range(&mut self, min: f32, max: f32) -> f32 {
        let u = (self.next_u64() >> 11) as f32 / (1u64 << 53) as f32;
        min + (max - min) * u
    }

    fn range_u32(&mut self, min: u32, max: u32) -> u32 {
        if max <= min {
            return min;
        }
        min + (self.next_u64() % (max - min + 1) as u64) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_networks_are_valid_and_connected() {
        for seed in 1..40u64 {
            let params = GenerateParams {
                seed,
                stations: 6 + (seed % 14) as u32,
                lines: 2 + (seed % 3) as u32,
                hubs: (seed % 3) as u32,
            }
            .sanitized();
            let net = generate(params, crate::config::i18n::Lang::Ru);
            assert!(
                net.validate().is_ok(),
                "seed {seed} выдал некорректную сеть"
            );
            assert_eq!(net.lines.len(), params.lines as usize);
            assert_eq!(
                net.stations.len(),
                params.stations as usize,
                "seed {seed}: неверное число станций"
            );
            let routing = net.routing();
            let first = net.stations[0].id;
            for s in &net.stations {
                assert!(
                    routing.is_connected(first, s.id),
                    "seed {seed}: сеть не связна"
                );
            }
            for line in &net.lines {
                assert!(
                    line.stations.len() >= 2,
                    "seed {seed}: линия короче двух станций"
                );
            }
        }
    }

    #[test]
    fn expand_adds_a_valid_branch() {
        let mut net = crate::network::demo_network();
        let before_lines = net.lines.len();
        let before_stations = net.stations.len();
        assert!(expand(&mut net, 5, 3, crate::config::i18n::Lang::Ru));
        assert_eq!(net.lines.len(), before_lines + 1);
        assert_eq!(net.stations.len(), before_stations + 3);
        assert!(net.validate().is_ok());
    }

    #[test]
    fn sanitize_protects_hubs() {
        let p = GenerateParams {
            seed: 1,
            stations: 10,
            lines: 1,
            hubs: 5,
        }
        .sanitized();
        assert_eq!(p.hubs, 0);
        let p = GenerateParams {
            seed: 1,
            stations: 4,
            lines: 8,
            hubs: 3,
        }
        .sanitized();
        assert!(p.lines <= 3);
    }
}
