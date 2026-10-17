use std::{
    sync::{atomic::Ordering, Arc, Mutex},
    time::Duration,
};

use eframe::egui;
use enigo::{Button, Coordinate, Direction, Enigo, Mouse, Settings};

use crate::{hotkeys::wake_unfocused, memory::BG_THREAD_STACK, state::SharedState};

pub fn spawn_clicker_worker(state: Arc<SharedState>, gui_ctx: Arc<Mutex<Option<egui::Context>>>) {
    let _ = std::thread::Builder::new()
        .name("clicker".into())
        .stack_size(BG_THREAD_STACK)
        .spawn(move || {
            let mut enigo = match Enigo::new(&Settings::default()) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("Enigo init failed (check Accessibility permission): {e:?}");
                    return;
                }
            };

            loop {
                state.wait_for_start();

                let mut clicks_done: u64 = 0;

                while state.is_running.load(Ordering::SeqCst) {
                    if state.use_custom_pos.load(Ordering::Relaxed) {
                        let x = state.pos_x.load(Ordering::Relaxed);
                        let y = state.pos_y.load(Ordering::Relaxed);
                        let _ = enigo.move_mouse(x, y, Coordinate::Abs);
                    }

                    let btn = match state.mouse_button.load(Ordering::Relaxed) {
                        1 => Button::Right,
                        2 => Button::Middle,
                        _ => Button::Left,
                    };

                    let _ = enigo.button(btn, Direction::Click);
                    if state.click_type.load(Ordering::Relaxed) == 1 {
                        spin_sleep::sleep(Duration::from_millis(35));
                        let _ = enigo.button(btn, Direction::Click);
                    }

                    clicks_done += 1;

                    if !state.repeat_infinite.load(Ordering::Relaxed) {
                        let max_times = state.repeat_times.load(Ordering::Relaxed).max(1);
                        if clicks_done >= max_times {
                            state.set_running(false);
                            if let Ok(g) = gui_ctx.lock() {
                                if let Some(ctx) = g.as_ref() {
                                    wake_unfocused(ctx);
                                }
                            }
                            break;
                        }
                    }

                    let interval = state.total_interval_ms.load(Ordering::Relaxed);
                    state.sleep_interval(interval);
                }
            }
        });
}
