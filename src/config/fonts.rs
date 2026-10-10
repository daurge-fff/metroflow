use macroquad::prelude::{load_ttf_font_from_bytes, Font};

const REGULAR_BYTES: &[u8] = include_bytes!("../../assets/fonts/PTSans-Regular.ttf");
const BOLD_BYTES: &[u8] = include_bytes!("../../assets/fonts/PTSans-Bold.ttf");
const ICONS_BYTES: &[u8] = include_bytes!("../../assets/icons/MaterialIcons-Regular.ttf");

#[derive(Clone)]
pub struct Fonts {
    pub regular: Option<Font>,
    pub bold: Option<Font>,
    pub icons: Option<Font>,
}

impl Fonts {
    pub fn load() -> Self {
        let regular = load_ttf_font_from_bytes(REGULAR_BYTES).ok();
        let bold = load_ttf_font_from_bytes(BOLD_BYTES).ok();
        let icons = load_ttf_font_from_bytes(ICONS_BYTES).ok();
        if regular.is_none() {
            macroquad::logging::warn!(
                "Не удалось загрузить шрифт PT Sans, используется встроенный"
            );
        }
        if icons.is_none() {
            macroquad::logging::warn!("Не удалось загрузить шрифт иконок");
        }
        Fonts {
            regular,
            bold,
            icons,
        }
    }

    pub fn regular(&self) -> Option<&Font> {
        self.regular.as_ref()
    }

    pub fn bold(&self) -> Option<&Font> {
        self.bold.as_ref().or(self.regular.as_ref())
    }

    pub fn icons(&self) -> Option<&Font> {
        self.icons.as_ref()
    }
}
