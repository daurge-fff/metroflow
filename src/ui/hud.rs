use crate::config::palette::with_alpha;
use crate::config::Settings;
use crate::geom::{Rect, Vec2};
use crate::network::{Network, StationId, StationKind};
use crate::render::{shapes, text};
use crate::simulation::Simulation;
use crate::ui::icons::{self, Icon};
use crate::ui::layout::Layout;
use crate::ui::widgets::{ButtonStyle, UiContext};
use crate::ui::UiAction;
use macroquad::prelude::*;
use std::collections::VecDeque;

const BTN: f32 = 40.0;

pub struct History {
    pub waiting: VecDeque<f32>,
    pub onboard: VecDeque<f32>,
    pub avg_wait: VecDeque<f32>,
    pub throughput: VecDeque<f32>,
    timer: f32,
    last_delivered: f32,
}

const HISTORY_CAP: usize = 120;

impl Default for History {
    fn default() -> Self {
        History {
            waiting: VecDeque::new(),
            onboard: VecDeque::new(),
            avg_wait: VecDeque::new(),
            throughput: VecDeque::new(),
            timer: 0.0,
            last_delivered: 0.0,
        }
    }
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn sample(&mut self, dt: f32, waiting: f32, onboard: f32, avg_wait: f32, delivered: f32) {
        self.timer += dt;
        if self.timer < 0.4 {
            return;
        }
        let rate = (delivered - self.last_delivered) / self.timer;
        self.last_delivered = delivered;
        self.timer = 0.0;
        push_capped(&mut self.waiting, waiting);
        push_capped(&mut self.onboard, onboard);
        push_capped(&mut self.avg_wait, avg_wait);
        push_capped(&mut self.throughput, rate.max(0.0));
    }
}

fn push_capped(buf: &mut VecDeque<f32>, value: f32) {
    buf.push_back(value);
    while buf.len() > HISTORY_CAP {
        buf.pop_front();
    }
}

fn transfers_label(ui: &UiContext, n: usize) -> String {
    match ui.lang {
        crate::config::i18n::Lang::Ru => match n {
            0 => "без пересадок".to_string(),
            1 => "1 пересадка".to_string(),
            n => format!("{n} пересадки"),
        },
        crate::config::i18n::Lang::En => match n {
            0 => "no transfers".to_string(),
            1 => "1 transfer".to_string(),
            n => format!("{n} transfers"),
        },
    }
}

pub fn format_duration(lang: crate::config::i18n::Lang, seconds: f32) -> String {
    if seconds < 60.0 {
        match lang {
            crate::config::i18n::Lang::Ru => format!("{seconds:.0} с"),
            crate::config::i18n::Lang::En => format!("{seconds:.0} s"),
        }
    } else {
        let m = (seconds / 60.0).floor() as u32;
        let s = seconds as u32 % 60;
        match lang {
            crate::config::i18n::Lang::Ru => format!("{m} м {s:02} с"),
            crate::config::i18n::Lang::En => format!("{m} m {s:02} s"),
        }
    }
}

pub fn draw_top_bar(
    ui: &mut UiContext,
    network: &Network,
    sim: &Simulation,
    settings: &Settings,
    layout: &Layout,
    edit_mode: bool,
    congested: usize,
) {
    let bar = layout.top_bar;
    ui.note_region(bar);
    draw_rectangle(
        bar.pos.x,
        bar.pos.y,
        bar.size.x,
        bar.size.y,
        ui.palette.panel_alt,
    );
    draw_line(
        bar.pos.x,
        bar.bottom(),
        bar.right(),
        bar.bottom(),
        1.5,
        ui.palette.border,
    );

    let logo_center = Vec2::new(38.0, bar.size.y * 0.5);
    draw_circle(
        logo_center.x,
        logo_center.y,
        13.0,
        with_alpha(ui.palette.accent, 0.18),
    );
    draw_circle_lines(logo_center.x, logo_center.y, 13.0, 2.0, ui.palette.accent);
    draw_line(
        logo_center.x - 6.0,
        logo_center.y,
        logo_center.x + 6.0,
        logo_center.y,
        2.0,
        ui.palette.accent_strong,
    );
    draw_line(
        logo_center.x,
        logo_center.y - 6.0,
        logo_center.x,
        logo_center.y + 6.0,
        2.0,
        ui.palette.accent_strong,
    );

    let roomy = bar.size.x >= 1040.0;
    if roomy {
        text::draw(
            ui.fonts,
            "MetroFlow",
            Vec2::new(60.0, 12.0),
            22.0,
            ui.palette.text_primary,
            true,
        );
        let status = if sim.clock.paused {
            ui.tr("на паузе", "paused")
        } else {
            ui.tr("симуляция идёт", "running")
        };
        text::draw(
            ui.fonts,
            &format!("{} · {}", network.name, status),
            Vec2::new(61.0, 38.0),
            12.0,
            ui.palette.text_muted,
            false,
        );
    }

    let mut x = bar.right() - 16.0;
    let y = (bar.size.y - BTN) * 0.5;

    let menu = Rect::new(x - BTN, y, BTN, BTN);
    x -= BTN + 8.0;
    let load_btn = Rect::new(x - BTN, y, BTN, BTN);
    x -= BTN + 8.0;
    let save_btn = Rect::new(x - BTN, y, BTN, BTN);
    x -= BTN + 8.0;
    let edit_btn = Rect::new(x - BTN, y, BTN, BTN);
    x -= BTN + 8.0;
    let chart_btn = Rect::new(x - BTN, y, BTN, BTN);
    x -= BTN + 18.0;

    let seg_w = if roomy { 216.0 } else { 150.0 };
    let seg_h = 38.0;
    let seg = Rect::new(x - seg_w, (bar.size.y - seg_h) * 0.5, seg_w, seg_h);
    x -= seg_w + 14.0;

    let pause_btn = Rect::new(x - BTN, y, BTN, BTN);
    x -= BTN + 22.0;

    ui.icon_button(menu, Icon::Menu, false, true, UiAction::OpenMenu);
    ui.icon_button(load_btn, Icon::Load, false, true, UiAction::LoadGame);
    ui.icon_button(save_btn, Icon::Save, false, true, UiAction::SaveGame);
    ui.icon_button(edit_btn, Icon::Edit, edit_mode, true, UiAction::ToggleEdit);
    ui.icon_button(chart_btn, Icon::Chart, false, true, UiAction::OpenAnalytics);
    ui.segmented(
        seg,
        &["1×", "2×", "4×", "8×", "16×"],
        settings.time_scale_index,
        UiAction::SetScale,
    );
    let pause_icon = if sim.clock.paused {
        Icon::Play
    } else {
        Icon::Pause
    };
    ui.icon_button(
        pause_btn,
        pause_icon,
        sim.clock.paused,
        true,
        UiAction::TogglePause,
    );

    let time_right = x;
    text::draw_right(
        ui.fonts,
        &sim.clock.format_time(),
        Vec2::new(time_right, 10.0),
        26.0,
        ui.palette.text_primary,
        true,
    );
    text::draw_right(
        ui.fonts,
        &ui.tf(
            "День {d}",
            "Day {d}",
            &[("d", &sim.clock.day().to_string())],
        ),
        Vec2::new(time_right, 40.0),
        12.0,
        ui.palette.text_muted,
        false,
    );

    let waiting = sim.total_waiting();
    let stat_x = time_right - 170.0;
    if stat_x > 320.0 {
        text::draw_right(
            ui.fonts,
            &ui.tf(
                "Ожидают: {n}",
                "Waiting: {n}",
                &[("n", &waiting.to_string())],
            ),
            Vec2::new(stat_x, 16.0),
            14.0,
            ui.palette.text_secondary,
            false,
        );
        if congested > 0 {
            text::draw_right(
                ui.fonts,
                &ui.tf(
                    "Перегружено станций: {n}",
                    "Congested stations: {n}",
                    &[("n", &congested.to_string())],
                ),
                Vec2::new(stat_x, 37.0),
                14.0,
                ui.palette.danger,
                true,
            );
        } else {
            text::draw_right(
                ui.fonts,
                &ui.tf(
                    "В поездах: {n}",
                    "Onboard: {n}",
                    &[("n", &sim.total_onboard().to_string())],
                ),
                Vec2::new(stat_x, 37.0),
                14.0,
                ui.palette.text_secondary,
                false,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_side_panel(
    ui: &mut UiContext,
    network: &Network,
    sim: &Simulation,
    selection: Option<StationId>,
    history: &History,
    preview: Option<(StationId, StationId)>,
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
    let mut y = area.pos.y + 18.0 - scroll;

    match selection.and_then(|id| network.station(id)) {
        Some(station) => {
            draw_station_card(ui, network, sim, station, preview, x, w, &mut y);
        }
        None => {
            draw_network_card(ui, network, sim, history, x, w, &mut y);
        }
    }
    y - (area.pos.y + 18.0) + scroll
}

#[allow(clippy::too_many_arguments)]
fn draw_station_card(
    ui: &mut UiContext,
    network: &Network,
    sim: &Simulation,
    station: &crate::network::Station,
    preview: Option<(StationId, StationId)>,
    x: f32,
    w: f32,
    y: &mut f32,
) {
    let badge_label = match ui.lang {
        crate::config::i18n::Lang::Ru => station.kind.label(),
        crate::config::i18n::Lang::En => station.kind.label_en(),
    };
    let badge_w = text::measure(ui.fonts, badge_label, 12.0, false).x + 20.0;
    let (badge_color, badge_text) = match station.kind {
        StationKind::Central => (ui.palette.accent, ui.palette.map_bg),
        StationKind::Interchange => (ui.palette.warning, ui.palette.map_bg),
        StationKind::Regular => (ui.palette.panel_raised, ui.palette.text_secondary),
    };
    shapes::fill_round_rect(x, *y, badge_w, 22.0, 7.0, badge_color);
    text::draw_centered(
        ui.fonts,
        badge_label,
        Vec2::new(x + badge_w * 0.5, *y + 11.0),
        12.0,
        badge_text,
        false,
    );
    *y += 32.0;

    text::draw(
        ui.fonts,
        &station.name,
        Vec2::new(x, *y),
        22.0,
        ui.palette.text_primary,
        true,
    );
    *y += 36.0;

    ui.divider(Rect::new(x, *y, w, 0.0), ui.palette.border);
    *y += 14.0;

    let now = sim.now();
    let waiting = sim.passengers.waiting_at(station.id);
    let trains = sim.trains_serving(network, station.id);
    let load = average_load_at(sim, network, station.id);
    stat_row_icon(
        ui,
        Icon::Person,
        ui.tr("Ожидают пассажиры", "Waiting passengers"),
        &waiting.to_string(),
        x,
        w,
        y,
    );
    let avg_label = ui.tr("Среднее ожидание", "Average wait");
    match sim.passengers.average_wait(station.id, now) {
        Some(avg) => stat_row_icon(
            ui,
            Icon::Clock,
            avg_label,
            &format_duration(ui.lang, avg),
            x,
            w,
            y,
        ),
        None => stat_row_icon(ui, Icon::Clock, avg_label, "—", x, w, y),
    }
    stat_row_icon(
        ui,
        Icon::Route,
        ui.tr("Обслуживающих поездов", "Serving trains"),
        &trains.to_string(),
        x,
        w,
        y,
    );
    stat_row_icon(
        ui,
        Icon::Check,
        ui.tr("Доставлено к цели", "Delivered"),
        &sim.passengers.served_at(station.id).to_string(),
        x,
        w,
        y,
    );

    *y += 4.0;
    load_bar(
        ui,
        ui.tr("Загруженность поездов", "Train load"),
        load,
        x,
        w,
        y,
    );
    *y += 8.0;

    let breakdown = sim.passengers.destination_breakdown(station.id);
    if !breakdown.is_empty() {
        ui.divider(Rect::new(x, *y, w, 0.0), ui.palette.border);
        *y += 12.0;
        text::draw(
            ui.fonts,
            ui.tr("Куда едут ожидающие", "Where passengers go"),
            Vec2::new(x, *y),
            13.0,
            ui.palette.text_muted,
            true,
        );
        *y += 20.0;
        let max_count = breakdown[0].1.max(1);
        for (destination, count) in breakdown.iter().take(5) {
            let name = network
                .station(*destination)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| "?".to_string());
            let selected = preview == Some((station.id, *destination));
            let row = Rect::new(x - 6.0, *y - 4.0, w + 12.0, 34.0);
            let clicked = ui.list_row(row, selected);
            text::draw(
                ui.fonts,
                &name,
                Vec2::new(x, *y),
                14.0,
                ui.palette.text_primary,
                false,
            );
            let transfers = sim
                .plan(network, station.id, *destination)
                .map(|route| transfers_label(ui, route.transfers))
                .unwrap_or_default();
            text::draw(
                ui.fonts,
                &transfers,
                Vec2::new(x + 148.0, *y + 2.0),
                11.0,
                ui.palette.text_muted,
                false,
            );
            let count_str = count.to_string();
            let cw = text::measure(ui.fonts, &count_str, 13.0, true).x + 16.0;
            shapes::fill_round_rect(
                x + w - cw,
                *y - 3.0,
                cw,
                22.0,
                7.0,
                with_alpha(ui.palette.accent, 0.18),
            );
            text::draw_centered(
                ui.fonts,
                &count_str,
                Vec2::new(x + w - cw * 0.5, *y + 8.0),
                13.0,
                ui.palette.accent_strong,
                true,
            );
            let bar_w = w * (*count as f32 / max_count as f32);
            shapes::fill_round_rect(
                x,
                *y + 20.0,
                w,
                3.0,
                1.5,
                with_alpha(ui.palette.border, 0.6),
            );
            shapes::fill_round_rect(x, *y + 20.0, bar_w.max(2.0), 3.0, 1.5, ui.palette.accent);
            if clicked {
                ui.emit(UiAction::PreviewRoute(station.id, *destination));
            }
            *y += 32.0;
        }
        *y += 4.0;
    }

    if let Some((origin, destination)) = preview {
        if origin == station.id {
            if let Some(route) = sim.plan(network, origin, destination) {
                *y += 4.0;
                ui.divider(Rect::new(x, *y, w, 0.0), ui.palette.border);
                *y += 12.0;
                let dest_name = network
                    .station(destination)
                    .map(|s| s.name.clone())
                    .unwrap_or_default();
                text::draw(
                    ui.fonts,
                    &ui.tf("Маршрут до «{d}»", "Route to “{d}”", &[("d", &dest_name)]),
                    Vec2::new(x, *y),
                    14.0,
                    ui.palette.accent_strong,
                    true,
                );
                text::draw_right(
                    ui.fonts,
                    &format!(
                        "≈ {:.0} {} · {}",
                        route.total_time,
                        ui.tr("с", "s"),
                        transfers_label(ui, route.transfers)
                    ),
                    Vec2::new(x + w, *y + 1.0),
                    11.0,
                    ui.palette.text_muted,
                    false,
                );
                *y += 24.0;
                if route.legs.is_empty() {
                    text::draw(
                        ui.fonts,
                        ui.tr("Вы уже на месте", "You are already there"),
                        Vec2::new(x, *y),
                        13.0,
                        ui.palette.text_muted,
                        false,
                    );
                    *y += 22.0;
                }
                for (i, leg) in route.legs.iter().enumerate() {
                    let (line_name, color, style) = match network.line(leg.line) {
                        Some(line) => (
                            line.name.clone(),
                            Color::from_rgba(line.color[0], line.color[1], line.color[2], 255),
                            line.style,
                        ),
                        None => continue,
                    };
                    icons::line_chip(Rect::new(x, *y + 4.0, 22.0, 8.0), color, style);
                    text::draw(
                        ui.fonts,
                        &line_name,
                        Vec2::new(x + 30.0, *y),
                        14.0,
                        ui.palette.text_primary,
                        false,
                    );
                    text::draw_right(
                        ui.fonts,
                        &ui.tf(
                            "{n} ост.",
                            "{n} stops",
                            &[("n", &leg.stations.len().saturating_sub(1).to_string())],
                        ),
                        Vec2::new(x + w, *y + 1.0),
                        12.0,
                        ui.palette.text_secondary,
                        false,
                    );
                    *y += 22.0;
                    if i + 1 < route.legs.len() {
                        shapes::fill_round_rect(
                            x + 1.0,
                            *y + 4.0,
                            20.0,
                            3.0,
                            1.5,
                            ui.palette.warning,
                        );
                        text::draw(
                            ui.fonts,
                            &ui.tf(
                                "пересадка ≈ {t} с",
                                "transfer ≈ {t} s",
                                &[("t", &format!("{:.0}", crate::network::TRANSFER_PENALTY))],
                            ),
                            Vec2::new(x + 30.0, *y),
                            12.0,
                            ui.palette.warning,
                            false,
                        );
                        *y += 20.0;
                    }
                }
                *y += 6.0;
                let hide = Rect::new(x, *y, w, 30.0);
                ui.icon_text_button(
                    hide,
                    Icon::Close,
                    ui.tr("Скрыть маршрут", "Hide route"),
                    ButtonStyle::Ghost,
                    true,
                    UiAction::PreviewRoute(origin, destination),
                );
                *y += 36.0;
            }
        }
    }

    ui.divider(Rect::new(x, *y, w, 0.0), ui.palette.border);
    *y += 14.0;
    text::draw(
        ui.fonts,
        ui.tr("Подключённые линии", "Connected lines"),
        Vec2::new(x, *y),
        13.0,
        ui.palette.text_muted,
        true,
    );
    *y += 24.0;
    if station.lines.is_empty() {
        text::draw(
            ui.fonts,
            ui.tr("Нет данных", "No data"),
            Vec2::new(x, *y),
            13.0,
            ui.palette.text_muted,
            false,
        );
        *y += 24.0;
    }
    for line_id in &station.lines {
        if let Some(line) = network.line(*line_id) {
            let color = Color::from_rgba(line.color[0], line.color[1], line.color[2], 255);
            icons::line_chip(Rect::new(x, *y + 8.0, 30.0, 8.0), color, line.style);
            text::draw(
                ui.fonts,
                &line.name,
                Vec2::new(x + 40.0, *y),
                15.0,
                ui.palette.text_primary,
                false,
            );
            let style_label = match ui.lang {
                crate::config::i18n::Lang::Ru => line.style.label(),
                crate::config::i18n::Lang::En => line.style.label_en(),
            };
            text::draw_right(
                ui.fonts,
                style_label,
                Vec2::new(x + w, *y + 2.0),
                12.0,
                ui.palette.text_muted,
                false,
            );
            *y += 28.0;
        }
    }
}

fn draw_network_card(
    ui: &mut UiContext,
    network: &Network,
    sim: &Simulation,
    history: &History,
    x: f32,
    w: f32,
    y: &mut f32,
) {
    text::draw(
        ui.fonts,
        ui.tr("Обзор сети", "Network overview"),
        Vec2::new(x, *y),
        13.0,
        ui.palette.text_muted,
        true,
    );
    *y += 26.0;
    text::draw(
        ui.fonts,
        &network.name,
        Vec2::new(x, *y),
        22.0,
        ui.palette.text_primary,
        true,
    );
    *y += 40.0;

    ui.divider(Rect::new(x, *y, w, 0.0), ui.palette.border);
    *y += 14.0;

    stat_row(
        ui,
        ui.tr("Станций", "Stations"),
        &network.stations.len().to_string(),
        x,
        w,
        y,
    );
    stat_row(
        ui,
        ui.tr("Линий", "Lines"),
        &network.lines.len().to_string(),
        x,
        w,
        y,
    );
    stat_row(
        ui,
        ui.tr("Поездов", "Trains"),
        &sim.trains.len().to_string(),
        x,
        w,
        y,
    );
    stat_row(
        ui,
        ui.tr("Пассажиров ожидают", "Waiting passengers"),
        &sim.total_waiting().to_string(),
        x,
        w,
        y,
    );
    let avg_label = ui.tr("Среднее ожидание", "Average wait");
    match sim.passengers.network_average_wait(sim.now()) {
        Some(avg) => stat_row(ui, avg_label, &format_duration(ui.lang, avg), x, w, y),
        None => stat_row(ui, avg_label, "—", x, w, y),
    }
    let avg_served = ui.tr("Ожидание (в среднем)", "Wait (average)");
    match sim.passengers.average_served_wait() {
        Some(avg) => stat_row(ui, avg_served, &format_duration(ui.lang, avg), x, w, y),
        None => stat_row(ui, avg_served, "—", x, w, y),
    }
    *y += 4.0;
    load_bar(
        ui,
        ui.tr("Пик загрузки поезда", "Peak train load"),
        sim.peak_load(),
        x,
        w,
        y,
    );
    load_bar(
        ui,
        ui.tr("Средняя загрузка", "Average load"),
        sim.average_load(),
        x,
        w,
        y,
    );
    *y += 10.0;

    text::draw(
        ui.fonts,
        ui.tr("Доставлено всего", "Total delivered"),
        Vec2::new(x, *y),
        13.0,
        ui.palette.text_muted,
        true,
    );
    text::draw_right(
        ui.fonts,
        &sim.passengers.total_delivered.to_string(),
        Vec2::new(x + w, *y - 4.0),
        20.0,
        ui.palette.success,
        true,
    );
    *y += 28.0;

    draw_chart(
        ui,
        Rect::new(x, *y, w, 100.0),
        ui.tr("Очереди и пассажиры в пути", "Queues & passengers onboard"),
        &[
            (
                &history.waiting,
                ui.palette.station_waiting,
                ui.tr("Ожидают", "Waiting"),
            ),
            (
                &history.onboard,
                ui.palette.accent,
                ui.tr("В поездах", "Onboard"),
            ),
        ],
        true,
        Some((
            crate::render::map::ALERT_THRESHOLD as f32,
            ui.tr("порог", "threshold"),
        )),
    );
    *y += 112.0;

    draw_chart(
        ui,
        Rect::new(x, *y, w, 100.0),
        ui.tr("Среднее ожидание (секунды)", "Average wait (seconds)"),
        &[(
            &history.avg_wait,
            ui.palette.warning,
            ui.tr("Ожидание", "Waiting"),
        )],
        true,
        None,
    );
    *y += 112.0;

    let hint = ui.tr(
        "Выберите станцию, чтобы увидеть её показатели и направления пассажиров.",
        "Select a station to see its metrics and where passengers go.",
    );
    draw_wrapped(ui, hint, x, w, y, ui.palette.text_muted, 13.0);
}

pub fn draw_chart(
    ui: &mut UiContext,
    rect: Rect,
    title: &str,
    series: &[(&VecDeque<f32>, Color, &str)],
    fill_first: bool,
    threshold: Option<(f32, &str)>,
) {
    shapes::fill_round_rect(
        rect.pos.x,
        rect.pos.y,
        rect.size.x,
        rect.size.y,
        8.0,
        ui.palette.panel_raised,
    );
    text::draw(
        ui.fonts,
        title,
        Vec2::new(rect.pos.x + 10.0, rect.pos.y + 6.0),
        12.0,
        ui.palette.text_secondary,
        true,
    );

    let inner = Rect::new(
        rect.pos.x + 10.0,
        rect.pos.y + 24.0,
        rect.size.x - 20.0,
        rect.size.y - 52.0,
    );

    let mut max_value = series
        .iter()
        .flat_map(|(s, _, _)| s.iter())
        .copied()
        .fold(1.0f32, f32::max);
    if let Some((value, _)) = threshold {
        max_value = max_value.max(value * 1.15);
    }

    for i in 0..=2 {
        let y = inner.bottom() - inner.size.y * i as f32 / 2.0;
        draw_line(
            inner.pos.x,
            y,
            inner.right(),
            y,
            1.0,
            with_alpha(ui.palette.border, 0.5),
        );
    }

    if let Some((value, label)) = threshold {
        let y = inner.bottom() - inner.size.y * (value / max_value).min(1.0);
        shapes::stroke_polyline(
            &[
                macroquad::prelude::vec2(inner.pos.x, y),
                macroquad::prelude::vec2(inner.right(), y),
            ],
            1.5,
            with_alpha(ui.palette.danger, 0.7),
        );
        text::draw_right(
            ui.fonts,
            label,
            Vec2::new(inner.right(), y - 14.0),
            10.0,
            with_alpha(ui.palette.danger, 0.9),
            false,
        );
    }

    let step = inner.size.x / (HISTORY_CAP as f32 - 1.0);
    let points_of = |data: &VecDeque<f32>| -> Vec<macroquad::prelude::Vec2> {
        let n = data.len();
        data.iter()
            .enumerate()
            .map(|(i, v)| {
                macroquad::prelude::vec2(
                    inner.right() - (n - 1 - i) as f32 * step,
                    inner.bottom() - inner.size.y * (*v / max_value).clamp(0.0, 1.0),
                )
            })
            .collect()
    };

    for (index, (data, color, _)) in series.iter().enumerate() {
        let points = points_of(data);
        if points.len() < 2 {
            continue;
        }
        if fill_first && index == 0 {
            for seg in points.windows(2) {
                let quad = [
                    seg[0],
                    seg[1],
                    macroquad::prelude::vec2(seg[1].x, inner.bottom()),
                    macroquad::prelude::vec2(seg[0].x, inner.bottom()),
                ];
                shapes::fill_convex_polygon(&quad, with_alpha(*color, 0.18));
            }
        }
        shapes::stroke_polyline(&points, 2.0, *color);
        if let Some(last) = points.last() {
            draw_circle(last.x, last.y, 2.5, *color);
        }
    }

    let legend_y = rect.bottom() - 12.0;
    let mut lx = rect.pos.x + 10.0;
    for (data, color, label) in series {
        let value = data.back().copied().unwrap_or(0.0);
        shapes::fill_round_rect(lx, legend_y - 1.5, 14.0, 3.0, 1.5, *color);
        let text_str = format!("{label} {value:.0}");
        text::draw(
            ui.fonts,
            &text_str,
            Vec2::new(lx + 20.0, legend_y - 7.0),
            11.0,
            ui.palette.text_secondary,
            false,
        );
        lx += 20.0 + text::measure(ui.fonts, &text_str, 11.0, false).x + 14.0;
    }
}

fn stat_row_icon(
    ui: &mut UiContext,
    icon: Icon,
    label: &str,
    value: &str,
    x: f32,
    w: f32,
    y: &mut f32,
) {
    icons::draw(
        ui.fonts,
        icon,
        Rect::new(x, *y + 1.0, 15.0, 15.0),
        ui.palette.text_muted,
    );
    text::draw(
        ui.fonts,
        label,
        Vec2::new(x + 22.0, *y),
        14.0,
        ui.palette.text_secondary,
        false,
    );
    text::draw_right(
        ui.fonts,
        value,
        Vec2::new(x + w, *y - 2.0),
        16.0,
        ui.palette.text_primary,
        true,
    );
    *y += 28.0;
}

fn stat_row(ui: &mut UiContext, label: &str, value: &str, x: f32, w: f32, y: &mut f32) {
    text::draw(
        ui.fonts,
        label,
        Vec2::new(x, *y),
        14.0,
        ui.palette.text_secondary,
        false,
    );
    text::draw_right(
        ui.fonts,
        value,
        Vec2::new(x + w, *y - 2.0),
        16.0,
        ui.palette.text_primary,
        true,
    );
    *y += 28.0;
}

fn load_bar(ui: &mut UiContext, label: &str, value: f32, x: f32, w: f32, y: &mut f32) {
    text::draw(
        ui.fonts,
        label,
        Vec2::new(x, *y),
        13.0,
        ui.palette.text_secondary,
        false,
    );
    text::draw_right(
        ui.fonts,
        &format!("{:.0}%", value * 100.0),
        Vec2::new(x + w, *y - 1.0),
        14.0,
        ui.palette.text_primary,
        true,
    );
    *y += 20.0;
    let bar_h = 8.0;
    shapes::fill_round_rect(x, *y, w, bar_h, 4.0, ui.palette.panel_raised);
    let fill_w = (w * value.clamp(0.0, 1.0)).max(0.0);
    if fill_w > 2.0 {
        let color = load_color(ui, value);
        shapes::fill_round_rect(x, *y, fill_w, bar_h, 4.0, color);
    }
    *y += bar_h + 14.0;
}

fn load_color(ui: &mut UiContext, value: f32) -> Color {
    let _ = ui;
    if value < 0.5 {
        Color::from_rgba(102, 187, 106, 255)
    } else if value < 0.8 {
        Color::from_rgba(245, 177, 76, 255)
    } else {
        Color::from_rgba(239, 90, 90, 255)
    }
}

fn average_load_at(sim: &Simulation, network: &Network, station: StationId) -> f32 {
    let Some(st) = network.station(station) else {
        return 0.0;
    };
    let trains: Vec<_> = sim
        .trains
        .iter()
        .filter(|t| st.lines.contains(&t.line))
        .collect();
    if trains.is_empty() {
        return 0.0;
    }
    trains.iter().map(|t| t.load_factor()).sum::<f32>() / trains.len() as f32
}

fn draw_wrapped(
    ui: &mut UiContext,
    text_str: &str,
    x: f32,
    w: f32,
    y: &mut f32,
    color: Color,
    size: f32,
) {
    let mut line = String::new();
    for word in text_str.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if text::measure(ui.fonts, &candidate, size, false).x > w {
            text::draw(ui.fonts, &line, Vec2::new(x, *y), size, color, false);
            *y += size + 6.0;
            line = word.to_string();
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        text::draw(ui.fonts, &line, Vec2::new(x, *y), size, color, false);
        *y += size + 6.0;
    }
}

type ControlCell = Option<(Icon, &'static str, UiAction)>;

pub fn draw_controls(ui: &mut UiContext, settings: &Settings, layout: &Layout) {
    let card = layout.controls;
    ui.panel(card);

    let pad = 12.0;
    let btn = 46.0;
    let gap = 8.0;
    let mut hovered: Option<&str> = None;

    let rows: [[ControlCell; 4]; 3] = [
        [
            Some((Icon::Plus, ui.tr("Приблизить", "Zoom in"), UiAction::ZoomIn)),
            Some((
                Icon::Minus,
                ui.tr("Отдалить", "Zoom out"),
                UiAction::ZoomOut,
            )),
            Some((
                Icon::Fit,
                ui.tr("Вписать сеть", "Fit network"),
                UiAction::FitView,
            )),
            Some((
                Icon::Center,
                ui.tr("Центрировать", "Center"),
                UiAction::CenterNetwork,
            )),
        ],
        [
            Some((
                Icon::Tag,
                ui.tr("Подписи", "Labels"),
                UiAction::ToggleLabels,
            )),
            Some((Icon::Grid, ui.tr("Сетка", "Grid"), UiAction::ToggleGrid)),
            Some((
                Icon::Person,
                ui.tr("Пассажиры", "Passengers"),
                UiAction::ToggleWaiting,
            )),
            Some((
                Icon::Motion,
                ui.tr("Анимации", "Animations"),
                UiAction::ToggleMotion,
            )),
        ],
        [
            Some((
                Icon::Heat,
                ui.tr("Нагрузка станций", "Station load"),
                UiAction::ToggleHeat,
            )),
            Some((
                Icon::Flow,
                ui.tr("Поток по линиям", "Line flow"),
                UiAction::ToggleFlow,
            )),
            Some((Icon::Help, ui.tr("Справка", "Help"), UiAction::ToggleHelp)),
            None,
        ],
    ];
    let toggles = [
        [false, false, false, false],
        [
            settings.show_labels,
            settings.show_grid,
            settings.show_waiting,
            !settings.reduced_motion,
        ],
        [settings.show_heat, settings.show_flow, false, false],
    ];

    for (r, row) in rows.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            let rect = Rect::new(
                card.pos.x + pad + c as f32 * (btn + gap),
                card.pos.y + pad + r as f32 * (btn + gap),
                btn,
                btn,
            );
            let Some((icon, name, action)) = cell else {
                ui.icon_button(rect, Icon::Close, false, true, UiAction::ToggleControls);
                if rect.contains(ui.mouse) {
                    hovered = Some(ui.tr("Свернуть", "Hide"));
                }
                continue;
            };
            let toggled = toggles[r][c];
            ui.icon_button(rect, *icon, toggled, true, action.clone());
            if rect.contains(ui.mouse) {
                hovered = Some(name);
            }
        }
    }

    if let Some(name) = hovered {
        let dims = text::measure(ui.fonts, name, 13.0, false);
        let tw = dims.x + 20.0;
        let ty = card.pos.y - 34.0;
        let tx = card.pos.x;
        shapes::fill_round_rect(
            tx,
            ty,
            tw,
            26.0,
            8.0,
            with_alpha(ui.palette.panel_raised, 0.98),
        );
        shapes::stroke_round_rect(tx, ty, tw, 26.0, 8.0, 1.0, ui.palette.border);
        text::draw_centered(
            ui.fonts,
            name,
            Vec2::new(tx + tw * 0.5, ty + 13.0),
            13.0,
            ui.palette.text_primary,
            false,
        );
    }
}

pub fn draw_controls_collapsed(ui: &mut UiContext, layout: &Layout) {
    let card = layout.controls;
    let b = Rect::new(card.pos.x, card.bottom() - 46.0, 46.0, 46.0);
    ui.icon_button(b, Icon::Layers, false, true, UiAction::ToggleControls);
}

pub fn draw_help_overlay(ui: &mut UiContext, layout: &Layout) {
    let win = layout.window;
    draw_rectangle(0.0, 0.0, win.x, win.y, with_alpha(ui.palette.map_bg, 0.55));
    let card_w = (win.x * 0.8).clamp(420.0, 620.0);
    let card_h = 470.0;
    let card = Rect::new(
        (win.x - card_w) * 0.5,
        (win.y - card_h) * 0.5,
        card_w,
        card_h,
    );
    ui.panel(card);

    let pad = 26.0;
    let x = card.pos.x + pad;
    let w = card.size.x - pad * 2.0;
    let mut y = card.pos.y + 22.0;

    text::draw(
        ui.fonts,
        ui.tr("Справка", "Help"),
        Vec2::new(x, y),
        24.0,
        ui.palette.text_primary,
        true,
    );
    let close = Rect::new(x + w - 34.0, y - 2.0, 34.0, 30.0);
    ui.icon_button(close, Icon::Close, false, true, UiAction::ToggleHelp);
    y += 44.0;

    let sections: [(&str, &[(&str, &str)]); 3] = [
        (
            ui.tr("Мышь", "Mouse"),
            &[
                (
                    ui.tr("Клик по станции", "Click station"),
                    ui.tr("выбрать / карточка", "select / info"),
                ),
                (
                    ui.tr("Перетаскивание карты", "Drag map"),
                    ui.tr("панорама", "pan"),
                ),
                (
                    ui.tr("Колесо", "Wheel"),
                    ui.tr("приближение и отдаление", "zoom"),
                ),
            ],
        ),
        (
            ui.tr("Клавиатура", "Keyboard"),
            &[
                (
                    ui.tr("Пробел", "Space"),
                    ui.tr("пауза / продолжить", "pause / resume"),
                ),
                (ui.tr("1–5", "1–5"), ui.tr("скорость времени", "time speed")),
                (
                    ui.tr("F / C", "F / C"),
                    ui.tr("вписать / центрировать", "fit / center"),
                ),
                (ui.tr("+ / −", "+ / −"), ui.tr("зум", "zoom")),
                (
                    ui.tr("L / G / W", "L / G / W"),
                    ui.tr("подписи / сетка / пассажиры", "labels / grid / passengers"),
                ),
                (
                    ui.tr("H / Esc", "H / Esc"),
                    ui.tr("справка / меню", "help / menu"),
                ),
            ],
        ),
        (
            ui.tr("Редактор (карандаш)", "Editor (pencil)"),
            &[
                (
                    ui.tr("Выбор", "Select"),
                    ui.tr(
                        "клик, перетаскивание, переименование",
                        "click, drag, rename",
                    ),
                ),
                (
                    ui.tr("Станция", "Station"),
                    ui.tr(
                        "ставить; на линии станет пересадкой",
                        "place; on a line becomes a transfer",
                    ),
                ),
                (
                    ui.tr("Ветка", "Branch"),
                    ui.tr("вести маршрут по станциям", "draw a route by stations"),
                ),
                (
                    ui.tr("Удалить", "Erase"),
                    ui.tr("убрать станцию", "remove station"),
                ),
                (
                    ui.tr("Станция на станцию", "Station onto station"),
                    ui.tr("объединить их", "merge them"),
                ),
            ],
        ),
    ];

    for (title, rows) in sections {
        text::draw(
            ui.fonts,
            title,
            Vec2::new(x, y),
            14.0,
            ui.palette.accent_strong,
            true,
        );
        y += 24.0;
        for (key, desc) in rows {
            text::draw(
                ui.fonts,
                key,
                Vec2::new(x + 6.0, y),
                14.0,
                ui.palette.text_primary,
                true,
            );
            text::draw(
                ui.fonts,
                desc,
                Vec2::new(x + 200.0, y),
                13.0,
                ui.palette.text_secondary,
                false,
            );
            y += 22.0;
        }
        y += 8.0;
    }
}

pub fn draw_pause_indicator(ui: &mut UiContext, layout: &Layout, paused: bool) {
    if !paused {
        return;
    }
    let vp = layout.map_viewport;
    draw_rectangle(
        vp.pos.x,
        vp.pos.y,
        vp.size.x,
        vp.size.y,
        with_alpha(ui.palette.overlay, 0.16),
    );
    let label = ui.tr("Пауза", "Paused");
    let dims = text::measure(ui.fonts, label, 15.0, true);
    let w = dims.x + 66.0;
    let h = 36.0;
    let x = layout.window.x * 0.5 - w * 0.5;
    let y = vp.pos.y + 18.0;
    ui.note_region(Rect::new(x, y, w, h));
    shapes::fill_round_rect(x, y, w, h, 10.0, with_alpha(ui.palette.panel_raised, 0.95));
    shapes::stroke_round_rect(x, y, w, h, 10.0, 1.5, ui.palette.warning);
    icons::draw(
        ui.fonts,
        Icon::Pause,
        Rect::new(x + 12.0, y + h * 0.5 - 10.0, 20.0, 20.0),
        ui.palette.warning,
    );
    text::draw(
        ui.fonts,
        label,
        Vec2::new(x + 38.0, y + h * 0.5 - dims.y * 0.5),
        15.0,
        ui.palette.text_primary,
        true,
    );
}
