use crate::geom::{Rect, Vec2};
use crate::network::Network;
use crate::render::{shapes, text};
use crate::simulation::Simulation;
use crate::ui::hud::{draw_chart, format_duration, History};
use crate::ui::icons::{self, Icon};
use crate::ui::layout::Layout;
use crate::ui::widgets::UiContext;
use crate::ui::UiAction;
use macroquad::prelude::*;

pub fn draw(
    ui: &mut UiContext,
    network: &Network,
    sim: &Simulation,
    history: &History,
    scroll: f32,
    layout: &Layout,
) -> f32 {
    let win = layout.window;
    draw_rectangle(0.0, 0.0, win.x, win.y, ui.palette.bg_top);

    let margin = 26.0;
    let x = margin;
    let w = win.x - margin * 2.0;

    let content_h = if win.y < 660.0 {
        draw_compact(ui, network, sim, history, x, w, win.y);
        win.y - 140.0
    } else {
        let mut y = 84.0 - scroll;
        draw_kpis(ui, network, sim, history, x, w, y);
        y += 194.0;
        draw_charts(ui, history, x, w, y, 190.0);
        y += 206.0;
        let gap = 16.0;
        let lines_w = w * 0.56;
        let stations_w = w - lines_w - gap;
        let table_h = 66.0 + 30.0 * network.lines.len().min(12) as f32;
        let bus_h = 66.0 + 30.0 * sim.busiest_stations(10).len() as f32;
        draw_lines_table(ui, network, sim, Rect::new(x, y, lines_w, table_h));
        draw_busiest(
            ui,
            network,
            sim,
            Rect::new(x + lines_w + gap, y, stations_w, bus_h),
        );
        y += table_h.max(bus_h) + 16.0;
        y - (84.0 - scroll)
    };

    draw_rectangle(0.0, 0.0, win.x, 64.0, ui.palette.panel);
    draw_header(ui, sim, x, w, 20.0);
    draw_line(0.0, 64.0, win.x, 64.0, 1.5, ui.palette.border);
    content_h
}

#[allow(clippy::too_many_arguments)]
fn draw_compact(
    ui: &mut UiContext,
    network: &Network,
    sim: &Simulation,
    history: &History,
    x: f32,
    w: f32,
    h: f32,
) {
    let gap = 12.0;
    let top = 74.0;
    let kpi_h = 50.0;
    draw_kpi_strip(ui, network, sim, history, Rect::new(x, top, w, kpi_h));
    let body_y = top + kpi_h + gap;
    let body_h = (h - body_y - 12.0).max(90.0);
    let left_w = w * 0.54;
    let right_w = w - left_w - gap;
    draw_charts(ui, history, x, left_w, body_y, body_h);

    // Fixed-height cards (content based) so they never stretch/jump around.
    let lines_content = 50.0 + network.lines.len().min(12) as f32 * 30.0 + 14.0;
    let bus_h = 184.0;
    let lines_h = lines_content.min((body_h - gap - bus_h).max(80.0));
    let rx = x + left_w + gap;
    draw_lines_table(ui, network, sim, Rect::new(rx, body_y, right_w, lines_h));
    draw_busiest(
        ui,
        network,
        sim,
        Rect::new(rx, body_y + lines_h + gap, right_w, bus_h),
    );
}

fn draw_kpi_strip(
    ui: &mut UiContext,
    network: &Network,
    sim: &Simulation,
    history: &History,
    rect: Rect,
) {
    let congested = network
        .stations
        .iter()
        .filter(|s| sim.waiting_at(s.id) >= crate::render::map::ALERT_THRESHOLD)
        .count();
    let throughput = history.throughput.back().copied().unwrap_or(0.0);
    let cells = [
        (
            ui.tr("Доставлено", "Delivered"),
            sim.passengers.total_delivered.to_string(),
            ui.palette.success,
        ),
        (
            ui.tr("Ожидают", "Waiting"),
            sim.total_waiting().to_string(),
            ui.palette.station_waiting,
        ),
        (
            ui.tr("В пути", "Onboard"),
            sim.total_onboard().to_string(),
            ui.palette.accent,
        ),
        (
            ui.tr("Ср. ожидание", "Avg wait"),
            sim.passengers
                .network_average_wait(sim.now())
                .map(|s| format_duration(ui.lang, s))
                .unwrap_or_else(|| "—".to_string()),
            ui.palette.warning,
        ),
        (
            ui.tr("Загрузка", "Load"),
            format!("{:.0}%", sim.average_load() * 100.0),
            ui.palette.danger,
        ),
        (
            ui.tr("Перевезено/с", "Throughput"),
            format!("{throughput:.1}"),
            ui.palette.success,
        ),
        (
            ui.tr("Поездов", "Trains"),
            sim.trains.len().to_string(),
            ui.palette.text_primary,
        ),
        (
            ui.tr("Станций", "Stations"),
            network.stations.len().to_string(),
            ui.palette.text_primary,
        ),
        (
            ui.tr("Перегружено", "Congested"),
            congested.to_string(),
            ui.palette.danger,
        ),
        (
            ui.tr("Издержки", "Cost"),
            format!("{:.0}", sim.operational_cost),
            ui.palette.warning,
        ),
    ];
    let gap = 8.0;
    let n = cells.len() as f32;
    let cw = (rect.size.x - gap * (n - 1.0)) / n;
    for (i, (label, value, color)) in cells.iter().enumerate() {
        let r = Rect::new(
            rect.pos.x + i as f32 * (cw + gap),
            rect.pos.y,
            cw,
            rect.size.y,
        );
        shapes::fill_round_rect(r.pos.x, r.pos.y, r.size.x, r.size.y, 8.0, ui.palette.panel);
        shapes::stroke_round_rect(
            r.pos.x,
            r.pos.y,
            r.size.x,
            r.size.y,
            8.0,
            1.2,
            ui.palette.border,
        );
        shapes::fill_round_rect(r.pos.x, r.pos.y, 3.0, r.size.y, 1.5, *color);
        text::draw(
            ui.fonts,
            label,
            Vec2::new(r.pos.x + 9.0, r.pos.y + 7.0),
            10.0,
            ui.palette.text_muted,
            false,
        );
        text::draw(
            ui.fonts,
            value,
            Vec2::new(r.pos.x + 9.0, r.pos.y + 22.0),
            19.0,
            ui.palette.text_primary,
            true,
        );
    }
}

fn draw_header(ui: &mut UiContext, sim: &Simulation, x: f32, w: f32, y: f32) {
    text::draw(
        ui.fonts,
        ui.tr("Аналитика", "Analytics"),
        Vec2::new(x, y),
        28.0,
        ui.palette.text_primary,
        true,
    );
    let day_label = ui.tf(
        "День {d}",
        "Day {d}",
        &[("d", &sim.clock.day().to_string())],
    );
    text::draw(
        ui.fonts,
        &format!("{} · {}", day_label, sim.clock.format_time()),
        Vec2::new(x + 176.0, y + 9.0),
        14.0,
        ui.palette.text_muted,
        false,
    );

    let close = Rect::new(x + w - 40.0, y, 40.0, 34.0);
    ui.icon_button(close, Icon::Close, false, true, UiAction::CloseAnalytics);
}

fn draw_kpis(
    ui: &mut UiContext,
    network: &Network,
    sim: &Simulation,
    history: &History,
    x: f32,
    w: f32,
    y: f32,
) {
    let congested = network
        .stations
        .iter()
        .filter(|s| sim.waiting_at(s.id) >= crate::render::map::ALERT_THRESHOLD)
        .count();
    let throughput = history.throughput.back().copied().unwrap_or(0.0);
    let row1 = [
        (
            ui.tr("Доставлено", "Delivered"),
            sim.passengers.total_delivered.to_string(),
            ui.palette.success,
        ),
        (
            ui.tr("Ожидают", "Waiting"),
            sim.total_waiting().to_string(),
            ui.palette.station_waiting,
        ),
        (
            ui.tr("В пути", "Onboard"),
            sim.total_onboard().to_string(),
            ui.palette.accent,
        ),
        (
            ui.tr("Среднее ожидание", "Average wait"),
            sim.passengers
                .network_average_wait(sim.now())
                .map(|s| format_duration(ui.lang, s))
                .unwrap_or_else(|| "—".to_string()),
            ui.palette.warning,
        ),
        (
            ui.tr("Пик загрузки", "Peak load"),
            format!("{:.0}%", sim.peak_load() * 100.0),
            ui.palette.danger,
        ),
    ];
    let row2 = [
        (
            ui.tr("Средняя загрузка", "Average load"),
            format!("{:.0}%", sim.average_load() * 100.0),
            ui.palette.accent,
        ),
        (
            ui.tr("Пропускная способность", "Throughput"),
            format!("{throughput:.1}{}", ui.tr("/с", "/s")),
            ui.palette.success,
        ),
        (
            ui.tr("Поездов", "Trains"),
            sim.trains.len().to_string(),
            ui.palette.text_primary,
        ),
        (
            ui.tr("Станций", "Stations"),
            network.stations.len().to_string(),
            ui.palette.text_primary,
        ),
        (
            ui.tr("Перегружено", "Congested"),
            congested.to_string(),
            ui.palette.danger,
        ),
        (
            ui.tr("Издержки", "Cost"),
            format!("{:.0}", sim.operational_cost),
            ui.palette.warning,
        ),
    ];
    let gap = 14.0;
    for (row, dy) in [(row1.as_slice(), 0.0f32), (row2.as_slice(), 92.0)] {
        let n = row.len() as f32;
        let cw = (w - gap * (n - 1.0)) / n;
        for (i, (label, value, color)) in row.iter().enumerate() {
            let rect = Rect::new(x + i as f32 * (cw + gap), y + dy, cw, 80.0);
            shapes::fill_round_rect(
                rect.pos.x,
                rect.pos.y,
                rect.size.x,
                rect.size.y,
                10.0,
                ui.palette.panel,
            );
            shapes::stroke_round_rect(
                rect.pos.x,
                rect.pos.y,
                rect.size.x,
                rect.size.y,
                10.0,
                1.5,
                ui.palette.border,
            );
            shapes::fill_round_rect(rect.pos.x, rect.pos.y, 4.0, rect.size.y, 2.0, *color);
            text::draw(
                ui.fonts,
                label,
                Vec2::new(rect.pos.x + 16.0, rect.pos.y + 14.0),
                12.0,
                ui.palette.text_muted,
                false,
            );
            text::draw(
                ui.fonts,
                value,
                Vec2::new(rect.pos.x + 16.0, rect.pos.y + 38.0),
                24.0,
                ui.palette.text_primary,
                true,
            );
        }
    }
}

fn draw_charts(ui: &mut UiContext, history: &History, x: f32, w: f32, y: f32, h: f32) {
    let gap = 16.0;
    let cw = (w - gap * 2.0) / 3.0;
    draw_chart(
        ui,
        Rect::new(x, y, cw, h),
        ui.tr("Очереди и пассажиры в пути", "Queues & passengers"),
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
    draw_chart(
        ui,
        Rect::new(x + cw + gap, y, cw, h),
        ui.tr("Среднее ожидание (секунды)", "Average wait (seconds)"),
        &[(
            &history.avg_wait,
            ui.palette.warning,
            ui.tr("Ожидание", "Waiting"),
        )],
        true,
        None,
    );
    draw_chart(
        ui,
        Rect::new(x + (cw + gap) * 2.0, y, cw, h),
        ui.tr("Пропускная способность", "Throughput"),
        &[(
            &history.throughput,
            ui.palette.success,
            ui.tr("Доставка", "Delivery"),
        )],
        true,
        None,
    );
}

fn section(ui: &mut UiContext, rect: Rect, title: &str) -> (f32, f32, f32) {
    shapes::fill_round_rect(
        rect.pos.x,
        rect.pos.y,
        rect.size.x,
        rect.size.y,
        12.0,
        ui.palette.panel,
    );
    shapes::stroke_round_rect(
        rect.pos.x,
        rect.pos.y,
        rect.size.x,
        rect.size.y,
        12.0,
        1.5,
        ui.palette.border,
    );
    let pad = 16.0;
    let x = rect.pos.x + pad;
    let w = rect.size.x - pad * 2.0;
    text::draw(
        ui.fonts,
        title,
        Vec2::new(x, rect.pos.y + 16.0),
        16.0,
        ui.palette.text_primary,
        true,
    );
    (x, w, rect.pos.y + 48.0)
}

fn draw_lines_table(ui: &mut UiContext, network: &Network, sim: &Simulation, rect: Rect) {
    let (x, w, mut y) = section(ui, rect, ui.tr("Линии", "Lines"));
    header_cells(
        ui,
        x,
        w,
        y,
        &[
            (0.0, ui.tr("Линия", "Line")),
            (0.42, ui.tr("Поезда", "Trains")),
            (0.6, ui.tr("Перевезено", "Carried")),
            (0.82, ui.tr("Загрузка", "Load")),
        ],
    );
    y += 20.0;

    let mut lines: Vec<_> = network.lines.iter().collect();
    lines.sort_by_key(|l| std::cmp::Reverse(sim.line_boarded(l.id)));
    for line in lines {
        if y + 26.0 > rect.bottom() {
            break;
        }
        let color = Color::from_rgba(line.color[0], line.color[1], line.color[2], 255);
        icons::line_chip(Rect::new(x, y + 6.0, 22.0, 8.0), color, line.style);
        text::draw(
            ui.fonts,
            &line.name,
            Vec2::new(x + 32.0, y),
            15.0,
            ui.palette.text_primary,
            false,
        );
        text::draw(
            ui.fonts,
            &ui.tf(
                "{n} ст.",
                "{n} st.",
                &[("n", &line.stations.len().to_string())],
            ),
            Vec2::new(
                x + 32.0 + text::measure(ui.fonts, &line.name, 15.0, false).x + 8.0,
                y + 2.0,
            ),
            11.0,
            ui.palette.text_muted,
            false,
        );
        let trains = sim.trains.iter().filter(|t| t.line == line.id).count();
        text::draw(
            ui.fonts,
            &trains.to_string(),
            Vec2::new(x + w * 0.42, y),
            15.0,
            ui.palette.text_secondary,
            true,
        );
        text::draw(
            ui.fonts,
            &sim.line_boarded(line.id).to_string(),
            Vec2::new(x + w * 0.6, y),
            15.0,
            ui.palette.text_primary,
            true,
        );

        let bar = Rect::new(x + w * 0.82, y + 4.0, w * 0.18 - 4.0, 9.0);
        shapes::fill_round_rect(
            bar.pos.x,
            bar.pos.y,
            bar.size.x,
            bar.size.y,
            4.5,
            ui.palette.panel_raised,
        );
        let load = sim.line_load_now(line.id).clamp(0.0, 1.0);
        if load > 0.01 {
            shapes::fill_round_rect(
                bar.pos.x,
                bar.pos.y,
                bar.size.x * load,
                bar.size.y,
                4.5,
                color,
            );
        }
        y += 30.0;
    }
}

fn draw_busiest(ui: &mut UiContext, network: &Network, sim: &Simulation, rect: Rect) {
    let (x, w, y) = section(ui, rect, ui.tr("Загруженные станции", "Busiest stations"));
    let busiest = sim.busiest_stations(8);
    if busiest.is_empty() {
        text::draw(
            ui.fonts,
            ui.tr("Очередей нет", "No queues"),
            Vec2::new(x, y),
            13.0,
            ui.palette.text_muted,
            false,
        );
        return;
    }
    let col_gap = 14.0;
    let col_w = (w - col_gap) * 0.5;
    let per_col = busiest.len().div_ceil(2);
    let row_h = 30.0;
    for (i, (id, waiting)) in busiest.iter().enumerate() {
        let col = i / per_col;
        let row = i % per_col;
        let cx = x + col as f32 * (col_w + col_gap);
        let ry = y + row as f32 * row_h;
        if ry + row_h > rect.bottom() + 4.0 {
            continue;
        }
        let Some(station) = network.station(*id) else {
            continue;
        };
        let rowr = Rect::new(cx - 6.0, ry - 3.0, col_w + 4.0, row_h - 2.0);
        let clicked = ui.list_row(rowr, false);
        draw_circle(cx + 4.0, ry + 8.0, 4.0, ui.palette.accent);
        let name = elide(ui, &station.name, col_w - 56.0, 13.0);
        text::draw(
            ui.fonts,
            &name,
            Vec2::new(cx + 15.0, ry),
            13.0,
            ui.palette.text_primary,
            false,
        );
        text::draw_right(
            ui.fonts,
            &waiting.to_string(),
            Vec2::new(cx + col_w - 4.0, ry),
            14.0,
            alert_color(ui, *waiting),
            true,
        );
        if clicked {
            ui.emit(UiAction::FocusStation(*id));
            ui.emit(UiAction::CloseAnalytics);
        }
    }
}

fn elide(ui: &UiContext, s: &str, max_w: f32, size: f32) -> String {
    if max_w <= 0.0 || text::measure(ui.fonts, s, size, false).x <= max_w {
        return s.to_string();
    }
    let mut out = String::new();
    for ch in s.chars() {
        let cand: String = format!("{out}{ch}");
        if text::measure(ui.fonts, &cand, size, false).x > max_w - 10.0 {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

fn header_cells(ui: &mut UiContext, x: f32, w: f32, y: f32, cells: &[(f32, &str)]) {
    for (fx, label) in cells {
        text::draw(
            ui.fonts,
            label,
            Vec2::new(x + w * fx, y),
            12.0,
            ui.palette.text_muted,
            false,
        );
    }
}

fn alert_color(ui: &UiContext, waiting: u32) -> Color {
    let threshold = crate::render::map::ALERT_THRESHOLD as f32;
    let heat = (waiting as f32 / threshold).clamp(0.0, 1.0);
    crate::config::palette::mix(ui.palette.text_primary, ui.palette.danger, heat)
}
