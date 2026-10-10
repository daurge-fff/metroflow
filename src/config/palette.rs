use macroquad::prelude::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LineStyle {
    Solid,
    Dashed,
    Dotted,
    DashDot,
}

impl LineStyle {
    pub fn label(self) -> &'static str {
        match self {
            LineStyle::Solid => "Сплошная",
            LineStyle::Dashed => "Штриховая",
            LineStyle::Dotted => "Пунктирная",
            LineStyle::DashDot => "Штрих-пунктир",
        }
    }

    pub fn label_en(self) -> &'static str {
        match self {
            LineStyle::Solid => "Solid",
            LineStyle::Dashed => "Dashed",
            LineStyle::Dotted => "Dotted",
            LineStyle::DashDot => "Dash-dot",
        }
    }

    pub fn dash_pattern(self) -> &'static [(f32, f32)] {
        match self {
            LineStyle::Solid => &[(1.0, 0.0)],
            LineStyle::Dashed => &[(14.0, 10.0)],
            LineStyle::Dotted => &[(2.0, 9.0)],
            LineStyle::DashDot => &[(16.0, 8.0), (2.0, 8.0)],
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Theme {
    #[default]
    Dark,
    Light,
    Amoled,
}

impl Theme {
    pub const ALL: [Theme; 3] = [Theme::Dark, Theme::Light, Theme::Amoled];

    pub fn label(self) -> &'static str {
        match self {
            Theme::Dark => "Тёмная",
            Theme::Light => "Светлая",
            Theme::Amoled => "AMOLED",
        }
    }

    pub fn label_en(self) -> &'static str {
        match self {
            Theme::Dark => "Dark",
            Theme::Light => "Light",
            Theme::Amoled => "AMOLED",
        }
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|t| *t == self).unwrap_or(0)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub bg_top: Color,
    pub bg_bottom: Color,
    pub map_bg: Color,
    pub map_grid: Color,
    pub panel: Color,
    pub panel_alt: Color,
    pub panel_raised: Color,
    pub border: Color,
    pub border_strong: Color,
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_muted: Color,
    pub accent: Color,
    pub accent_strong: Color,
    pub on_accent: Color,
    pub warning: Color,
    pub danger: Color,
    pub success: Color,
    pub track_casing: Color,
    pub station_fill: Color,
    pub station_border: Color,
    pub station_waiting: Color,
    pub overlay: Color,
}

impl Palette {
    pub fn for_theme(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self::metroflow(),
            Theme::Light => Self::light(),
            Theme::Amoled => Self::amoled(),
        }
    }

    pub const fn metroflow() -> Self {
        Palette {
            bg_top: Color::from_rgba(13, 18, 30, 255),
            bg_bottom: Color::from_rgba(18, 26, 43, 255),
            map_bg: Color::from_rgba(15, 21, 34, 255),
            map_grid: Color::from_rgba(38, 50, 74, 90),
            panel: Color::from_rgba(22, 30, 46, 242),
            panel_alt: Color::from_rgba(28, 38, 57, 242),
            panel_raised: Color::from_rgba(35, 47, 70, 255),
            border: Color::from_rgba(52, 68, 98, 255),
            border_strong: Color::from_rgba(78, 100, 138, 255),
            text_primary: Color::from_rgba(232, 238, 248, 255),
            text_secondary: Color::from_rgba(150, 166, 196, 255),
            text_muted: Color::from_rgba(104, 120, 150, 255),
            accent: Color::from_rgba(79, 195, 247, 255),
            accent_strong: Color::from_rgba(140, 214, 255, 255),
            on_accent: Color::from_rgba(10, 16, 28, 255),
            warning: Color::from_rgba(245, 177, 76, 255),
            danger: Color::from_rgba(239, 90, 90, 255),
            success: Color::from_rgba(102, 187, 106, 255),
            track_casing: Color::from_rgba(9, 13, 22, 235),
            station_fill: Color::from_rgba(238, 243, 251, 255),
            station_border: Color::from_rgba(13, 18, 30, 255),
            station_waiting: Color::from_rgba(245, 177, 76, 255),
            overlay: Color::from_rgba(8, 12, 20, 200),
        }
    }

    pub const fn amoled() -> Self {
        Palette {
            bg_top: Color::from_rgba(0, 0, 0, 255),
            bg_bottom: Color::from_rgba(0, 0, 0, 255),
            map_bg: Color::from_rgba(0, 0, 0, 255),
            map_grid: Color::from_rgba(30, 40, 54, 110),
            panel: Color::from_rgba(7, 8, 11, 250),
            panel_alt: Color::from_rgba(12, 14, 18, 250),
            panel_raised: Color::from_rgba(20, 22, 28, 255),
            border: Color::from_rgba(38, 42, 52, 255),
            border_strong: Color::from_rgba(70, 78, 94, 255),
            text_primary: Color::from_rgba(233, 239, 248, 255),
            text_secondary: Color::from_rgba(150, 162, 182, 255),
            text_muted: Color::from_rgba(96, 108, 128, 255),
            accent: Color::from_rgba(79, 195, 247, 255),
            accent_strong: Color::from_rgba(140, 214, 255, 255),
            on_accent: Color::from_rgba(4, 8, 14, 255),
            warning: Color::from_rgba(245, 177, 76, 255),
            danger: Color::from_rgba(239, 90, 90, 255),
            success: Color::from_rgba(102, 187, 106, 255),
            track_casing: Color::from_rgba(0, 0, 0, 245),
            station_fill: Color::from_rgba(238, 243, 251, 255),
            station_border: Color::from_rgba(0, 0, 0, 255),
            station_waiting: Color::from_rgba(245, 177, 76, 255),
            overlay: Color::from_rgba(0, 0, 0, 215),
        }
    }

    pub const fn light() -> Self {
        Palette {
            bg_top: Color::from_rgba(247, 249, 252, 255),
            bg_bottom: Color::from_rgba(232, 237, 244, 255),
            map_bg: Color::from_rgba(238, 242, 248, 255),
            map_grid: Color::from_rgba(150, 165, 190, 110),
            panel: Color::from_rgba(255, 255, 255, 250),
            panel_alt: Color::from_rgba(238, 242, 248, 250),
            panel_raised: Color::from_rgba(228, 233, 241, 255),
            border: Color::from_rgba(206, 214, 226, 255),
            border_strong: Color::from_rgba(160, 172, 192, 255),
            text_primary: Color::from_rgba(26, 32, 44, 255),
            text_secondary: Color::from_rgba(82, 94, 116, 255),
            text_muted: Color::from_rgba(122, 134, 154, 255),
            accent: Color::from_rgba(32, 120, 214, 255),
            accent_strong: Color::from_rgba(20, 96, 184, 255),
            on_accent: Color::from_rgba(255, 255, 255, 255),
            warning: Color::from_rgba(200, 130, 20, 255),
            danger: Color::from_rgba(206, 60, 60, 255),
            success: Color::from_rgba(38, 150, 84, 255),
            track_casing: Color::from_rgba(196, 204, 216, 210),
            station_fill: Color::from_rgba(255, 255, 255, 255),
            station_border: Color::from_rgba(60, 70, 90, 255),
            station_waiting: Color::from_rgba(210, 140, 30, 255),
            overlay: Color::from_rgba(236, 240, 246, 190),
        }
    }
}

pub const LINE_COLORS: [Color; 24] = [
    Color::from_rgba(229, 83, 75, 255),   // 0 красный
    Color::from_rgba(79, 163, 227, 255),  // 1 синий
    Color::from_rgba(86, 197, 150, 255),  // 2 зелёный
    Color::from_rgba(242, 177, 52, 255),  // 3 янтарный
    Color::from_rgba(176, 124, 232, 255), // 4 фиолетовый
    Color::from_rgba(236, 124, 171, 255), // 5 розовый
    Color::from_rgba(55, 196, 196, 255),  // 6 бирюзовый
    Color::from_rgba(227, 139, 79, 255),  // 7 оранжевый
    Color::from_rgba(160, 190, 70, 255),  // 8 лаймовый
    Color::from_rgba(120, 150, 235, 255), // 9 васильковый
    Color::from_rgba(214, 69, 112, 255),  // 10 малиновый
    Color::from_rgba(91, 200, 245, 255),  // 11 небесный
    Color::from_rgba(79, 209, 163, 255),  // 12 мятный
    Color::from_rgba(232, 197, 71, 255),  // 13 золотой
    Color::from_rgba(124, 124, 240, 255), // 14 индиго
    Color::from_rgba(240, 128, 90, 255),  // 15 коралловый
    Color::from_rgba(199, 125, 255, 255), // 16 сиреневый
    Color::from_rgba(255, 143, 163, 255), // 17 лососевый
    Color::from_rgba(90, 209, 224, 255),  // 18 аква
    Color::from_rgba(123, 223, 154, 255), // 19 весенний
    Color::from_rgba(207, 224, 90, 255),  // 20 лайм-салатовый
    Color::from_rgba(240, 176, 126, 255), // 21 персиковый
    Color::from_rgba(217, 140, 199, 255), // 22 орхидея
    Color::from_rgba(136, 153, 240, 255), // 23 сине-фиолетовый
];

pub const LINE_STYLES: [LineStyle; 10] = [
    LineStyle::Solid,
    LineStyle::Dashed,
    LineStyle::Dotted,
    LineStyle::DashDot,
    LineStyle::Solid,
    LineStyle::Dashed,
    LineStyle::Dotted,
    LineStyle::DashDot,
    LineStyle::Solid,
    LineStyle::Dashed,
];

pub fn line_style(index: usize) -> LineStyle {
    LINE_STYLES[index % LINE_STYLES.len()]
}

pub fn shade(color: Color, factor: f32) -> Color {
    Color::new(
        (color.r * factor).clamp(0.0, 1.0),
        (color.g * factor).clamp(0.0, 1.0),
        (color.b * factor).clamp(0.0, 1.0),
        color.a,
    )
}

pub fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

pub fn with_alpha(color: Color, a: f32) -> Color {
    Color::new(color.r, color.g, color.b, a)
}
