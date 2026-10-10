use crate::geom::{Rect, Vec2};
use crate::network::{LineId, Network, StationId};
use crate::render::{shapes, text};
use crate::ui::icons::{self, Icon};
use crate::ui::layout::Layout;
use crate::ui::widgets::{ButtonStyle, UiContext};
use crate::ui::{EditorTool, UiAction};
use macroquad::prelude::*;

pub struct EditorView {
    pub tool: EditorTool,
    pub active_line: Option<LineId>,
    pub color_index: usize,
    pub auto_trains: bool,
    pub auto_cars: bool,
    pub trains_now: u32,
    pub cars_now: u32,
}

pub fn draw_toolbar(
    ui: &mut UiContext,
    layout: &Layout,
    tool: EditorTool,
    active_line: Option<&str>,
) {
    let tools = [
        EditorTool::Select,
        EditorTool::Station,
        EditorTool::Link,
        EditorTool::Erase,
    ];
    let _ = active_line;
    let btn = 46.0;
    let gap = 8.0;
    let pad = 10.0;
    let sep = 14.0;
    let n = tools.len() + 2;
    let w = pad * 2.0 + btn * n as f32 + gap * (n as f32 - 1.0) + sep;
    let h = 62.0;
    let rect = Rect::new(
        layout.map_viewport.pos.x + 16.0,
        layout.map_viewport.pos.y + 14.0,
        w,
        h,
    );
    ui.panel(rect);
    let slot = |i: usize| -> Rect {
        let extra = if i >= tools.len() { sep } else { 0.0 };
        let bx = rect.pos.x + pad + i as f32 * (btn + gap) + extra;
        Rect::new(bx, rect.pos.y + 8.0, btn, 34.0)
    };
    for (i, t) in tools.iter().enumerate() {
        let b = slot(i);
        let selected = *t == tool;
        ui.icon_button(b, t.icon(), selected, true, UiAction::SetTool(*t));
        let color = if selected {
            ui.palette.accent_strong
        } else {
            ui.palette.text_muted
        };
        text::draw_centered(
            ui.fonts,
            match ui.lang {
                crate::config::i18n::Lang::Ru => t.label(),
                crate::config::i18n::Lang::En => t.label_en(),
            },
            Vec2::new(b.center().x, rect.pos.y + 52.0),
            11.0,
            color,
            selected,
        );
    }
    let actions = [
        (Icon::Undo, ui.tr("Отмена", "Undo"), UiAction::EditorUndo),
        (Icon::Redo, ui.tr("Повтор", "Redo"), UiAction::EditorRedo),
    ];
    for (k, (icon, label, action)) in actions.iter().enumerate() {
        let b = slot(tools.len() + k);
        ui.icon_button(b, *icon, false, true, action.clone());
        text::draw_centered(
            ui.fonts,
            label,
            Vec2::new(b.center().x, rect.pos.y + 52.0),
            11.0,
            ui.palette.text_muted,
            false,
        );
    }
}

pub fn draw_panel(
    ui: &mut UiContext,
    network: &Network,
    view: &EditorView,
    selected_station: Option<StationId>,
    rename_buffer: Option<&str>,
    scroll: f32,
    layout: &Layout,
) -> f32 {
    let area = layout.side_panel;
    ui.note_region(area);
    draw_rectangle(
        area.pos.x,
        area.pos.y,
        area.size.x,
        area.size.y,
        ui.palette.panel,
    );
    draw_line(
        area.pos.x,
        area.pos.y,
        area.pos.x,
        area.bottom(),
        1.5,
        ui.palette.border,
    );

    let pad = 18.0;
    let x = area.pos.x + pad;
    let w = area.size.x - pad * 2.0;
    let origin = area.pos.y + 18.0;
    let mut y = origin - scroll;

    text::draw(
        ui.fonts,
        ui.tr("Редактор карты", "Map editor"),
        Vec2::new(x, y),
        22.0,
        ui.palette.text_primary,
        true,
    );
    y += 30.0;
    text::draw(
        ui.fonts,
        &ui.tf(
            "Инструмент: {t}",
            "Tool: {t}",
            &[(
                "t",
                match ui.lang {
                    crate::config::i18n::Lang::Ru => view.tool.label(),
                    crate::config::i18n::Lang::En => view.tool.label_en(),
                },
            )],
        ),
        Vec2::new(x, y),
        13.0,
        ui.palette.accent_strong,
        false,
    );
    y += 26.0;

    let hint = match view.tool {
        EditorTool::Select => ui.tr(
            "Кликните станцию, чтобы выбрать её.",
            "Click a station to select it.",
        ),
        EditorTool::Station => ui.tr(
            "Кликните по пустому месту карты, чтобы поставить станцию.",
            "Click empty space to place a station.",
        ),
        EditorTool::Link => ui.tr(
            "Выберите линию ниже и кликайте станции, чтобы вести ветку.",
            "Pick a line below and click stations to draw it.",
        ),
        EditorTool::Erase => ui.tr(
            "Кликните станцию, чтобы удалить её со всех линий.",
            "Click a station to delete it from all lines.",
        ),
    };
    y = draw_wrapped(ui, hint, x, w, y, ui.palette.text_muted, 12.0) + 8.0;

    ui.divider(Rect::new(x, y, w, 0.0), ui.palette.border);
    y += 14.0;

    let gap = 10.0;
    let half = (w - gap) * 0.5;
    let new_rect = Rect::new(x, y, half, 40.0);
    ui.action_button(
        new_rect,
        ui.tr("+ Новая линия", "+ New line"),
        ButtonStyle::Primary,
        true,
        UiAction::NewLine,
    );
    let expand_rect = Rect::new(x + half + gap, y, half, 40.0);
    ui.icon_text_button(
        expand_rect,
        Icon::AddCircle,
        ui.tr("Расширить сеть", "Expand network"),
        ButtonStyle::Secondary,
        true,
        UiAction::ExpandNetwork,
    );
    y += 50.0;

    text::draw(
        ui.fonts,
        ui.tr("Линии", "Lines"),
        Vec2::new(x, y),
        13.0,
        ui.palette.text_muted,
        true,
    );
    y += 20.0;
    if network.lines.is_empty() {
        text::draw(
            ui.fonts,
            ui.tr("Линий пока нет", "No lines yet"),
            Vec2::new(x, y),
            13.0,
            ui.palette.text_muted,
            false,
        );
        y += 28.0;
    }
    let row_h = 34.0;
    for line in &network.lines {
        let row = Rect::new(x, y, w - 34.0, row_h);
        let active = view.active_line == Some(line.id);
        if ui.list_row(row, active) {
            ui.emit(UiAction::SelectLine(line.id));
        }
        let color = Color::from_rgba(line.color[0], line.color[1], line.color[2], 255);
        icons::line_chip(Rect::new(x + 6.0, y + 13.0, 26.0, 8.0), color, line.style);
        text::draw(
            ui.fonts,
            &line.name,
            Vec2::new(x + 40.0, y + 7.0),
            14.0,
            ui.palette.text_primary,
            false,
        );
        text::draw_right(
            ui.fonts,
            &ui.tf(
                "{n} ст.",
                "{n} st.",
                &[("n", &line.stations.len().to_string())],
            ),
            Vec2::new(x + w - 42.0, y + 8.0),
            12.0,
            ui.palette.text_muted,
            false,
        );
        let del = Rect::new(x + w - 30.0, y + 5.0, 26.0, 24.0);
        ui.icon_button(
            del,
            Icon::Delete,
            false,
            true,
            UiAction::DeleteLine(line.id),
        );
        y += row_h + 4.0;
    }

    if let Some(active_id) = view.active_line {
        if let Some(line) = network.line(active_id) {
            y += 6.0;
            ui.divider(Rect::new(x, y, w, 0.0), ui.palette.border);
            y += 14.0;
            text::draw(
                ui.fonts,
                ui.tr("Активная линия", "Active line"),
                Vec2::new(x, y),
                13.0,
                ui.palette.text_muted,
                true,
            );
            y += 22.0;
            text::draw(
                ui.fonts,
                &line.name,
                Vec2::new(x, y),
                16.0,
                ui.palette.text_primary,
                true,
            );
            y += 26.0;

            text::draw(
                ui.fonts,
                ui.tr("Цвет", "Color"),
                Vec2::new(x, y),
                12.0,
                ui.palette.text_muted,
                true,
            );
            y += 20.0;
            let cols = 6usize;
            let gap = 7.0;
            let sw = (w - gap * (cols as f32 - 1.0)) / cols as f32;
            let colors = crate::config::palette::LINE_COLORS;
            for (i, c) in colors.iter().enumerate() {
                let row = (i / cols) as f32;
                let col = (i % cols) as f32;
                let rect = Rect::new(x + col * (sw + gap), y + row * (sw + gap), sw, sw);
                let rgb = [
                    (c.r * 255.0).round() as u8,
                    (c.g * 255.0).round() as u8,
                    (c.b * 255.0).round() as u8,
                ];
                let selected = rgb == line.color;
                shapes::fill_round_rect(rect.pos.x, rect.pos.y, rect.size.x, rect.size.y, 7.0, *c);
                let (bw, bc) = if selected {
                    (3.0, ui.palette.accent_strong)
                } else {
                    (1.5, ui.palette.border)
                };
                shapes::stroke_round_rect(
                    rect.pos.x,
                    rect.pos.y,
                    rect.size.x,
                    rect.size.y,
                    7.0,
                    bw,
                    bc,
                );
                if rect.contains(ui.mouse) {
                    ui.pointer_over_ui = true;
                    if ui.pressed {
                        ui.emit(UiAction::SetLineColor(active_id, i));
                    }
                }
            }
            let rows = colors.len().div_ceil(cols) as f32;
            y += rows * (sw + gap) + 4.0;

            let style_btn = Rect::new(x, y, w, 32.0);
            ui.action_button(
                style_btn,
                &ui.tf(
                    "Стиль: {s}",
                    "Style: {s}",
                    &[(
                        "s",
                        match ui.lang {
                            crate::config::i18n::Lang::Ru => line.style.label(),
                            crate::config::i18n::Lang::En => line.style.label_en(),
                        },
                    )],
                ),
                ButtonStyle::Secondary,
                true,
                UiAction::CycleLineStyle,
            );
            y += 40.0;

            if !view.auto_trains {
                manual_stepper(
                    ui,
                    Rect::new(x, y, w, 30.0),
                    ui.tr("Поездов на направление", "Trains per direction"),
                    view.trains_now,
                    UiAction::AdjustLineTrains(active_id, -1),
                    UiAction::AdjustLineTrains(active_id, 1),
                );
                y += 38.0;
            }
            if !view.auto_cars {
                manual_stepper(
                    ui,
                    Rect::new(x, y, w, 30.0),
                    ui.tr("Вагонов в составе", "Cars per train"),
                    view.cars_now,
                    UiAction::AdjustLineCars(active_id, -1),
                    UiAction::AdjustLineCars(active_id, 1),
                );
                y += 38.0;
            }

            let pop = Rect::new(x, y, w, 32.0);
            ui.icon_text_button(
                pop,
                Icon::Undo,
                ui.tr("Убрать последнюю станцию", "Remove last station"),
                ButtonStyle::Ghost,
                !line.stations.is_empty(),
                UiAction::RemoveLineLastStation(active_id),
            );
            y += 42.0;
        }
    }

    y += 6.0;
    ui.divider(Rect::new(x, y, w, 0.0), ui.palette.border);
    y += 14.0;
    match selected_station.and_then(|id| network.station(id)) {
        Some(station) => {
            text::draw(
                ui.fonts,
                ui.tr("Выделенная станция", "Selected station"),
                Vec2::new(x, y),
                13.0,
                ui.palette.text_muted,
                true,
            );
            y += 20.0;
            match rename_buffer {
                Some(buffer) => {
                    let field = Rect::new(x, y, w, 38.0);
                    shapes::fill_round_rect(
                        field.pos.x,
                        field.pos.y,
                        field.size.x,
                        field.size.y,
                        8.0,
                        ui.palette.panel_raised,
                    );
                    shapes::stroke_round_rect(
                        field.pos.x,
                        field.pos.y,
                        field.size.x,
                        field.size.y,
                        8.0,
                        1.5,
                        ui.palette.accent,
                    );
                    let caret = if ((ui.time * 2.0) as i32) % 2 == 0 {
                        "|"
                    } else {
                        ""
                    };
                    text::draw(
                        ui.fonts,
                        &format!("{buffer}{caret}"),
                        Vec2::new(field.pos.x + 10.0, field.pos.y + 10.0),
                        16.0,
                        ui.palette.text_primary,
                        false,
                    );
                    y += 30.0;
                    text::draw(
                        ui.fonts,
                        ui.tr(
                            "Enter — сохранить, Esc — отмена",
                            "Enter — save, Esc — cancel",
                        ),
                        Vec2::new(x, y),
                        11.0,
                        ui.palette.text_muted,
                        false,
                    );
                    y += 24.0;
                }
                None => {
                    text::draw(
                        ui.fonts,
                        &station.name,
                        Vec2::new(x, y),
                        16.0,
                        ui.palette.text_primary,
                        true,
                    );
                    y += 26.0;
                    let rename = Rect::new(x, y, w, 34.0);
                    ui.icon_text_button(
                        rename,
                        Icon::Edit,
                        ui.tr("Переименовать", "Rename"),
                        ButtonStyle::Secondary,
                        true,
                        UiAction::StartRename(station.id),
                    );
                    y += 42.0;
                }
            }
            if !station.lines.is_empty() {
                text::draw(
                    ui.fonts,
                    ui.tr("Отсоединить от линии", "Detach from line"),
                    Vec2::new(x, y),
                    12.0,
                    ui.palette.text_muted,
                    true,
                );
                y += 20.0;
                for line_id in &station.lines {
                    let (name, color, style) = match network.line(*line_id) {
                        Some(line) => (
                            line.name.clone(),
                            Color::from_rgba(line.color[0], line.color[1], line.color[2], 255),
                            line.style,
                        ),
                        None => continue,
                    };
                    let row = Rect::new(x, y, w, 30.0);
                    ui.note_region(row);
                    icons::line_chip(Rect::new(x + 2.0, y + 11.0, 24.0, 8.0), color, style);
                    text::draw(
                        ui.fonts,
                        &name,
                        Vec2::new(x + 34.0, y + 6.0),
                        14.0,
                        ui.palette.text_primary,
                        false,
                    );
                    let unlink = Rect::new(x + w - 30.0, y + 3.0, 26.0, 24.0);
                    ui.icon_button(
                        unlink,
                        Icon::Close,
                        false,
                        true,
                        UiAction::DetachFromLine(*line_id, station.id),
                    );
                    y += 32.0;
                }
                y += 6.0;
            }

            let del = Rect::new(x, y, w, 34.0);
            ui.icon_text_button(
                del,
                Icon::Delete,
                ui.tr("Удалить станцию", "Delete station"),
                ButtonStyle::Ghost,
                true,
                UiAction::DeleteStation(station.id),
            );
            y += 42.0;
        }
        None => {
            text::draw(
                ui.fonts,
                ui.tr("Станция не выделена", "No station selected"),
                Vec2::new(x, y),
                13.0,
                ui.palette.text_muted,
                false,
            );
            y += 24.0;
        }
    }

    (y + 8.0) - (origin - scroll)
}

fn manual_stepper(
    ui: &mut UiContext,
    rect: Rect,
    label: &str,
    value: u32,
    dec: UiAction,
    inc: UiAction,
) {
    let (x, y, w) = (rect.pos.x, rect.pos.y, rect.size.x);
    text::draw(
        ui.fonts,
        label,
        Vec2::new(x, y + 7.0),
        13.0,
        ui.palette.text_primary,
        false,
    );
    let bw = 32.0;
    let bh = 30.0;
    let plus = Rect::new(x + w - bw, y, bw, bh);
    let minus = Rect::new(x + w - bw * 2.0 - 8.0, y, bw, bh);
    ui.icon_button(minus, Icon::Minus, false, true, dec);
    ui.icon_button(plus, Icon::Plus, false, true, inc);
    text::draw_right(
        ui.fonts,
        &value.to_string(),
        Vec2::new(x + w - bw * 2.0 - 14.0, y + 6.0),
        15.0,
        ui.palette.text_primary,
        true,
    );
}

fn draw_wrapped(
    ui: &mut UiContext,
    text_str: &str,
    x: f32,
    w: f32,
    mut y: f32,
    color: Color,
    size: f32,
) -> f32 {
    let mut line = String::new();
    for word in text_str.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if text::measure(ui.fonts, &candidate, size, false).x > w {
            text::draw(ui.fonts, &line, Vec2::new(x, y), size, color, false);
            y += size + 6.0;
            line = word.to_string();
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        text::draw(ui.fonts, &line, Vec2::new(x, y), size, color, false);
        y += size + 6.0;
    }
    y
}
