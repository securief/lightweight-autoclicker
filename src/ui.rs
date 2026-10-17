use std::fs;
use std::sync::{atomic::Ordering, Arc, Mutex};

use eframe::egui;
use global_hotkey::GlobalHotKeyManager;

use crate::{
    config::AppConfig,
    hotkeys::{self, rebind_hotkey, short_label, wake_unfocused, HotkeySlot},
    memory::{trim_memory, BG_THREAD_STACK},
    picker::Picker,
    state::SharedState,
    texts::Texts,
};

pub struct LightweightAutoclickerApp {
    cfg: AppConfig,
    state: Arc<SharedState>,
    gui_ctx: Arc<Mutex<Option<egui::Context>>>,
    _hotkey_manager: GlobalHotKeyManager,
    hotkey_slot: HotkeySlot,
    show_hotkey_modal: bool,
    hk_recording: bool,
    was_picking: bool,
    frames_rendered: u8,
    initial_memory_trimmed: bool,
    was_focused: bool,
    applied_top: bool,
    applied_theme: bool,
    centered_once: bool,
    fit_stage: u8,
    measured_h: f32,
}

#[cfg(target_os = "windows")]
const FONT_CANDIDATES: &[&str] = &[
    "C:\\Windows\\Fonts\\segoeui.ttf",
    "C:\\Windows\\Fonts\\arial.ttf",
];
#[cfg(target_os = "linux")]
const FONT_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/usr/share/fonts/TTF/DejaVuSans.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
    "/usr/share/fonts/opentype/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
    "/usr/share/fonts/gnu-free/FreeSans.ttf",
];
#[cfg(target_os = "macos")]
const FONT_CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/SFNS.ttf",
    "/System/Library/Fonts/Supplemental/Arial.ttf",
];
#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
const FONT_CANDIDATES: &[&str] = &[];

fn load_system_font(ctx: &egui::Context) {
    let data = FONT_CANDIDATES
        .iter()
        .find_map(|p| crate::memory::map_file_readonly(p).map(egui::FontData::from_static))
        .or_else(|| {
            FONT_CANDIDATES
                .iter()
                .find_map(|p| fs::read(p).ok().map(egui::FontData::from_owned))
        });
    let Some(data) = data else {
        return;
    };
    let mut fonts = egui::FontDefinitions::empty();
    fonts.font_data.insert("system".to_owned(), data);
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "system".to_owned());
    }
    ctx.set_fonts(fonts);
}

fn combo_arrow(
    ui: &egui::Ui,
    rect: egui::Rect,
    visuals: &egui::style::WidgetVisuals,
    _is_open: bool,
    _above_or_below: egui::AboveOrBelow,
) {
    let r = egui::Rect::from_center_size(
        rect.center(),
        egui::vec2(rect.width() * 0.7, rect.height() * 0.45),
    );
    ui.painter().add(egui::Shape::convex_polygon(
        vec![r.left_top(), r.right_top(), r.center_bottom()],
        visuals.fg_stroke.color,
        egui::Stroke::NONE,
    ));
}

fn key_to_name(key: egui::Key) -> Option<&'static str> {
    Some(match key {
        egui::Key::F1 => "F1",
        egui::Key::F2 => "F2",
        egui::Key::F3 => "F3",
        egui::Key::F4 => "F4",
        egui::Key::F5 => "F5",
        egui::Key::F6 => "F6",
        egui::Key::F7 => "F7",
        egui::Key::F8 => "F8",
        egui::Key::F9 => "F9",
        egui::Key::F10 => "F10",
        egui::Key::F11 => "F11",
        egui::Key::F12 => "F12",
        egui::Key::A => "A",
        egui::Key::B => "B",
        egui::Key::C => "C",
        egui::Key::D => "D",
        egui::Key::E => "E",
        egui::Key::F => "F",
        egui::Key::G => "G",
        egui::Key::H => "H",
        egui::Key::I => "I",
        egui::Key::J => "J",
        egui::Key::K => "K",
        egui::Key::L => "L",
        egui::Key::M => "M",
        egui::Key::N => "N",
        egui::Key::O => "O",
        egui::Key::P => "P",
        egui::Key::Q => "Q",
        egui::Key::R => "R",
        egui::Key::S => "S",
        egui::Key::T => "T",
        egui::Key::U => "U",
        egui::Key::V => "V",
        egui::Key::W => "W",
        egui::Key::X => "X",
        egui::Key::Y => "Y",
        egui::Key::Z => "Z",
        egui::Key::Num0 => "0",
        egui::Key::Num1 => "1",
        egui::Key::Num2 => "2",
        egui::Key::Num3 => "3",
        egui::Key::Num4 => "4",
        egui::Key::Num5 => "5",
        egui::Key::Num6 => "6",
        egui::Key::Num7 => "7",
        egui::Key::Num8 => "8",
        egui::Key::Num9 => "9",
        egui::Key::Space => "SPACE",
        egui::Key::Tab => "TAB",
        _ => return None,
    })
}

impl LightweightAutoclickerApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        state: Arc<SharedState>,
        cfg: AppConfig,
        gui_ctx: Arc<Mutex<Option<egui::Context>>>,
    ) -> Self {
        if let Ok(mut g) = gui_ctx.lock() {
            *g = Some(cc.egui_ctx.clone());
        }
        load_system_font(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(if cfg.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });
        let manager = GlobalHotKeyManager::new().expect("hotkey manager init");
        let hotkey_slot = hotkeys::register_hotkeys(
            &manager,
            Arc::clone(&state),
            cc.egui_ctx.clone(),
            &cfg.hotkey,
        );
        let mut style = (*cc.egui_ctx.style()).clone();
        style.animation_time = 0.0;
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(6.0);
        style.visuals.widgets.inactive.rounding = egui::Rounding::same(6.0);
        style.visuals.widgets.hovered.rounding = egui::Rounding::same(6.0);
        style.visuals.widgets.active.rounding = egui::Rounding::same(6.0);
        style.visuals.widgets.open.rounding = egui::Rounding::same(6.0);
        cc.egui_ctx.set_style(style);
        Self {
            cfg,
            state,
            gui_ctx,
            _hotkey_manager: manager,
            hotkey_slot,
            show_hotkey_modal: false,
            hk_recording: false,
            was_picking: false,
            frames_rendered: 0,
            initial_memory_trimmed: false,
            was_focused: true,
            applied_top: false,
            applied_theme: false,
            centered_once: false,
            fit_stage: 0,
            measured_h: 0.0,
        }
    }

    fn start_picking_location(&mut self) {
        if self.state.is_running() {
            return;
        }
        if self.state.is_picking_location.swap(true, Ordering::SeqCst) {
            return;
        }
        self.cfg.use_custom_pos = true;
        self.state.use_custom_pos.store(true, Ordering::Relaxed);

        let state = Arc::clone(&self.state);
        let ctxs = Arc::clone(&self.gui_ctx);
        let _ = std::thread::Builder::new()
            .name("pickloc".into())
            .stack_size(BG_THREAD_STACK)
            .spawn(move || {
                let picker = Picker::new();
                std::thread::sleep(std::time::Duration::from_millis(250));
                loop {
                    let (x, y, left_down) = picker.sample();
                    state.pos_x.store(x, Ordering::Relaxed);
                    state.pos_y.store(y, Ordering::Relaxed);
                    if let Ok(g) = ctxs.lock() {
                        if let Some(ctx) = g.as_ref() {
                            wake_unfocused(ctx);
                        }
                    }
                    if left_down {
                        state.is_picking_location.store(false, Ordering::SeqCst);
                        if let Ok(g) = ctxs.lock() {
                            if let Some(ctx) = g.as_ref() {
                                wake_unfocused(ctx);
                            }
                        }
                        trim_memory();
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(16));
                }
            });
    }

    fn apply_top(&mut self, ctx: &egui::Context) {
        let level = if self.cfg.always_on_top {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        ctx.set_visuals(if self.cfg.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });
    }

    fn maybe_autosave(&self) {
        if self.cfg.autosave {
            self.cfg.save();
        }
    }
}

impl eframe::App for LightweightAutoclickerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.initial_memory_trimmed {
            self.frames_rendered = self.frames_rendered.saturating_add(1);
            if self.fit_stage == 2 && self.frames_rendered >= 8 {
                self.initial_memory_trimmed = true;
                trim_memory();
            }
        }
        let focused = ctx.input(|i| i.focused);
        if self.was_focused && !focused && self.initial_memory_trimmed {
            trim_memory();
        }
        self.was_focused = focused;
        if !self.applied_top {
            self.applied_top = true;
            self.apply_top(ctx);
        }
        if !self.applied_theme {
            self.applied_theme = true;
            self.apply_theme(ctx);
        }
        if self.initial_memory_trimmed {
            ctx.input(|i| {
                let v = i.viewport();
                if v.minimized.unwrap_or(false) {
                    return;
                }
                if let Some(rect) = v.outer_rect {
                    let (x, y) = (rect.min.x, rect.min.y);
                    if x.is_finite() && y.is_finite() && x > -1000.0 && y > -1000.0 {
                        self.cfg.window_x = Some(x);
                        self.cfg.window_y = Some(y);
                    }
                }
            });
        }

        let is_running = self.state.is_running.load(Ordering::SeqCst);
        let is_picking = self.state.is_picking_location.load(Ordering::Relaxed);

        let is_id = self.cfg.lang == 1;
        let mut changed = false;

        if is_picking || (self.was_picking && !is_picking) {
            self.cfg.pos_x = self.state.pos_x.load(Ordering::Relaxed);
            self.cfg.pos_y = self.state.pos_y.load(Ordering::Relaxed);
            changed = !is_picking;
        }
        self.was_picking = is_picking;

        let t = Texts::for_lang(self.cfg.lang);
        let t_pick = if is_picking { "..." } else { t.pick };
        let hk = short_label(&self.hotkey_slot.name);

        if self.fit_stage == 0 {
            self.fit_stage = 1;
            ctx.request_repaint();
        }
        let mut panel_frame = egui::Frame::central_panel(&ctx.style());
        panel_frame.inner_margin.bottom = 0.0;
        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let mut top_dirty = false;
                    if ui.checkbox(&mut self.cfg.autosave, t.autosave).changed() {
                        self.cfg.save();
                        changed = true;
                    }
                    if ui
                        .checkbox(&mut self.cfg.always_on_top, t.always_on_top)
                        .changed()
                    {
                        top_dirty = true;
                    }
                    let mut dark = self.cfg.dark_mode;
                    if ui.checkbox(&mut dark, t.dark).changed() {
                        self.cfg.dark_mode = dark;
                        self.apply_theme(ctx);
                        changed = true;
                    }
                    if top_dirty {
                        self.apply_top(ctx);
                        changed = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        egui::ComboBox::from_id_salt("lang_combo")
                            .selected_text(if is_id { "Indonesia" } else { "English" })
                            .width(88.0)
                            .icon(combo_arrow)
                            .show_ui(ui, |ui| {
                                changed |= ui
                                    .selectable_value(&mut self.cfg.lang, 0, "English")
                                    .changed();
                                changed |= ui
                                    .selectable_value(&mut self.cfg.lang, 1, "Indonesia")
                                    .changed();
                            });
                        if ui
                            .button(egui::RichText::new(hk).strong().monospace())
                            .clicked()
                        {
                            self.show_hotkey_modal = true;
                            self.hk_recording = false;
                        }
                    });
                });
                ui.add_space(4.0);

                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(t.interval).strong());
                        changed |= ui
                            .add(egui::DragValue::new(&mut self.cfg.hours).range(0..=999))
                            .changed();
                        ui.label(t.unit_h);
                        changed |= ui
                            .add(egui::DragValue::new(&mut self.cfg.minutes).range(0..=59))
                            .changed();
                        ui.label(t.unit_m);
                        changed |= ui
                            .add(egui::DragValue::new(&mut self.cfg.seconds).range(0..=59))
                            .changed();
                        ui.label(t.unit_s);
                        changed |= ui
                            .add(egui::DragValue::new(&mut self.cfg.milliseconds).range(0..=999))
                            .changed();
                        ui.label(t.unit_ms);
                    });
                });
                ui.add_space(4.0);

                ui.columns(2, |cols| {
                    cols[0].group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.label(egui::RichText::new(t.repeat).strong());
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            ui.spacing_mut().interact_size.x = 26.0;
                            let custom = !self.cfg.repeat_infinite;
                            changed |= ui
                                .radio_value(&mut self.cfg.repeat_infinite, true, t.infinite)
                                .changed();
                            changed |= ui
                                .radio_value(&mut self.cfg.repeat_infinite, false, "")
                                .changed();
                            changed |= ui
                                .add_enabled(
                                    custom,
                                    egui::DragValue::new(&mut self.cfg.repeat_times)
                                        .range(1..=999_999),
                                )
                                .changed();
                            ui.add_enabled(custom, egui::Label::new(egui::RichText::new(t.times)));
                        });
                    });
                    cols[1].group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.label(egui::RichText::new(t.button_type).strong());
                        egui::Grid::new("options_grid")
                            .num_columns(2)
                            .spacing([8.0, 6.0])
                            .show(ui, |ui| {
                                let btn_text = match self.cfg.mouse_button {
                                    1 => t.right,
                                    2 => t.middle,
                                    _ => t.left,
                                };
                                egui::ComboBox::from_id_salt("mouse_btn_combo")
                                    .selected_text(btn_text)
                                    .width(74.0)
                                    .icon(combo_arrow)
                                    .show_ui(ui, |ui| {
                                        changed |= ui
                                            .selectable_value(&mut self.cfg.mouse_button, 0, t.left)
                                            .changed();
                                        changed |= ui
                                            .selectable_value(
                                                &mut self.cfg.mouse_button,
                                                1,
                                                t.right,
                                            )
                                            .changed();
                                        changed |= ui
                                            .selectable_value(
                                                &mut self.cfg.mouse_button,
                                                2,
                                                t.middle,
                                            )
                                            .changed();
                                    });
                                let type_text = if self.cfg.click_type == 1 {
                                    t.double
                                } else {
                                    t.single
                                };
                                egui::ComboBox::from_id_salt("click_type_combo")
                                    .selected_text(type_text)
                                    .width(74.0)
                                    .icon(combo_arrow)
                                    .show_ui(ui, |ui| {
                                        changed |= ui
                                            .selectable_value(&mut self.cfg.click_type, 0, t.single)
                                            .changed();
                                        changed |= ui
                                            .selectable_value(&mut self.cfg.click_type, 1, t.double)
                                            .changed();
                                    });
                                ui.end_row();
                            });
                    });
                });
                ui.add_space(4.0);

                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        ui.spacing_mut().interact_size.x = 26.0;
                        ui.label(egui::RichText::new(t.position).strong());
                        changed |= ui
                            .radio_value(&mut self.cfg.use_custom_pos, false, t.cursor)
                            .changed();
                        changed |= ui
                            .radio_value(&mut self.cfg.use_custom_pos, true, "X:")
                            .changed();
                        changed |= ui
                            .add(egui::DragValue::new(&mut self.cfg.pos_x).speed(2.0))
                            .changed();
                        ui.label("Y:");
                        changed |= ui
                            .add(egui::DragValue::new(&mut self.cfg.pos_y).speed(2.0))
                            .changed();
                        if ui.button(t_pick).clicked() {
                            self.start_picking_location();
                        }
                    });
                });
                ui.add_space(8.0);

                let btn_text = if is_running {
                    format!("{} ({hk})", t.stop)
                } else {
                    format!("{} ({hk})", t.start)
                };

                let btn_color = if is_running {
                    egui::Color32::from_rgb(180, 50, 50)
                } else {
                    egui::Color32::from_rgb(45, 90, 150)
                };

                let main_btn = egui::Button::new(
                    egui::RichText::new(btn_text)
                        .size(15.0)
                        .strong()
                        .color(egui::Color32::WHITE),
                )
                .fill(btn_color)
                .rounding(6.0);

                let btn_resp = ui.add_sized([ui.available_width(), 38.0], main_btn);
                self.measured_h = btn_resp.rect.bottom() + 8.0;
                if btn_resp.clicked() {
                    self.state.toggle();
                }

                if changed {
                    self.state.sync_from_config(&self.cfg);
                    self.maybe_autosave();
                }
            });

        if self.fit_stage == 1 {
            self.fit_stage = 2;
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                410.0,
                self.measured_h,
            )));
            ctx.request_repaint();
        }

        if !self.centered_once && self.fit_stage == 2 {
            self.centered_once = true;
            if self.cfg.saved_window_pos().is_none() {
                if let Some(cmd) = egui::ViewportCommand::center_on_screen(ctx) {
                    ctx.send_viewport_cmd(cmd);
                }
            }
        }

        if self.show_hotkey_modal {
            egui::Window::new(t.set_hotkey)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .collapsible(false)
                .resizable(false)
                .fixed_size([220.0, 100.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(t.hotkey_action);
                        let hk_text = if self.hk_recording { "..." } else { hk };
                        let hk_btn =
                            egui::Button::new(egui::RichText::new(hk_text).strong().monospace())
                                .min_size(egui::vec2(120.0, 0.0))
                                .rounding(egui::Rounding::same(6.0));
                        if ui.add(hk_btn).clicked() {
                            self.hk_recording = true;
                        }
                    });
                    if self.hk_recording {
                        ui.colored_label(egui::Color32::YELLOW, t.press_key);
                        let mut hit: Option<&'static str> = None;
                        let mut cancelled = false;
                        ctx.input(|i| {
                            for ev in &i.events {
                                if let egui::Event::Key {
                                    key, pressed: true, ..
                                } = ev
                                {
                                    if *key == egui::Key::Escape {
                                        cancelled = true;
                                    } else if let Some(name) = key_to_name(*key) {
                                        hit = Some(name);
                                    }
                                    break;
                                }
                            }
                        });
                        if cancelled {
                            self.hk_recording = false;
                        } else if let Some(name) = hit {
                            let slot = rebind_hotkey(
                                &self._hotkey_manager,
                                &self.hotkey_slot,
                                name,
                                &self.state,
                            );
                            self.hotkey_slot = slot;
                            self.cfg.hotkey = name.to_string();
                            self.hk_recording = false;
                            self.state.sync_from_config(&self.cfg);
                            self.maybe_autosave();
                        }
                    }
                    ui.add_space(6.0);
                    if ui.button(t.close).clicked() {
                        self.show_hotkey_modal = false;
                        self.hk_recording = false;
                    }
                });
        }

        self.state
            .is_recording_hotkey
            .store(self.hk_recording, Ordering::SeqCst);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.cfg.autosave {
            self.cfg.save();
            return;
        }
        let mut disk_cfg = AppConfig::load();
        disk_cfg.window_x = self.cfg.window_x;
        disk_cfg.window_y = self.cfg.window_y;
        disk_cfg.autosave = false;
        disk_cfg.save();
    }
}

pub fn spawn_show_signal_watcher(gui_ctx: Arc<Mutex<Option<egui::Context>>>) {
    let _ = std::thread::Builder::new()
        .name("show-signal".into())
        .stack_size(BG_THREAD_STACK)
        .spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_millis(300));

            let ctx = match gui_ctx.lock() {
                Ok(g) => g.as_ref().cloned(),
                Err(_) => None,
            };
            let Some(ctx) = ctx else { continue };
            if !crate::instance::take_show_signal() {
                continue;
            }

            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            wake_unfocused(&ctx);

            std::thread::sleep(std::time::Duration::from_millis(250));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            wake_unfocused(&ctx);
        });
}
