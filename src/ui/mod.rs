pub mod analytics;
pub mod editor;
pub mod hud;
pub mod icons;
pub mod layout;
pub mod menu;
pub mod scroll;
pub mod widgets;

use crate::geom::Vec2;
use crate::network::LineId;
use crate::network::StationId;
use icons::Icon;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EditorTool {
    Select,
    Station,
    Link,
    Erase,
}

impl EditorTool {
    pub fn label(self) -> &'static str {
        match self {
            EditorTool::Select => "Выбор",
            EditorTool::Station => "Станция",
            EditorTool::Link => "Ветка",
            EditorTool::Erase => "Удалить",
        }
    }

    pub fn label_en(self) -> &'static str {
        match self {
            EditorTool::Select => "Select",
            EditorTool::Station => "Station",
            EditorTool::Link => "Branch",
            EditorTool::Erase => "Erase",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            EditorTool::Select => Icon::Center,
            EditorTool::Station => Icon::AddCircle,
            EditorTool::Link => Icon::Route,
            EditorTool::Erase => Icon::Delete,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum UiAction {
    NewGame,
    NewEmpty,
    OpenSetup,
    CloseSetup,
    OpenAbout,
    CloseAbout,
    OpenLink(String),
    GenerateMap,
    GenerateRandom,
    AdjustStations(i32),
    AdjustLines(i32),
    AdjustHubs(i32),
    AdjustTrainsMin(i32),
    AdjustTrainsMax(i32),
    ToggleAutoTrains,
    LoadGame,
    SaveGame,
    OpenMenu,
    OpenSettings,
    CloseSettings,
    Quit,
    TogglePause,
    SetScale(usize),
    ZoomIn,
    ZoomOut,
    FitView,
    CenterNetwork,
    ToggleLabels,
    ToggleWaiting,
    ToggleGrid,
    ToggleMotion,
    ToggleHeat,
    ToggleFlow,
    ToggleHelp,
    ToggleControls,
    ToggleAutoCars,
    ToggleAutosave,
    AdjustFlowGrowth(i32),
    AdjustUiScale(i32),
    SetUiScale(f32),
    SetCarCapacity(u32),
    SetCarsMin(u32),
    SetCarsMax(u32),
    SetTrainsMin(u32),
    SetTrainsMax(u32),
    SetFlowGrowth(f32),
    ExpandNetwork,
    SetFlow(usize),
    SetLanguage(usize),
    SetTheme(usize),
    AdjustCarCapacity(i32),
    AdjustCarsMin(i32),
    AdjustCarsMax(i32),
    OpenAnalytics,
    CloseAnalytics,
    FocusStation(StationId),
    PanTo(Vec2),
    ToggleEdit,
    SetTool(EditorTool),
    NewLine,
    SelectLine(LineId),
    DeleteLine(LineId),
    RemoveLineLastStation(LineId),
    CycleLineColor,
    SetLineColor(LineId, usize),
    AdjustLineTrains(LineId, i32),
    AdjustLineCars(LineId, i32),
    CycleLineStyle,
    DeleteStation(StationId),
    DetachFromLine(LineId, StationId),
    StartRename(StationId),
    PreviewRoute(StationId, StationId),
    EditorUndo,
    EditorRedo,
}
