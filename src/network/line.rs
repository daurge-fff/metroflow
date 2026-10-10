use crate::config::palette::LineStyle;
use crate::network::StationId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct LineId(pub u32);

impl std::fmt::Display for LineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "L{}", self.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Line {
    pub id: LineId,
    pub name: String,
    pub color: [u8; 3],
    pub style: LineStyle,
    pub stations: Vec<StationId>,
    pub is_loop: bool,
    pub base_speed: f32,
    /// Manual trains-per-direction when auto-trains is disabled (0/None = use
    /// the global minimum).
    #[serde(default)]
    pub trains_override: Option<u32>,
    /// Manual cars-per-train when auto-consist is disabled.
    #[serde(default)]
    pub cars_override: Option<u32>,
}

impl Line {
    pub fn new(id: LineId, name: impl Into<String>, color: [u8; 3], style: LineStyle) -> Self {
        Line {
            id,
            name: name.into(),
            color,
            style,
            stations: Vec::new(),
            is_loop: false,
            base_speed: 150.0,
            trains_override: None,
            cars_override: None,
        }
    }

    pub fn segment_count(&self) -> usize {
        match self.stations.len() {
            0 | 1 => 0,
            n if self.is_loop => n,
            n => n - 1,
        }
    }

    pub fn advance(&self, segment: usize, direction: i8) -> (usize, i8) {
        let segs = self.segment_count();
        if segs == 0 {
            return (0, 1);
        }
        if self.is_loop {
            return ((segment + 1) % segs, 1);
        }
        if direction >= 0 {
            if segment + 1 >= segs {
                (segment, -1)
            } else {
                (segment + 1, 1)
            }
        } else if segment == 0 {
            (0, 1)
        } else {
            (segment - 1, -1)
        }
    }

    pub fn station_after(&self, segment: usize, direction: i8) -> Option<StationId> {
        let (seg, dir) = self.advance(segment, direction);
        let (a, b) = self.segment_endpoints(seg)?;
        Some(if dir >= 0 { b } else { a })
    }

    pub fn segment_endpoints(&self, index: usize) -> Option<(StationId, StationId)> {
        let count = self.segment_count();
        if count == 0 || index >= count {
            return None;
        }
        let a = self.stations[index];
        let b = if index + 1 < self.stations.len() {
            self.stations[index + 1]
        } else {
            self.stations[0]
        };
        Some((a, b))
    }
}
