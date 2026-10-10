use crate::geom::Vec2;
use crate::network::LineId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub struct StationId(pub u32);

impl std::fmt::Display for StationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "S{}", self.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum StationKind {
    Regular,
    Interchange,
    Central,
}

impl StationKind {
    pub fn label(self) -> &'static str {
        match self {
            StationKind::Regular => "Обычная станция",
            StationKind::Interchange => "Пересадочная станция",
            StationKind::Central => "Центральная станция",
        }
    }

    pub fn label_en(self) -> &'static str {
        match self {
            StationKind::Regular => "Regular station",
            StationKind::Interchange => "Interchange",
            StationKind::Central => "Central station",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Station {
    pub id: StationId,
    pub name: String,
    pub pos: Vec2,
    pub kind: StationKind,
    #[serde(default)]
    pub lines: Vec<LineId>,
}

impl Station {
    pub fn new(id: StationId, name: impl Into<String>, pos: Vec2, kind: StationKind) -> Self {
        Station {
            id,
            name: name.into(),
            pos,
            kind,
            lines: Vec::new(),
        }
    }
}
