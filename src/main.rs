#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod hotkeys;
mod icon;
mod instance;
mod memory;
mod picker;
mod state;
mod texts;
mod ui;
mod worker;

use std::sync::{Arc, Mutex};

use eframe::egui;

use crate::{
    config::AppConfig, instance::WINDOW_TITLE, state::SharedState, ui::LightweightAutoclickerApp,
    worker::spawn_clicker_worker,
};

fn main() -> eframe::Result<()> {
    let _instance_lock = match instance::acquire() {
        Some(lock) => lock,
        None => return Ok(()),
    };

    let cfg = AppConfig::load();
    let state = Arc::new(SharedState::from_config(&cfg));
    let gui_ctx: Arc<Mutex<Option<egui::Context>>> = Arc::new(Mutex::new(None));

    spawn_clicker_worker(Arc::clone(&state), Arc::clone(&gui_ctx));
    ui::spawn_show_signal_watcher(Arc::clone(&gui_ctx));

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([410.0, 240.0])
        .with_resizable(false)
        .with_maximize_button(false)
        .with_icon(crate::icon::app_icon());
    if let Some((x, y)) = cfg.saved_window_pos() {
        viewport = viewport.with_position(egui::pos2(x, y));
    }
    let native_options = eframe::NativeOptions {
        viewport,
        multisampling: 0,
        depth_buffer: 0,
        stencil_buffer: 0,
        dithering: false,
        ..Default::default()
    };

    eframe::run_native(
        WINDOW_TITLE,
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(LightweightAutoclickerApp::new(
                cc, state, cfg, gui_ctx,
            )))
        }),
    )
}
