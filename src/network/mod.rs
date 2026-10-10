mod generate;
mod line;
mod routing;
mod station;

pub use generate::{expand, generate, GenerateParams};
pub use line::{Line, LineId};
pub use routing::{Route, RouteLeg, Routing, TRANSFER_PENALTY};
pub use station::{Station, StationId, StationKind};

use crate::config::palette::LINE_COLORS;
use crate::geom::{point_segment_distance, Rect, Vec2};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Network {
    pub name: String,
    pub stations: Vec<Station>,
    pub lines: Vec<Line>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetworkError {
    EmptyNetwork,
    DuplicateStation(StationId),
    DuplicateLine(LineId),
    UnknownStation(StationId),
    UnknownLine(LineId),
    LineTooShort(LineId),
    AdjacentDuplicate(LineId),
    NonFinitePosition(StationId),
}

impl std::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkError::EmptyNetwork => write!(f, "сеть не содержит станций"),
            NetworkError::DuplicateStation(id) => {
                write!(f, "дублирующийся идентификатор станции {id}")
            }
            NetworkError::DuplicateLine(id) => write!(f, "дублирующийся идентификатор линии {id}"),
            NetworkError::UnknownStation(id) => write!(f, "ссылка на несуществующую станцию {id}"),
            NetworkError::UnknownLine(id) => write!(f, "ссылка на несуществующую линию {id}"),
            NetworkError::LineTooShort(id) => {
                write!(f, "линия {id} содержит меньше двух станций")
            }
            NetworkError::AdjacentDuplicate(id) => {
                write!(f, "линия {id} содержит соседние одинаковые станции")
            }
            NetworkError::NonFinitePosition(id) => {
                write!(f, "станция {id} имеет некорректные координаты")
            }
        }
    }
}

impl std::error::Error for NetworkError {}

impl Network {
    pub fn new(name: impl Into<String>) -> Self {
        Network {
            name: name.into(),
            stations: Vec::new(),
            lines: Vec::new(),
        }
    }

    fn next_station_id(&self) -> StationId {
        StationId(
            self.stations
                .iter()
                .map(|s| s.id.0)
                .max()
                .map_or(0, |m| m + 1),
        )
    }

    fn next_line_id(&self) -> LineId {
        LineId(self.lines.iter().map(|l| l.id.0).max().map_or(0, |m| m + 1))
    }

    pub fn add_station(
        &mut self,
        name: impl Into<String>,
        pos: Vec2,
        kind: StationKind,
    ) -> StationId {
        let id = self.next_station_id();
        self.stations.push(Station::new(id, name, pos, kind));
        id
    }

    pub fn add_line(&mut self, name: impl Into<String>, color_index: usize) -> LineId {
        let id = self.next_line_id();
        let color = LINE_COLORS[color_index % LINE_COLORS.len()];
        let rgb = [
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
        ];
        let style = crate::config::palette::line_style(color_index);
        let mut line = Line::new(id, name, rgb, style);
        line.base_speed = 150.0;
        self.lines.push(line);
        id
    }

    pub fn append_station(&mut self, line: LineId, station: StationId) -> Result<(), NetworkError> {
        if self.station(station).is_none() {
            return Err(NetworkError::UnknownStation(station));
        }
        let Some(line_ref) = self.line_mut(line) else {
            return Err(NetworkError::UnknownLine(line));
        };
        if line_ref.stations.last() == Some(&station) {
            return Err(NetworkError::AdjacentDuplicate(line));
        }
        line_ref.stations.push(station);
        self.rebuild_connections();
        self.recompute_station_kinds();
        Ok(())
    }

    pub fn detach_station(&mut self, line: LineId, station: StationId) -> bool {
        let detached = if let Some(line_ref) = self.line_mut(line) {
            let before = line_ref.stations.len();
            line_ref.stations.retain(|s| *s != station);
            line_ref.stations.len() != before
        } else {
            false
        };
        if detached {
            self.rebuild_connections();
            self.recompute_station_kinds();
        }
        detached
    }

    pub fn pop_station(&mut self, line: LineId) -> bool {
        let removed = if let Some(line_ref) = self.line_mut(line) {
            line_ref.stations.pop().is_some()
        } else {
            false
        };
        if removed {
            self.rebuild_connections();
            self.recompute_station_kinds();
        }
        removed
    }

    pub fn remove_line(&mut self, id: LineId) -> bool {
        let before = self.lines.len();
        self.lines.retain(|l| l.id != id);
        let removed = self.lines.len() != before;
        if removed {
            self.rebuild_connections();
            self.recompute_station_kinds();
        }
        removed
    }

    pub fn remove_station(&mut self, id: StationId) -> bool {
        let before = self.stations.len();
        self.stations.retain(|s| s.id != id);
        if self.stations.len() == before {
            return false;
        }
        for line in &mut self.lines {
            line.stations.retain(|s| *s != id);
        }
        self.rebuild_connections();
        self.recompute_station_kinds();
        true
    }

    pub fn merge_stations(&mut self, from: StationId, into: StationId) -> bool {
        if from == into || self.station(from).is_none() || self.station(into).is_none() {
            return false;
        }
        for line in &mut self.lines {
            for station in &mut line.stations {
                if *station == from {
                    *station = into;
                }
            }
            line.stations.dedup();
            if line.is_loop
                && line.stations.len() >= 2
                && line.stations.first() == line.stations.last()
            {
                line.stations.pop();
            }
        }
        self.stations.retain(|s| s.id != from);
        self.rebuild_connections();
        self.recompute_station_kinds();
        true
    }

    pub fn recompute_station_kinds(&mut self) {
        for station in &mut self.stations {
            let count = station.lines.len();
            station.kind = match count {
                0 | 1 => StationKind::Regular,
                2 => StationKind::Interchange,
                _ => StationKind::Central,
            };
        }
    }

    pub fn next_station_name(&self) -> String {
        format!("Станция {}", self.stations.len() + 1)
    }

    pub fn routing(&self) -> Routing {
        Routing::build(self)
    }

    pub fn segments_near_point(&self, pos: Vec2, radius: f32) -> Vec<(LineId, usize, f32)> {
        let mut result = Vec::new();
        for line in &self.lines {
            let mut best: Option<(usize, f32)> = None;
            for i in 0..line.segment_count() {
                let Some((a, b)) = line.segment_endpoints(i) else {
                    continue;
                };
                let (Some(pa), Some(pb)) = (self.station(a), self.station(b)) else {
                    continue;
                };
                let d = point_segment_distance(pos, pa.pos, pb.pos);
                if d <= radius && best.is_none_or(|(_, bd)| d < bd) {
                    best = Some((i, d));
                }
            }
            if let Some((segment, distance)) = best {
                result.push((line.id, segment, distance));
            }
        }
        result
    }

    pub fn connect_station_to_nearby_lines(
        &mut self,
        station: StationId,
        radius: f32,
    ) -> Vec<LineId> {
        let Some(pos) = self.station(station).map(|s| s.pos) else {
            return Vec::new();
        };
        let hits = self.segments_near_point(pos, radius);
        let mut connected = Vec::new();
        for (line_id, segment, _) in hits {
            let Some(line) = self.line_mut(line_id) else {
                continue;
            };
            if line.stations.contains(&station) {
                continue;
            }
            line.stations.insert(segment + 1, station);
            connected.push(line_id);
        }
        if !connected.is_empty() {
            self.rebuild_connections();
            self.recompute_station_kinds();
        }
        connected
    }

    pub fn station(&self, id: StationId) -> Option<&Station> {
        self.stations.iter().find(|s| s.id == id)
    }

    pub fn station_mut(&mut self, id: StationId) -> Option<&mut Station> {
        self.stations.iter_mut().find(|s| s.id == id)
    }

    pub fn line(&self, id: LineId) -> Option<&Line> {
        self.lines.iter().find(|l| l.id == id)
    }

    pub fn line_mut(&mut self, id: LineId) -> Option<&mut Line> {
        self.lines.iter_mut().find(|l| l.id == id)
    }

    pub fn rebuild_connections(&mut self) {
        for station in &mut self.stations {
            station.lines.clear();
        }
        let connections: Vec<(LineId, Vec<StationId>)> = self
            .lines
            .iter()
            .map(|l| (l.id, l.stations.clone()))
            .collect();
        for (line_id, stations) in connections {
            for sid in stations {
                if let Some(station) = self.station_mut(sid) {
                    if !station.lines.contains(&line_id) {
                        station.lines.push(line_id);
                    }
                }
            }
        }
    }

    pub fn validate(&self) -> Result<(), NetworkError> {
        if self.stations.is_empty() {
            return Err(NetworkError::EmptyNetwork);
        }
        for (i, s) in self.stations.iter().enumerate() {
            if self.stations[..i].iter().any(|o| o.id == s.id) {
                return Err(NetworkError::DuplicateStation(s.id));
            }
            if !s.pos.is_finite() {
                return Err(NetworkError::NonFinitePosition(s.id));
            }
        }
        for (i, l) in self.lines.iter().enumerate() {
            if self.lines[..i].iter().any(|o| o.id == l.id) {
                return Err(NetworkError::DuplicateLine(l.id));
            }
            if l.stations.len() < 2 {
                return Err(NetworkError::LineTooShort(l.id));
            }
            for (j, sid) in l.stations.iter().enumerate() {
                if self.station(*sid).is_none() {
                    return Err(NetworkError::UnknownStation(*sid));
                }
                let next = l.stations.get(j + 1);
                if let Some(next) = next {
                    if next == sid {
                        return Err(NetworkError::AdjacentDuplicate(l.id));
                    }
                }
            }
            if l.is_loop && l.stations.first() == l.stations.last() {
                return Err(NetworkError::AdjacentDuplicate(l.id));
            }
        }
        Ok(())
    }

    pub fn bounds(&self) -> Option<Rect> {
        if self.stations.is_empty() {
            return None;
        }
        let mut min = Vec2::splat(f32::MAX);
        let mut max = Vec2::splat(f32::MIN);
        for s in &self.stations {
            min.x = min.x.min(s.pos.x);
            min.y = min.y.min(s.pos.y);
            max.x = max.x.max(s.pos.x);
            max.y = max.y.max(s.pos.y);
        }
        let size = max - min;
        if !min.is_finite() || !size.is_finite() {
            return None;
        }
        Some(Rect { pos: min, size })
    }
}

pub fn empty_network() -> Network {
    empty_network_lang(crate::config::i18n::Lang::Ru)
}

pub fn empty_network_lang(lang: crate::config::i18n::Lang) -> Network {
    Network::new(match lang {
        crate::config::i18n::Lang::Ru => "Моя карта",
        crate::config::i18n::Lang::En => "My map",
    })
}

pub fn demo_network() -> Network {
    demo_network_lang(crate::config::i18n::Lang::Ru)
}

pub fn demo_network_lang(lang: crate::config::i18n::Lang) -> Network {
    use crate::config::i18n::Lang;
    let en = lang == Lang::En;
    let pick = |ru: &str, tr: &str| -> String {
        if en {
            tr.to_string()
        } else {
            ru.to_string()
        }
    };
    let mut net = Network::new(pick("Демонстрационная сеть", "Demo network"));

    let north = net.add_station(
        pick("Северные ворота", "North Gate"),
        Vec2::new(610.0, 150.0),
        StationKind::Regular,
    );
    let observatory = net.add_station(
        pick("Обсерватория", "Observatory"),
        Vec2::new(900.0, 180.0),
        StationKind::Regular,
    );
    let museum = net.add_station(
        pick("Музей", "Museum"),
        Vec2::new(330.0, 250.0),
        StationKind::Interchange,
    );
    let central = net.add_station(
        pick("Центральный узел", "Central Hub"),
        Vec2::new(600.0, 400.0),
        StationKind::Central,
    );
    let riverside = net.add_station(
        pick("Набережная", "Riverside"),
        Vec2::new(900.0, 400.0),
        StationKind::Regular,
    );
    let exchange = net.add_station(
        pick("Биржа", "Exchange"),
        Vec2::new(600.0, 560.0),
        StationKind::Regular,
    );
    let foundry = net.add_station(
        pick("Литейный двор", "Foundry"),
        Vec2::new(300.0, 540.0),
        StationKind::Interchange,
    );
    let park = net.add_station(
        pick("Парк", "Park"),
        Vec2::new(600.0, 700.0),
        StationKind::Interchange,
    );
    let harbor = net.add_station(
        pick("Гавань", "Harbor"),
        Vec2::new(900.0, 640.0),
        StationKind::Regular,
    );
    let meadow = net.add_station(
        pick("Луговое", "Meadow"),
        Vec2::new(300.0, 720.0),
        StationKind::Regular,
    );

    let red = net.add_line(pick("Красная", "Red"), 0);
    let blue = net.add_line(pick("Синяя", "Blue"), 1);
    let green = net.add_line(pick("Зелёная", "Green"), 2);
    let amber = net.add_line(pick("Янтарная", "Amber"), 3);

    for (line, stations) in [
        (red, [north, central, park].as_slice()),
        (blue, [museum, central, riverside, harbor].as_slice()),
        (green, [observatory, central, foundry, meadow].as_slice()),
        (amber, [museum, foundry, exchange, park].as_slice()),
    ] {
        for station in stations {
            net.append_station(line, *station)
                .expect("демо-сеть конструируется из корректных ссылок");
        }
    }
    if let Some(l) = net.line_mut(red) {
        l.base_speed = 170.0;
    }
    if let Some(l) = net.line_mut(blue) {
        l.base_speed = 150.0;
    }
    if let Some(l) = net.line_mut(green) {
        l.base_speed = 145.0;
    }
    if let Some(l) = net.line_mut(amber) {
        l.base_speed = 135.0;
    }

    net.rebuild_connections();
    net
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_network_is_valid() {
        let net = demo_network();
        assert!(
            net.validate().is_ok(),
            "демо-сеть должна проходить проверку"
        );
        assert_eq!(net.stations.len(), 10);
        assert_eq!(net.lines.len(), 4);
    }

    #[test]
    fn connections_are_rebuilt() {
        let net = demo_network();
        let central = net
            .stations
            .iter()
            .find(|s| s.kind == StationKind::Central)
            .unwrap();
        assert_eq!(
            central.lines.len(),
            3,
            "центральная станция обслуживает 3 линии"
        );
    }

    #[test]
    fn rejects_adjacent_duplicates() {
        let mut net = Network::new("t");
        let a = net.add_station("A", Vec2::ZERO, StationKind::Regular);
        let b = net.add_station("B", Vec2::new(10.0, 0.0), StationKind::Regular);
        let l = net.add_line("L", 0);
        net.append_station(l, a).unwrap();
        assert!(matches!(
            net.append_station(l, a),
            Err(NetworkError::AdjacentDuplicate(_))
        ));
        net.append_station(l, b).unwrap();
        net.line_mut(l).unwrap().stations.push(b);
        assert!(matches!(
            net.validate(),
            Err(NetworkError::AdjacentDuplicate(_))
        ));
    }

    #[test]
    fn removals_rebuild_connections_and_kinds() {
        let mut net = Network::new("t");
        let a = net.add_station("A", Vec2::ZERO, StationKind::Regular);
        let b = net.add_station("B", Vec2::new(100.0, 0.0), StationKind::Regular);
        let c = net.add_station("C", Vec2::new(200.0, 0.0), StationKind::Regular);
        let l1 = net.add_line("L1", 0);
        let l2 = net.add_line("L2", 1);
        for l in [l1, l2] {
            net.append_station(l, a).unwrap();
            net.append_station(l, b).unwrap();
        }
        net.append_station(l1, c).unwrap();
        assert_eq!(net.station(b).unwrap().lines.len(), 2);
        assert_eq!(net.station(a).unwrap().kind, StationKind::Interchange);

        assert!(net.remove_line(l2));
        assert_eq!(net.station(b).unwrap().lines.len(), 1);
        assert_eq!(net.station(a).unwrap().kind, StationKind::Regular);

        assert!(net.remove_station(b));
        assert!(net.station(b).is_none());
        assert!(!net.station(a).is_none());
        assert!(net.line(l1).unwrap().stations.iter().all(|s| *s != b));
    }

    #[test]
    fn rejects_unknown_station_reference() {
        let mut net = Network::new("t");
        let a = net.add_station("A", Vec2::ZERO, StationKind::Regular);
        let b = net.add_station("B", Vec2::new(10.0, 0.0), StationKind::Regular);
        let l = net.add_line("L", 0);
        net.append_station(l, a).unwrap();
        net.append_station(l, b).unwrap();
        net.line_mut(l).unwrap().stations.push(StationId(999));
        assert!(matches!(
            net.validate(),
            Err(NetworkError::UnknownStation(_))
        ));
    }

    #[test]
    fn placing_station_on_line_inserts_and_creates_transfer() {
        let mut net = Network::new("t");
        let a = net.add_station("A", Vec2::new(0.0, 0.0), StationKind::Regular);
        let b = net.add_station("B", Vec2::new(400.0, 0.0), StationKind::Regular);
        let l1 = net.add_line("L1", 0);
        net.append_station(l1, a).unwrap();
        net.append_station(l1, b).unwrap();

        let mid = net.add_station("M", Vec2::new(200.0, 0.0), StationKind::Regular);
        let connected = net.connect_station_to_nearby_lines(mid, 46.0);
        assert_eq!(connected, vec![l1]);
        assert_eq!(net.line(l1).unwrap().stations, vec![a, mid, b]);
        assert!(net.validate().is_ok());

        let c = net.add_station("C", Vec2::new(200.0, -300.0), StationKind::Regular);
        let l2 = net.add_line("L2", 1);
        net.append_station(l2, c).unwrap();
        net.append_station(l2, mid).unwrap();
        assert_eq!(net.station(mid).unwrap().lines.len(), 2);
        assert_eq!(net.station(mid).unwrap().kind, StationKind::Interchange);
    }

    #[test]
    fn empty_network_has_no_bounds() {
        let net = empty_network();
        assert!(net.bounds().is_none());
    }

    #[test]
    fn bounds_cover_all_stations() {
        let net = demo_network();
        let b = net.bounds().unwrap();
        for s in &net.stations {
            assert!(s.pos.x >= b.pos.x && s.pos.x <= b.right());
            assert!(s.pos.y >= b.pos.y && s.pos.y <= b.bottom());
        }
    }
}
