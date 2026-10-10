use crate::config::i18n::Lang;
use serde::{Deserialize, Serialize};

pub const TIME_SCALES: [f32; 5] = [1.0, 2.0, 4.0, 8.0, 16.0];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum FlowLevel {
    Calm,
    #[default]
    Normal,
    Busy,
    Rush,
}

impl FlowLevel {
    pub const ALL: [FlowLevel; 4] = [
        FlowLevel::Calm,
        FlowLevel::Normal,
        FlowLevel::Busy,
        FlowLevel::Rush,
    ];

    pub fn label(self) -> &'static str {
        match self {
            FlowLevel::Calm => "Спокойный",
            FlowLevel::Normal => "Обычный",
            FlowLevel::Busy => "Людно",
            FlowLevel::Rush => "Час пик",
        }
    }

    pub fn label_en(self) -> &'static str {
        match self {
            FlowLevel::Calm => "Calm",
            FlowLevel::Normal => "Normal",
            FlowLevel::Busy => "Busy",
            FlowLevel::Rush => "Rush hour",
        }
    }

    pub fn multiplier(self) -> f32 {
        match self {
            FlowLevel::Calm => 0.6,
            FlowLevel::Normal => 1.0,
            FlowLevel::Busy => 1.5,
            FlowLevel::Rush => 2.2,
        }
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|l| *l == self).unwrap_or(1)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub show_labels: bool,
    pub show_waiting: bool,
    pub show_grid: bool,
    pub show_heat: bool,
    pub show_flow: bool,
    pub reduced_motion: bool,
    pub time_scale_index: usize,
    pub language: Lang,

    pub cars_min: u32,
    pub cars_max: u32,
    pub car_capacity: u32,
    pub auto_cars: bool,
    pub trains_min: u32,
    pub trains_max: u32,
    pub auto_trains: bool,
    pub flow_level: FlowLevel,
    pub flow_growth: f32,
    pub autosave: bool,
    pub ui_scale: f32,
    pub controls_hidden: bool,
    pub theme: crate::config::palette::Theme,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            show_labels: true,
            show_waiting: true,
            show_grid: true,
            show_heat: false,
            show_flow: false,
            reduced_motion: false,
            time_scale_index: 1,
            language: Lang::En,
            cars_min: 2,
            cars_max: 4,
            car_capacity: 30,
            auto_cars: true,
            trains_min: 1,
            trains_max: 3,
            auto_trains: true,
            flow_level: FlowLevel::Normal,
            flow_growth: 0.12,
            autosave: true,
            ui_scale: 0.0,
            controls_hidden: false,
            theme: crate::config::palette::Theme::Dark,
        }
    }
}

pub fn auto_ui_scale() -> f32 {
    if cfg!(target_os = "android") || cfg!(target_os = "ios") {
        1.4
    } else {
        1.0
    }
}

impl Settings {
    pub fn sanitize(&mut self) {
        self.cars_min = self.cars_min.clamp(1, 6);
        self.cars_max = self.cars_max.clamp(self.cars_min, 8);
        self.car_capacity = self.car_capacity.clamp(10, 120);
        self.trains_min = self.trains_min.clamp(1, 6);
        self.trains_max = self.trains_max.clamp(self.trains_min, 8);
        self.flow_growth = self.flow_growth.clamp(0.0, 1.0);
        self.time_scale_index = self.time_scale_index.min(TIME_SCALES.len() - 1);
        if self.ui_scale != 0.0 {
            self.ui_scale = self.ui_scale.clamp(0.8, 3.0);
        }
    }
}
