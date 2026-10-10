use crate::camera::Camera;
use crate::config::palette::Palette;
use crate::config::{Fonts, Settings};
use crate::geom::{Rect, Vec2};
use crate::network::{
    demo_network_lang, empty_network_lang, expand, generate, GenerateParams, LineId, Network,
    StationId, StationKind,
};
use crate::render::map::{self, MapContext};
use crate::render::RenderState;
use crate::save;
use crate::simulation::{SimEvent, Simulation};
use crate::ui::editor::{self, EditorView};
use crate::ui::hud::History;
use crate::ui::widgets::{Toast, ToastKind, UiContext};
use crate::ui::{hud, layout::Layout, menu, EditorTool, UiAction};
use macroquad::prelude::*;
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scene {
    MainMenu,
    Settings,
    NewSetup,
    Simulation,
    Analytics,
    About,
}

#[derive(Default)]
struct DragState {
    start: Option<Vec2>,
    last: Vec2,
    moved: bool,
    velocity: Vec2,
    grabbing: Option<StationId>,
}

const CONNECT_RADIUS: f32 = 46.0;
const SCROLL_BOTTOM_PAD: f32 = 28.0;

pub struct EditorState {
    pub tool: EditorTool,
    pub active_line: Option<LineId>,
    pub color_index: usize,
    undo: Vec<Network>,
    redo: Vec<Network>,
    next_color: usize,
}

const UNDO_LIMIT: usize = 50;

impl Default for EditorState {
    fn default() -> Self {
        EditorState {
            tool: EditorTool::Select,
            active_line: None,
            color_index: 0,
            undo: Vec::new(),
            redo: Vec::new(),
            next_color: 0,
        }
    }
}

impl EditorState {
    fn record(&mut self, network: &Network) {
        self.undo.push(network.clone());
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn pop_undo(&mut self, current: &Network) -> Option<Network> {
        let prev = self.undo.pop()?;
        self.redo.push(current.clone());
        Some(prev)
    }

    fn pop_redo(&mut self, current: &Network) -> Option<Network> {
        let next = self.redo.pop()?;
        self.undo.push(current.clone());
        Some(next)
    }

    fn next_color_index(&mut self) -> usize {
        let index = self.next_color;
        self.next_color = (self.next_color + 1) % crate::config::palette::LINE_COLORS.len();
        index
    }

    fn reset(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.active_line = None;
        self.tool = EditorTool::Select;
    }
}

pub struct App {
    pub palette: Palette,
    pub fonts: Fonts,
    pub settings: Settings,
    pub scene: Scene,
    settings_return: Scene,
    pub camera: Camera,
    pub network: Network,
    pub sim: Simulation,
    pub render_state: RenderState,
    pub selection: Option<StationId>,
    pub hover: Option<StationId>,
    pub toast: Option<Toast>,
    pub quit: bool,
    pub edit_mode: bool,
    pub editor: EditorState,
    pub history: History,
    congested: BTreeSet<StationId>,
    route_preview: Option<(StationId, StationId)>,
    help_open: bool,
    gen_params: GenerateParams,
    autosave_timer: f32,
    ui_scale: f32,
    scroll_ui: crate::ui::scroll::Scroll,
    content_h: f32,
    scroll_panel: crate::ui::scroll::Scroll,
    panel_content_h: f32,
    pinch_prev: f32,
    renaming: Option<(StationId, String)>,
    drag: DragState,
    session_in_game: bool,
    session_edit: bool,
    waste_timer: f32,
    slider_rects: Vec<Rect>,
    slider_grab: bool,
    press_station: Option<StationId>,
    press_timer: f32,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let mut settings = save::load_settings();
        settings.sanitize();
        let palette = Palette::for_theme(settings.theme);
        let fonts = Fonts::load();
        let network = demo_network_lang(settings.language);
        let sim = Simulation::new(&network);
        let mut camera = Camera::new(Rect::new(0.0, 0.0, 1200.0, 800.0));
        camera.fit(network.bounds());

        let mut app = App {
            palette,
            fonts,
            settings,
            scene: Scene::MainMenu,
            settings_return: Scene::MainMenu,
            camera,
            network,
            sim,
            render_state: RenderState::new(),
            selection: None,
            hover: None,
            toast: None,
            quit: false,
            edit_mode: false,
            editor: EditorState::default(),
            history: History::new(),
            congested: BTreeSet::new(),
            route_preview: None,
            help_open: false,
            gen_params: GenerateParams {
                seed: 1,
                stations: 12,
                lines: 3,
                hubs: 1,
            },
            autosave_timer: 0.0,
            ui_scale: 1.0,
            scroll_ui: crate::ui::scroll::Scroll::new(),
            content_h: 0.0,
            scroll_panel: crate::ui::scroll::Scroll::new(),
            panel_content_h: 0.0,
            pinch_prev: 0.0,
            renaming: None,
            drag: DragState::default(),
            session_in_game: false,
            session_edit: false,
            waste_timer: 45.0,
            slider_rects: Vec::new(),
            slider_grab: false,
            press_station: None,
            press_timer: 0.0,
        };
        app.restore_session();
        app
    }

    fn restore_session(&mut self) {
        let session = save::load_session();
        if !session.in_game {
            return;
        }
        let Ok(data) = save::load_quick() else {
            return;
        };
        self.network = data.network;
        self.sim = data.simulation;
        self.sim.rebuild_routing(&self.network);
        self.sim.apply_settings(&self.settings);
        self.sim.clock.scale = crate::config::settings::TIME_SCALES[self
            .settings
            .time_scale_index
            .min(crate::config::settings::TIME_SCALES.len() - 1)];
        self.camera.fit(self.network.bounds());
        self.edit_mode = session.edit_mode;
        self.scene = Scene::Simulation;
        self.session_in_game = true;
        self.session_edit = session.edit_mode;
    }

    pub fn notify(&mut self, text: impl Into<String>, kind: ToastKind) {
        self.toast = Some(Toast::new(text, kind));
    }

    fn t(&self, ru: &'static str, en: &'static str) -> &'static str {
        match self.settings.language {
            crate::config::i18n::Lang::En => en,
            crate::config::i18n::Lang::Ru => ru,
        }
    }

    fn autosave(&mut self) {
        if !self.settings.autosave || self.network.stations.is_empty() {
            return;
        }
        let _ = save::save_quick(&self.network, &self.sim);
    }

    fn start_generated(&mut self) {
        self.gen_params = self.gen_params.sanitized();
        self.network = generate(self.gen_params, self.settings.language);
        self.sim = Simulation::new(&self.network);
        self.sim.apply_settings(&self.settings);
        self.camera.fit(self.network.bounds());
        self.selection = None;
        self.hover = None;
        self.edit_mode = false;
        self.editor.reset();
        self.renaming = None;
        self.route_preview = None;
        self.help_open = false;
        self.history = History::new();
        self.congested.clear();
        self.scene = Scene::Simulation;
        self.notify(
            self.t("Сгенерирована новая карта", "Generated a new map"),
            ToastKind::Success,
        );
    }

    fn tf(&self, ru: &str, en: &str, args: &[(&str, &str)]) -> String {
        let mut out = match self.settings.language {
            crate::config::i18n::Lang::En => en.to_string(),
            crate::config::i18n::Lang::Ru => ru.to_string(),
        };
        for (key, value) in args {
            out = out.replace(&format!("{{{key}}}"), value);
        }
        out
    }

    pub fn frame(&mut self, dt: f32) {
        let scale = if self.settings.ui_scale > 0.0 {
            self.settings.ui_scale
        } else {
            crate::config::settings::auto_ui_scale()
        };
        self.ui_scale = scale.max(0.1);
        let physical_w = screen_width();
        let physical_h = screen_height();
        let design_w = (physical_w / self.ui_scale).max(1.0);
        let design_h = (physical_h / self.ui_scale).max(1.0);
        crate::render::scale::set(self.ui_scale, design_w, design_h);
        set_camera(&crate::render::scale::ui_camera());
        clear_background(self.palette.bg_top);
        let layout = Layout::compute(design_w, design_h);
        self.camera.viewport = layout.map_viewport;

        self.render_state.selection = self.selection;
        self.render_state.hover = self.hover;
        self.render_state.update(dt);

        let (pointer_x, pointer_y) = mouse_position();
        let pointer = Vec2::new(pointer_x / self.ui_scale, pointer_y / self.ui_scale);
        let pointer_pressed = is_mouse_button_pressed(MouseButton::Left);
        let pointer_released = is_mouse_button_released(MouseButton::Left);

        if matches!(
            self.scene,
            Scene::Settings | Scene::NewSetup | Scene::Analytics | Scene::About
        ) {
            let over_slider = self.slider_rects.iter().any(|r| r.contains(pointer));
            if pointer_pressed {
                self.slider_grab = over_slider;
                if !over_slider {
                    self.scroll_ui.start_drag(pointer.y);
                }
            }
            if pointer_released {
                self.slider_grab = false;
                self.scroll_ui.end_drag();
            }
            self.scroll_ui.drag_to(pointer.y, dt);
            let (_, wheel) = mouse_wheel();
            if wheel != 0.0 {
                self.scroll_ui.nudge(-wheel * 64.0);
            }
            self.scroll_ui.update(dt);
        } else if self.scene == Scene::Simulation {
            let in_panel = layout.side_panel.contains(pointer);
            if pointer_pressed && in_panel {
                self.scroll_panel.start_drag(pointer.y);
            }
            self.scroll_panel.drag_to(pointer.y, dt);
            if pointer_released {
                self.scroll_panel.end_drag();
            }
            self.scroll_panel.update(dt);
        }

        let mut actions = match self.scene {
            Scene::MainMenu => self.draw_main_menu(&layout),
            Scene::Settings => self.draw_settings(&layout),
            Scene::NewSetup => self.draw_new_setup(&layout),
            Scene::Simulation => self.draw_simulation(&layout, dt),
            Scene::Analytics => self.draw_analytics(&layout, dt),
            Scene::About => self.draw_about(&layout),
        };

        if self.renaming.is_some() {
            self.update_rename();
        } else if self.help_open {
            if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::H) {
                self.help_open = false;
            }
        } else {
            actions.extend(keyboard_actions(self.scene));
            if self.scene == Scene::Simulation && is_key_pressed(KeyCode::H) {
                self.help_open = true;
            }
        }
        self.process_actions(actions);

        match self.scene {
            Scene::Settings | Scene::NewSetup | Scene::Analytics | Scene::About => {
                let visible = (design_h - 64.0).max(1.0);
                self.scroll_ui.max = (self.content_h + SCROLL_BOTTOM_PAD - visible).max(0.0);
            }
            Scene::Simulation => {
                let visible = (layout.side_panel.size.y - 36.0).max(1.0);
                self.scroll_panel.max =
                    (self.panel_content_h + SCROLL_BOTTOM_PAD - visible).max(0.0);
            }
            Scene::MainMenu => {}
        }

        let in_game = matches!(self.scene, Scene::Simulation | Scene::Analytics);
        if in_game != self.session_in_game || (in_game && self.edit_mode != self.session_edit) {
            if in_game && !self.session_in_game {
                self.autosave();
            }
            self.session_in_game = in_game;
            self.session_edit = self.edit_mode;
            let _ = save::save_session(in_game, self.edit_mode);
        }

        if self.scene == Scene::Simulation && self.settings.autosave {
            self.autosave_timer += dt;
            if self.autosave_timer >= 20.0 {
                self.autosave_timer = 0.0;
                self.autosave();
            }
        }

        if let Some(toast) = &mut self.toast {
            if !toast.update(dt) {
                self.toast = None;
            }
        }
        if let Some(toast) = &self.toast {
            crate::ui::widgets::draw_toast(&self.fonts, &self.palette, toast, layout.window);
        }
        set_default_camera();

        #[cfg(target_os = "android")]
        crate::android::maintain_immersive();
    }

    fn draw_main_menu(&mut self, layout: &Layout) -> Vec<UiAction> {
        let mut ui = UiContext::new(
            &self.fonts,
            &self.palette,
            self.render_state.time,
            self.settings.language,
            self.ui_scale,
        );
        let has_save = save::quick_exists();
        menu::draw_main_menu(&mut ui, layout, has_save);
        ui.take_actions()
    }

    fn draw_settings(&mut self, layout: &Layout) -> Vec<UiAction> {
        let mut ui = UiContext::new(
            &self.fonts,
            &self.palette,
            self.render_state.time,
            self.settings.language,
            self.ui_scale,
        );
        ui.slider_grab = self.slider_grab;
        self.content_h =
            menu::draw_settings(&mut ui, layout, &self.settings, self.scroll_ui.offset);
        self.slider_rects = ui.slider_rects.clone();
        ui.take_actions()
    }

    fn draw_about(&mut self, layout: &Layout) -> Vec<UiAction> {
        let mut ui = UiContext::new(
            &self.fonts,
            &self.palette,
            self.render_state.time,
            self.settings.language,
            self.ui_scale,
        );
        ui.slider_grab = self.slider_grab;
        self.content_h = menu::draw_about(&mut ui, layout, self.scroll_ui.offset);
        self.slider_rects = ui.slider_rects.clone();
        ui.take_actions()
    }

    fn draw_new_setup(&mut self, layout: &Layout) -> Vec<UiAction> {
        let mut ui = UiContext::new(
            &self.fonts,
            &self.palette,
            self.render_state.time,
            self.settings.language,
            self.ui_scale,
        );
        self.content_h = menu::draw_new_setup(
            &mut ui,
            layout,
            &self.gen_params,
            &self.settings,
            self.scroll_ui.offset,
        );
        ui.take_actions()
    }

    fn draw_analytics(&mut self, layout: &Layout, dt: f32) -> Vec<UiAction> {
        self.advance_simulation(dt);
        let mut ui = UiContext::new(
            &self.fonts,
            &self.palette,
            self.render_state.time,
            self.settings.language,
            self.ui_scale,
        );
        self.content_h = crate::ui::analytics::draw(
            &mut ui,
            &self.network,
            &self.sim,
            &self.history,
            self.scroll_ui.offset,
            layout,
        );
        ui.take_actions()
    }

    fn advance_simulation(&mut self, dt: f32) {
        self.sim.apply_settings(&self.settings);
        self.sim.update(&self.network, dt);

        let events: Vec<SimEvent> = std::mem::take(&mut self.sim.events);
        let reduced = self.settings.reduced_motion;
        for event in events {
            let SimEvent::Arrive { station, line, .. } = event;
            if let Some(st) = self.network.station(station) {
                let pos = st.pos;
                let color = self
                    .network
                    .line(line)
                    .map(|l| line_color_from_rgb(l.color))
                    .unwrap_or(self.palette.accent);
                self.render_state.push_pulse(pos, color, reduced);
            }
        }

        if !self.sim.clock.paused {
            let avg_wait = self
                .sim
                .passengers
                .network_average_wait(self.sim.now())
                .unwrap_or(0.0);
            self.history.sample(
                dt,
                self.sim.total_waiting() as f32,
                self.sim.total_onboard() as f32,
                avg_wait,
                self.sim.passengers.total_delivered as f32,
            );
        }
        self.update_congestion();
        self.update_line_load(dt);

        self.waste_timer -= dt;
        let baseline_cars: u32 = self
            .network
            .lines
            .iter()
            .map(|l| {
                let dirs = if l.is_loop { 1 } else { 2 };
                self.settings.cars_min.max(1) * self.settings.trains_min.max(1) * dirs
            })
            .sum();
        let total_cars: u32 = self.sim.trains.iter().map(|t| t.cars).sum();
        if self.waste_timer <= 0.0 {
            self.waste_timer = 60.0;
            if total_cars > baseline_cars && self.sim.average_load() < 0.18 {
                self.notify(
                    self.t(
                        "Полупустые поезда: издержки растут",
                        "Trains run half-empty: costs rising",
                    ),
                    ToastKind::Error,
                );
            }
        }
    }

    fn draw_simulation(&mut self, layout: &Layout, dt: f32) -> Vec<UiAction> {
        self.advance_simulation(dt);

        let reduced = self.settings.reduced_motion;
        self.camera.update(dt, reduced);
        self.camera.clamp_to(self.network.bounds());

        let route = self
            .route_preview
            .and_then(|(from, to)| self.sim.plan(&self.network, from, to));

        let (mx, my) = mouse_position();
        let (mx, my) = (mx / self.ui_scale, my / self.ui_scale);
        {
            let ctx = MapContext {
                network: &self.network,
                sim: &self.sim,
                camera: &self.camera,
                palette: &self.palette,
                fonts: &self.fonts,
                settings: &self.settings,
                state: &self.render_state,
                active_line: if self.edit_mode {
                    self.editor.active_line
                } else {
                    None
                },
                daylight: self.sim.clock.daylight(),
                route: route.as_ref(),
                cursor: Vec2::new(mx, my),
            };
            map::draw(&ctx);
        }

        let actions;
        let pointer_over_ui;
        {
            let (trains_now, cars_now) =
                match self.editor.active_line.and_then(|id| self.network.line(id)) {
                    Some(l) => (
                        l.trains_override.unwrap_or(self.settings.trains_min.max(1)),
                        l.cars_override.unwrap_or(self.settings.cars_min.max(1)),
                    ),
                    None => (
                        self.settings.trains_min.max(1),
                        self.settings.cars_min.max(1),
                    ),
                };
            let view = EditorView {
                tool: self.editor.tool,
                active_line: self.editor.active_line,
                color_index: self.editor.color_index,
                auto_trains: self.settings.auto_trains,
                auto_cars: self.settings.auto_cars,
                trains_now,
                cars_now,
            };
            let mut ui = UiContext::new(
                &self.fonts,
                &self.palette,
                self.render_state.time,
                self.settings.language,
                self.ui_scale,
            );
            if self.edit_mode {
                let rename_buffer = self
                    .renaming
                    .as_ref()
                    .filter(|(id, _)| self.selection == Some(*id))
                    .map(|(_, buffer)| buffer.as_str());
                let active_line_name = self
                    .editor
                    .active_line
                    .and_then(|id| self.network.line(id))
                    .map(|l| l.name.clone());
                editor::draw_toolbar(
                    &mut ui,
                    layout,
                    self.editor.tool,
                    active_line_name.as_deref(),
                );
                self.panel_content_h = editor::draw_panel(
                    &mut ui,
                    &self.network,
                    &view,
                    self.selection,
                    rename_buffer,
                    self.scroll_panel.offset,
                    layout,
                );
            } else {
                self.panel_content_h = hud::draw_side_panel(
                    &mut ui,
                    &self.network,
                    &self.sim,
                    self.selection,
                    &self.history,
                    self.route_preview,
                    self.scroll_panel.offset,
                    layout,
                );
            }
            hud::draw_top_bar(
                &mut ui,
                &self.network,
                &self.sim,
                &self.settings,
                layout,
                self.edit_mode,
                self.congested.len(),
            );
            if self.settings.controls_hidden {
                hud::draw_controls_collapsed(&mut ui, layout);
            } else {
                hud::draw_controls(&mut ui, &self.settings, layout);
            }
            hud::draw_pause_indicator(&mut ui, layout, self.sim.clock.paused);
            if self.help_open {
                hud::draw_help_overlay(&mut ui, layout);
                ui.pointer_over_ui = true;
            }
            pointer_over_ui = ui.pointer_over_ui;
            actions = ui.take_actions();
        }

        self.handle_pointer(layout, pointer_over_ui);
        actions
    }

    fn handle_pointer(&mut self, layout: &Layout, pointer_over_ui: bool) {
        let (mx, my) = mouse_position();
        let mouse = Vec2::new(mx / self.ui_scale, my / self.ui_scale);
        let over_map = layout.map_viewport.contains(mouse) && !pointer_over_ui;

        let pressed = is_mouse_button_pressed(MouseButton::Left);
        let down = is_mouse_button_down(MouseButton::Left);
        let released = is_mouse_button_released(MouseButton::Left);

        let active_touches = touches();
        if active_touches.len() >= 2 {
            let a = active_touches[0].position / self.ui_scale;
            let b = active_touches[1].position / self.ui_scale;
            let dist = (a - b).length();
            let mid = (a + b) * 0.5;
            if self.pinch_prev > 0.0 && dist > 0.0 {
                self.camera
                    .zoom_by(dist / self.pinch_prev, Vec2::new(mid.x, mid.y));
            }
            self.pinch_prev = dist;
            self.drag.start = None;
            return;
        }
        self.pinch_prev = 0.0;

        let (_, wheel_y) = mouse_wheel();
        if over_map && wheel_y.abs() > 0.0 {
            let factor = if wheel_y > 0.0 { 1.12 } else { 1.0 / 1.12 };
            self.camera.zoom_by(factor, mouse);
        }

        let tolerance = (6.0 / self.camera.zoom).clamp(4.0, 14.0);
        self.hover = if over_map {
            map::station_at_screen(&self.network, &self.camera, mouse, tolerance)
        } else {
            None
        };

        if pressed && over_map {
            self.drag.start = Some(mouse);
            self.drag.last = mouse;
            self.drag.moved = false;
            self.drag.velocity = Vec2::ZERO;
            self.camera.momentum = Vec2::ZERO;
            if self.edit_mode {
                if self.editor.tool == EditorTool::Select {
                    self.selection = self.hover;
                    self.drag.grabbing = self.hover;
                }
            } else {
                self.selection = self.hover;
                self.press_station = self.hover;
                self.press_timer = 0.0;
            }
        }

        // Long-press a station while playing to jump into the editor and drag it.
        if !down {
            self.press_station = None;
            self.press_timer = 0.0;
        } else if !self.edit_mode && !self.drag.moved && self.press_station.is_some() {
            self.press_timer += get_frame_time();
            if self.press_timer >= 0.5 {
                if let Some(st) = self.press_station.take() {
                    self.edit_mode = true;
                    self.editor.tool = EditorTool::Select;
                    self.editor.active_line = None;
                    self.scroll_panel.reset();
                    self.sim.clock.paused = true;
                    self.selection = Some(st);
                    self.drag.grabbing = Some(st);
                    self.drag.moved = true;
                    self.editor.record(&self.network);
                    self.notify(
                        self.t(
                            "Редактор включён: тяните станцию",
                            "Editor on: drag the station",
                        ),
                        ToastKind::Success,
                    );
                }
            }
        }

        if down {
            if let Some(id) = self.drag.grabbing {
                if !self.drag.moved {
                    self.editor.record(&self.network);
                    self.drag.moved = true;
                }
                let world = self.camera.world_from_screen(mouse);
                let pos = snap_to_grid(world);
                if let Some(station) = self.network.station_mut(id) {
                    station.pos = pos;
                }
            } else if let Some(start) = self.drag.start {
                let delta = mouse - self.drag.last;
                if (mouse - start).length() > 6.0 {
                    self.drag.moved = true;
                }
                if self.drag.moved && delta.length() > 0.0 {
                    self.camera.pan_by_screen(delta);
                }
                self.drag.velocity = delta;
                self.drag.last = mouse;
            }
        }

        if released {
            let was_grabbing = self.drag.grabbing.is_some();
            if let Some(grabbed) = self.drag.grabbing.take() {
                let merge_tolerance = (14.0 / self.camera.zoom).clamp(8.0, 30.0);
                let mut target = None;
                let mut best = merge_tolerance;
                for station in &self.network.stations {
                    if station.id == grabbed {
                        continue;
                    }
                    let d = self.camera.world_to_screen(station.pos).distance(mouse);
                    if d < best {
                        best = d;
                        target = Some(station.id);
                    }
                }
                if let Some(target) = target {
                    self.network.merge_stations(grabbed, target);
                    self.sim.remap_station(grabbed, target);
                    self.sim.reconcile(&self.network);
                    self.selection = Some(target);
                    self.hover = Some(target);
                    self.notify(
                        self.t("Станции объединены", "Stations merged"),
                        ToastKind::Success,
                    );
                } else if self.edit_mode {
                    let connected = self
                        .network
                        .connect_station_to_nearby_lines(grabbed, CONNECT_RADIUS);
                    if !connected.is_empty() {
                        self.sim.reconcile(&self.network);
                        self.notify(
                            self.t("Станция подключена к линии", "Station connected to a line"),
                            ToastKind::Success,
                        );
                    }
                }
            } else if self.edit_mode && self.drag.start.is_some() && !self.drag.moved && over_map {
                let hit = map::station_at_screen(&self.network, &self.camera, mouse, tolerance);
                let world = self.camera.world_from_screen(mouse);
                self.handle_editor_click(hit, world);
            }
            if !was_grabbing && self.drag.moved {
                let mut velocity = self.drag.velocity * 60.0;
                let len = velocity.length();
                if len > 2600.0 {
                    velocity = velocity / len * 2600.0;
                }
                self.camera.momentum = velocity;
            }
            self.drag.start = None;
            self.drag.moved = false;
        }
    }

    fn handle_editor_click(&mut self, hit: Option<StationId>, world: Vec2) {
        match self.editor.tool {
            EditorTool::Select => {
                self.selection = hit;
            }
            EditorTool::Station => {
                if hit.is_none() {
                    self.editor.record(&self.network);
                    let pos = snap_to_grid(world);
                    let name = self.tf(
                        "Станция {n}",
                        "Station {n}",
                        &[("n", &(self.network.stations.len() + 1).to_string())],
                    );
                    let id = self.network.add_station(name, pos, StationKind::Regular);
                    let connected = self
                        .network
                        .connect_station_to_nearby_lines(id, CONNECT_RADIUS);
                    self.selection = Some(id);
                    self.sim.reconcile(&self.network);
                    if connected.is_empty() {
                        self.notify(
                            self.t("Станция добавлена", "Station added"),
                            ToastKind::Success,
                        );
                    } else {
                        self.notify(
                            self.tf(
                                "Станция добавлена и соединена с линиями: {n}",
                                "Station added and connected to {n} line(s)",
                                &[("n", &connected.len().to_string())],
                            ),
                            ToastKind::Success,
                        );
                    }
                } else {
                    self.selection = hit;
                }
            }
            EditorTool::Link => {
                let Some(station) = hit else {
                    self.notify(
                        self.t(
                            "Инструмент «Ветка»: кликните по станции, чтобы добавить её в линию",
                            "Branch tool: click a station to add it to the line",
                        ),
                        ToastKind::Error,
                    );
                    return;
                };
                let station_name = self
                    .network
                    .station(station)
                    .map(|s| s.name.clone())
                    .unwrap_or_default();
                if let Some(line) = self.editor.active_line {
                    if self
                        .network
                        .line(line)
                        .map(|l| l.stations.last() == Some(&station))
                        .unwrap_or(false)
                    {
                        self.notify(
                            self.tf(
                                "«{s}» уже в конце линии",
                                "“{s}” is already at the line end",
                                &[("s", &station_name)],
                            ),
                            ToastKind::Error,
                        );
                        return;
                    }
                }
                self.editor.record(&self.network);
                let (line, created) = match self.editor.active_line {
                    Some(line) => (line, false),
                    None => {
                        let index = self.editor.color_index;
                        let name = self.tf(
                            "Линия {n}",
                            "Line {n}",
                            &[("n", &(self.network.lines.len() + 1).to_string())],
                        );
                        let line = self.network.add_line(name, index);
                        self.editor.active_line = Some(line);
                        (line, true)
                    }
                };
                let line_name = self
                    .network
                    .line(line)
                    .map(|l| l.name.clone())
                    .unwrap_or_default();
                let _ = self.network.append_station(line, station);
                self.sim.reconcile(&self.network);
                if created {
                    self.notify(
                        self.tf(
                            "Создана линия «{l}», добавлена «{s}»",
                            "Created line “{l}”, added “{s}”",
                            &[("l", &line_name), ("s", &station_name)],
                        ),
                        ToastKind::Success,
                    );
                } else {
                    self.notify(
                        self.tf(
                            "«{s}» добавлена в «{l}»",
                            "“{s}” added to “{l}”",
                            &[("s", &station_name), ("l", &line_name)],
                        ),
                        ToastKind::Success,
                    );
                }
            }
            EditorTool::Erase => {
                if let Some(station) = hit {
                    let station_name = self
                        .network
                        .station(station)
                        .map(|s| s.name.clone())
                        .unwrap_or_default();
                    self.editor.record(&self.network);
                    self.network.remove_station(station);
                    if self.selection == Some(station) {
                        self.selection = None;
                    }
                    self.sim.reconcile(&self.network);
                    self.notify(
                        self.tf(
                            "Станция «{s}» удалена",
                            "Station “{s}” deleted",
                            &[("s", &station_name)],
                        ),
                        ToastKind::Success,
                    );
                } else {
                    self.notify(
                        self.t(
                            "Кликните по станции, чтобы удалить её",
                            "Click a station to delete it",
                        ),
                        ToastKind::Error,
                    );
                }
            }
        }
    }

    fn update_rename(&mut self) {
        let Some(id) = self.renaming.as_ref().map(|(id, _)| *id) else {
            return;
        };
        let chars: Vec<char> = std::iter::from_fn(get_char_pressed).collect();
        let backspace = is_key_pressed(KeyCode::Backspace);
        let enter = is_key_pressed(KeyCode::Enter);
        let escape = is_key_pressed(KeyCode::Escape);

        if let Some((_, buffer)) = self.renaming.as_mut() {
            for ch in chars {
                if !ch.is_control() && buffer.chars().count() < 28 {
                    buffer.push(ch);
                }
            }
            if backspace {
                buffer.pop();
            }
        }

        if escape {
            self.renaming = None;
            return;
        }
        if enter {
            let name = self
                .renaming
                .as_ref()
                .map(|(_, buffer)| buffer.trim().to_string())
                .unwrap_or_default();
            if !name.is_empty() {
                self.editor.record(&self.network);
                if let Some(station) = self.network.station_mut(id) {
                    station.name = name;
                }
            }
            self.renaming = None;
        }
    }

    fn update_line_load(&mut self, dt: f32) {
        let mut current: std::collections::HashMap<LineId, f32> = std::collections::HashMap::new();
        for train in &self.sim.trains {
            *current.entry(train.line).or_insert(0.0) += train.onboard_count() as f32;
        }
        let k = (dt * 3.0).clamp(0.0, 1.0);
        for line in &self.network.lines {
            let cur = current.get(&line.id).copied().unwrap_or(0.0);
            let entry = self.render_state.line_load.entry(line.id).or_insert(cur);
            *entry += (cur - *entry) * k;
        }
    }

    fn update_congestion(&mut self) {
        let current: BTreeSet<StationId> = self
            .network
            .stations
            .iter()
            .filter(|s| self.sim.passengers.waiting_at(s.id) >= crate::render::map::ALERT_THRESHOLD)
            .map(|s| s.id)
            .collect();
        let newly: Vec<StationId> = current.difference(&self.congested).copied().collect();
        if let Some(id) = newly.first() {
            let name = self
                .network
                .station(*id)
                .map(|s| s.name.clone())
                .unwrap_or_default();
            let extra = if newly.len() > 1 {
                self.tf(
                    " и ещё {n}",
                    " and {n} more",
                    &[("n", &(newly.len() - 1).to_string())],
                )
            } else {
                String::new()
            };
            self.notify(
                self.tf(
                    "Перегрузка на станции «{name}»{extra}",
                    "Congestion at “{name}”{extra}",
                    &[("name", &name), ("extra", &extra)],
                ),
                ToastKind::Error,
            );
        }
        self.congested = current;
    }

    fn process_actions(&mut self, actions: Vec<UiAction>) {
        for action in actions {
            match action {
                UiAction::NewGame => {
                    self.network = demo_network_lang(self.settings.language);
                    self.sim = Simulation::new(&self.network);
                    self.sim.apply_settings(&self.settings);
                    self.camera.fit(self.network.bounds());
                    self.selection = None;
                    self.hover = None;
                    self.edit_mode = false;
                    self.editor.reset();
                    self.renaming = None;
                    self.route_preview = None;
                    self.help_open = false;
                    self.history = History::new();
                    self.congested.clear();
                    self.scene = Scene::Simulation;
                    self.notify(
                        self.t("Создана новая симуляция", "New simulation created"),
                        ToastKind::Success,
                    );
                }
                UiAction::NewEmpty => {
                    self.network = empty_network_lang(self.settings.language);
                    self.sim = Simulation::new(&self.network);
                    self.sim.apply_settings(&self.settings);
                    self.camera.fit(self.network.bounds());
                    self.selection = None;
                    self.hover = None;
                    self.edit_mode = true;
                    self.editor.reset();
                    self.renaming = None;
                    self.route_preview = None;
                    self.help_open = false;
                    self.history = History::new();
                    self.congested.clear();
                    self.editor.tool = EditorTool::Station;
                    self.sim.clock.paused = true;
                    self.scene = Scene::Simulation;
                    self.notify(
                        self.t(
                            "Пустая карта: ставьте станции инструментом «Станция»",
                            "Empty map: place stations with the Station tool",
                        ),
                        ToastKind::Success,
                    );
                }
                UiAction::LoadGame => match save::load_quick() {
                    Ok(data) => {
                        self.network = data.network;
                        self.sim = data.simulation;
                        self.sim.rebuild_routing(&self.network);
                        self.sim.apply_settings(&self.settings);
                        self.sim.clock.scale = crate::config::settings::TIME_SCALES[self
                            .settings
                            .time_scale_index
                            .min(crate::config::settings::TIME_SCALES.len() - 1)];
                        self.camera.fit(self.network.bounds());
                        self.selection = None;
                        self.hover = None;
                        self.edit_mode = false;
                        self.editor.reset();
                        self.renaming = None;
                        self.route_preview = None;
                        self.help_open = false;
                        self.history = History::new();
                        self.congested.clear();
                        self.scene = Scene::Simulation;
                        self.notify(
                            self.t("Сохранение загружено", "Save loaded"),
                            ToastKind::Success,
                        );
                    }
                    Err(err) => {
                        self.notify(
                            self.tf(
                                "Не удалось загрузить: {e}",
                                "Load failed: {e}",
                                &[("e", &err.localized(self.settings.language))],
                            ),
                            ToastKind::Error,
                        );
                    }
                },
                UiAction::SaveGame => match save::save_quick(&self.network, &self.sim) {
                    Ok(_) => {
                        self.notify(self.t("Сохранено", "Saved"), ToastKind::Success);
                    }
                    Err(err) => {
                        self.notify(
                            self.tf(
                                "Ошибка сохранения: {e}",
                                "Save failed: {e}",
                                &[("e", &err.localized(self.settings.language))],
                            ),
                            ToastKind::Error,
                        );
                    }
                },
                UiAction::OpenMenu => {
                    self.autosave();
                    self.renaming = None;
                    self.route_preview = None;
                    self.scene = Scene::MainMenu;
                }
                UiAction::OpenSettings => {
                    self.autosave();
                    self.scroll_ui.reset();
                    self.settings_return = self.scene;
                    self.scene = Scene::Settings;
                }
                UiAction::CloseSettings => {
                    let _ = save::save_settings(&self.settings);
                    self.scene = self.settings_return;
                }
                UiAction::Quit => {
                    self.autosave();
                    let _ = save::save_settings(&self.settings);
                    let _ = save::save_session(false, false);
                    #[cfg(target_os = "android")]
                    crate::android::quit_app();
                    #[cfg(not(target_os = "android"))]
                    {
                        self.quit = true;
                    }
                }
                UiAction::TogglePause => {
                    self.sim.clock.paused = !self.sim.clock.paused;
                }
                UiAction::SetScale(index) => {
                    self.settings.time_scale_index =
                        index.min(crate::config::settings::TIME_SCALES.len() - 1);
                    self.sim.clock.scale =
                        crate::config::settings::TIME_SCALES[self.settings.time_scale_index];
                }
                UiAction::ZoomIn => self.camera.zoom_by(1.15, self.camera.viewport.center()),
                UiAction::ZoomOut => self
                    .camera
                    .zoom_by(1.0 / 1.15, self.camera.viewport.center()),
                UiAction::FitView => self.camera.fit(self.network.bounds()),
                UiAction::CenterNetwork => self.camera.center(),
                UiAction::ToggleLabels => {
                    self.settings.show_labels = !self.settings.show_labels;
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::ToggleWaiting => {
                    self.settings.show_waiting = !self.settings.show_waiting;
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::ToggleGrid => {
                    self.settings.show_grid = !self.settings.show_grid;
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::ToggleMotion => {
                    self.settings.reduced_motion = !self.settings.reduced_motion;
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::ToggleHeat => {
                    self.settings.show_heat = !self.settings.show_heat;
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::ToggleFlow => {
                    self.settings.show_flow = !self.settings.show_flow;
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::ToggleHelp => {
                    self.help_open = !self.help_open;
                }
                UiAction::ToggleControls => {
                    self.settings.controls_hidden = !self.settings.controls_hidden;
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::ToggleAutoCars => {
                    self.settings.auto_cars = !self.settings.auto_cars;
                    self.settings.sanitize();
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::SetFlow(index) => {
                    let all = crate::config::settings::FlowLevel::ALL;
                    self.settings.flow_level = all[index.min(all.len() - 1)];
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::SetLanguage(index) => {
                    let all = crate::config::i18n::Lang::ALL;
                    self.settings.language = all[index.min(all.len() - 1)];
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::SetTheme(index) => {
                    let all = crate::config::palette::Theme::ALL;
                    self.settings.theme = all[index.min(all.len() - 1)];
                    self.palette = Palette::for_theme(self.settings.theme);
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::OpenSetup => {
                    self.scroll_ui.reset();
                    self.scene = Scene::NewSetup;
                }
                UiAction::CloseSetup => {
                    self.scene = Scene::MainMenu;
                }
                UiAction::OpenAbout => {
                    self.scroll_ui.reset();
                    self.scene = Scene::About;
                }
                UiAction::CloseAbout => {
                    self.scene = Scene::MainMenu;
                }
                UiAction::OpenLink(url) => {
                    crate::platform::open_url(&url);
                    self.notify(
                        self.t("Открываю ссылку…", "Opening link…"),
                        ToastKind::Success,
                    );
                }
                UiAction::AdjustStations(delta) => {
                    let v = self.gen_params.stations as i32 + delta;
                    self.gen_params.stations = v.clamp(4, 40) as u32;
                    self.gen_params = self.gen_params.sanitized();
                }
                UiAction::AdjustLines(delta) => {
                    let v = self.gen_params.lines as i32 + delta;
                    self.gen_params.lines = v.clamp(1, 8) as u32;
                    self.gen_params = self.gen_params.sanitized();
                }
                UiAction::AdjustHubs(delta) => {
                    let v = self.gen_params.hubs as i32 + delta;
                    self.gen_params.hubs = v.clamp(0, 40) as u32;
                    self.gen_params = self.gen_params.sanitized();
                }
                UiAction::AdjustTrainsMin(delta) => {
                    let v = self.settings.trains_min as i32 + delta;
                    self.settings.trains_min = v.clamp(1, 6) as u32;
                    self.settings.sanitize();
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::AdjustTrainsMax(delta) => {
                    let v = self.settings.trains_max as i32 + delta;
                    self.settings.trains_max = v.clamp(1, 8) as u32;
                    self.settings.sanitize();
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::ToggleAutoTrains => {
                    self.settings.auto_trains = !self.settings.auto_trains;
                    self.settings.sanitize();
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::ToggleAutosave => {
                    self.settings.autosave = !self.settings.autosave;
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::AdjustFlowGrowth(delta) => {
                    let percent = (self.settings.flow_growth * 100.0).round() as i32 + delta;
                    self.settings.flow_growth = percent.clamp(0, 100) as f32 / 100.0;
                    self.settings.sanitize();
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::AdjustUiScale(delta) => {
                    let current = if self.settings.ui_scale > 0.0 {
                        self.settings.ui_scale
                    } else {
                        crate::config::settings::auto_ui_scale()
                    };
                    let percent = (current * 100.0).round() as i32 + delta;
                    self.settings.ui_scale = percent.clamp(60, 300) as f32 / 100.0;
                    self.settings.sanitize();
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::SetUiScale(v) => {
                    self.settings.ui_scale = v.clamp(0.8, 3.0);
                    self.settings.sanitize();
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::SetCarCapacity(v) => {
                    self.settings.car_capacity = v.clamp(10, 120);
                    self.settings.sanitize();
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::SetCarsMin(v) => {
                    self.settings.cars_min = v.clamp(1, 6);
                    self.settings.sanitize();
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::SetCarsMax(v) => {
                    self.settings.cars_max = v.clamp(1, 8);
                    self.settings.sanitize();
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::SetTrainsMin(v) => {
                    self.settings.trains_min = v.clamp(1, 6);
                    self.settings.sanitize();
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::SetTrainsMax(v) => {
                    self.settings.trains_max = v.clamp(1, 8);
                    self.settings.sanitize();
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::SetFlowGrowth(v) => {
                    self.settings.flow_growth = v.clamp(0.0, 1.0);
                    self.settings.sanitize();
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::ExpandNetwork => {
                    let seed = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos() as u64)
                        .unwrap_or(1);
                    self.editor.record(&self.network);
                    if expand(&mut self.network, seed, 3, self.settings.language) {
                        self.sim.reconcile(&self.network);
                        self.camera.clamp_to(self.network.bounds());
                        self.notify(
                            self.t(
                                "Сеть расширена: добавлена новая линия",
                                "Network expanded: a new line added",
                            ),
                            ToastKind::Success,
                        );
                    }
                }
                UiAction::GenerateRandom => {
                    let seed = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos() as u64)
                        .unwrap_or(1);
                    self.gen_params = GenerateParams::random(seed);
                    self.start_generated();
                }
                UiAction::GenerateMap => {
                    self.start_generated();
                }
                UiAction::AdjustCarCapacity(delta) => {
                    let v = self.settings.car_capacity as i32 + delta;
                    self.settings.car_capacity = v.clamp(10, 120) as u32;
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::AdjustCarsMin(delta) => {
                    let v = self.settings.cars_min as i32 + delta;
                    self.settings.cars_min = v.clamp(1, 6) as u32;
                    self.settings.sanitize();
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::AdjustCarsMax(delta) => {
                    let v = self.settings.cars_max as i32 + delta;
                    self.settings.cars_max = v.clamp(1, 8) as u32;
                    self.settings.sanitize();
                    let _ = save::save_settings(&self.settings);
                }
                UiAction::OpenAnalytics => {
                    self.scene = Scene::Analytics;
                }
                UiAction::CloseAnalytics => {
                    self.scene = Scene::Simulation;
                }
                UiAction::FocusStation(id) => {
                    if let Some(station) = self.network.station(id) {
                        self.camera.target = station.pos;
                        self.camera.target_zoom = self.camera.zoom.max(1.2);
                    }
                    self.selection = Some(id);
                }
                UiAction::PanTo(world) => {
                    self.camera.target = world;
                    self.camera.clamp_to(self.network.bounds());
                }
                UiAction::ToggleEdit => {
                    self.edit_mode = !self.edit_mode;
                    self.renaming = None;
                    self.route_preview = None;
                    self.scroll_panel.reset();
                    if self.edit_mode {
                        self.sim.clock.paused = true;
                        self.editor.tool = EditorTool::Select;
                        self.editor.active_line = None;
                        self.notify(
                            self.t("Режим редактора: включён", "Editor mode: on"),
                            ToastKind::Success,
                        );
                    } else {
                        self.notify(
                            self.t("Режим редактора: выключен", "Editor mode: off"),
                            ToastKind::Success,
                        );
                    }
                }
                UiAction::SetTool(tool) => {
                    self.editor.tool = tool;
                }
                UiAction::NewLine => {
                    self.editor.record(&self.network);
                    let index = self.editor.next_color_index();
                    self.editor.color_index = index;
                    let name = self.tf(
                        "Линия {n}",
                        "Line {n}",
                        &[("n", &(self.network.lines.len() + 1).to_string())],
                    );
                    let line = self.network.add_line(name.clone(), index);
                    self.editor.active_line = Some(line);
                    self.editor.tool = EditorTool::Link;
                    self.sim.reconcile(&self.network);
                    self.notify(
                        self.tf(
                            "Создана «{n}». Кликайте станции, чтобы вести маршрут.",
                            "Created “{n}”. Click stations to draw the route.",
                            &[("n", &name)],
                        ),
                        ToastKind::Success,
                    );
                }
                UiAction::SelectLine(line) => {
                    self.editor.active_line = Some(line);
                    self.editor.color_index = self
                        .network
                        .line(line)
                        .map(|l| color_index_of(l.color))
                        .unwrap_or(self.editor.color_index);
                }
                UiAction::DeleteLine(line) => {
                    let name = self
                        .network
                        .line(line)
                        .map(|l| l.name.clone())
                        .unwrap_or_default();
                    self.editor.record(&self.network);
                    self.network.remove_line(line);
                    if self.editor.active_line == Some(line) {
                        self.editor.active_line = None;
                    }
                    self.sim.reconcile(&self.network);
                    self.notify(
                        self.tf("Линия «{n}» удалена", "Line “{n}” deleted", &[("n", &name)]),
                        ToastKind::Success,
                    );
                }
                UiAction::RemoveLineLastStation(line) => {
                    self.editor.record(&self.network);
                    let removed = self.network.pop_station(line);
                    self.sim.reconcile(&self.network);
                    if removed {
                        self.notify(
                            self.t(
                                "Последняя станция убрана из линии",
                                "Last station removed from the line",
                            ),
                            ToastKind::Success,
                        );
                    }
                }
                UiAction::CycleLineColor => {
                    if let Some(line) = self.editor.active_line {
                        self.editor.record(&self.network);
                        self.editor.color_index = self.editor.next_color_index();
                        let rgb = rgb_of_color_index(self.editor.color_index);
                        if let Some(l) = self.network.line_mut(line) {
                            l.color = rgb;
                        }
                    }
                }
                UiAction::SetLineColor(line_id, index) => {
                    self.editor.record(&self.network);
                    self.editor.color_index = index;
                    let rgb = rgb_of_color_index(index);
                    if let Some(l) = self.network.line_mut(line_id) {
                        l.color = rgb;
                    }
                }
                UiAction::AdjustLineTrains(line_id, delta) => {
                    let min = self.settings.trains_min.max(1);
                    if let Some(l) = self.network.line_mut(line_id) {
                        let cur = l.trains_override.unwrap_or(min) as i32;
                        l.trains_override = Some((cur + delta).clamp(1, 8) as u32);
                    }
                    self.sim.balance_trains(&self.network);
                }
                UiAction::AdjustLineCars(line_id, delta) => {
                    let min = self.settings.cars_min.max(1);
                    let max = self.settings.cars_max.max(min) as i32;
                    if let Some(l) = self.network.line_mut(line_id) {
                        let cur = l.cars_override.unwrap_or(min) as i32;
                        l.cars_override = Some((cur + delta).clamp(1, max) as u32);
                    }
                    self.sim.balance_trains(&self.network);
                }
                UiAction::CycleLineStyle => {
                    if let Some(line) = self.editor.active_line {
                        self.editor.record(&self.network);
                        if let Some(l) = self.network.line_mut(line) {
                            l.style =
                                crate::config::palette::line_style(style_index_of(l.style) + 1);
                        }
                    }
                }
                UiAction::DeleteStation(station) => {
                    self.editor.record(&self.network);
                    self.network.remove_station(station);
                    if self.selection == Some(station) {
                        self.selection = None;
                    }
                    self.sim.reconcile(&self.network);
                }
                UiAction::DetachFromLine(line, station) => {
                    self.editor.record(&self.network);
                    self.network.detach_station(line, station);
                    self.sim.reconcile(&self.network);
                }
                UiAction::StartRename(station) => {
                    if let Some(st) = self.network.station(station) {
                        self.selection = Some(station);
                        self.renaming = Some((station, st.name.clone()));
                    }
                }
                UiAction::PreviewRoute(from, to) => {
                    if self.route_preview == Some((from, to)) {
                        self.route_preview = None;
                    } else {
                        self.route_preview = Some((from, to));
                    }
                }
                UiAction::EditorUndo => {
                    if let Some(prev) = self.editor.pop_undo(&self.network) {
                        self.network = prev;
                        self.sim.reconcile(&self.network);
                        self.clamp_editor_refs();
                    }
                }
                UiAction::EditorRedo => {
                    if let Some(next) = self.editor.pop_redo(&self.network) {
                        self.network = next;
                        self.sim.reconcile(&self.network);
                        self.clamp_editor_refs();
                    }
                }
            }
        }
    }

    fn clamp_editor_refs(&mut self) {
        if let Some(line) = self.editor.active_line {
            if self.network.line(line).is_none() {
                self.editor.active_line = None;
            }
        }
        if let Some(station) = self.selection {
            if self.network.station(station).is_none() {
                self.selection = None;
            }
        }
        self.camera.clamp_to(self.network.bounds());
    }
}

fn snap_to_grid(v: Vec2) -> Vec2 {
    const STEP: f32 = 20.0;
    Vec2::new((v.x / STEP).round() * STEP, (v.y / STEP).round() * STEP)
}

fn rgb_of_color_index(index: usize) -> [u8; 3] {
    let c = crate::config::palette::LINE_COLORS[index % crate::config::palette::LINE_COLORS.len()];
    [
        (c.r * 255.0) as u8,
        (c.g * 255.0) as u8,
        (c.b * 255.0) as u8,
    ]
}

fn color_index_of(rgb: [u8; 3]) -> usize {
    crate::config::palette::LINE_COLORS
        .iter()
        .position(|c| {
            (c.r * 255.0) as u8 == rgb[0]
                && (c.g * 255.0) as u8 == rgb[1]
                && (c.b * 255.0) as u8 == rgb[2]
        })
        .unwrap_or(0)
}

fn style_index_of(style: crate::config::palette::LineStyle) -> usize {
    use crate::config::palette::LineStyle::*;
    match style {
        Solid => 0,
        Dashed => 1,
        Dotted => 2,
        DashDot => 3,
    }
}

fn line_color_from_rgb(rgb: [u8; 3]) -> Color {
    Color::from_rgba(rgb[0], rgb[1], rgb[2], 255)
}

fn keyboard_actions(scene: Scene) -> Vec<UiAction> {
    let mut actions = Vec::new();
    if scene == Scene::Settings {
        if is_key_pressed(KeyCode::Escape) {
            actions.push(UiAction::CloseSettings);
        }
        return actions;
    }
    if scene == Scene::Analytics {
        if is_key_pressed(KeyCode::Escape) {
            actions.push(UiAction::CloseAnalytics);
        }
        return actions;
    }
    if scene == Scene::NewSetup {
        if is_key_pressed(KeyCode::Escape) {
            actions.push(UiAction::CloseSetup);
        }
        return actions;
    }
    if is_key_pressed(KeyCode::Escape) {
        actions.push(match scene {
            Scene::Simulation => UiAction::OpenMenu,
            _ => UiAction::Quit,
        });
    }
    if scene != Scene::Simulation {
        if is_key_pressed(KeyCode::Enter) {
            actions.push(UiAction::OpenSetup);
        }
        return actions;
    }
    if is_key_pressed(KeyCode::Space) {
        actions.push(UiAction::TogglePause);
    }
    for (key, index) in [
        (KeyCode::Key1, 0),
        (KeyCode::Key2, 1),
        (KeyCode::Key3, 2),
        (KeyCode::Key4, 3),
        (KeyCode::Key5, 4),
    ] {
        if is_key_pressed(key) {
            actions.push(UiAction::SetScale(index));
        }
    }
    if is_key_pressed(KeyCode::F) {
        actions.push(UiAction::FitView);
    }
    if is_key_pressed(KeyCode::C) {
        actions.push(UiAction::CenterNetwork);
    }
    if is_key_pressed(KeyCode::L) {
        actions.push(UiAction::ToggleLabels);
    }
    if is_key_pressed(KeyCode::G) {
        actions.push(UiAction::ToggleGrid);
    }
    if is_key_pressed(KeyCode::W) {
        actions.push(UiAction::ToggleWaiting);
    }
    if is_key_pressed(KeyCode::Equal) {
        actions.push(UiAction::ZoomIn);
    }
    if is_key_pressed(KeyCode::Minus) {
        actions.push(UiAction::ZoomOut);
    }
    actions
}
