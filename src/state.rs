use std::sync::{
    atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicU8, Ordering},
    Condvar, Mutex, OnceLock,
};
use std::time::{Duration, Instant};

use crate::config::AppConfig;

pub const HOTKEY_REBIND_GRACE_MS: u64 = 300;

static EPOCH: OnceLock<Instant> = OnceLock::new();

fn now_ms() -> u64 {
    EPOCH.get_or_init(Instant::now).elapsed().as_millis() as u64
}

pub struct SharedState {
    pub is_running: AtomicBool,
    pub is_picking_location: AtomicBool,
    pub is_recording_hotkey: AtomicBool,
    hotkey_guard_until_ms: AtomicU64,
    pub total_interval_ms: AtomicU64,
    pub repeat_infinite: AtomicBool,
    pub repeat_times: AtomicU64,
    pub mouse_button: AtomicU8,
    pub click_type: AtomicU8,
    pub use_custom_pos: AtomicBool,
    pub pos_x: AtomicI32,
    pub pos_y: AtomicI32,
    wake_lock: Mutex<bool>,
    wake_cond: Condvar,
}

impl SharedState {
    pub fn from_config(cfg: &AppConfig) -> Self {
        let state = Self {
            is_running: AtomicBool::new(false),
            is_picking_location: AtomicBool::new(false),
            is_recording_hotkey: AtomicBool::new(false),
            hotkey_guard_until_ms: AtomicU64::new(0),
            total_interval_ms: AtomicU64::new(3000),
            repeat_infinite: AtomicBool::new(true),
            repeat_times: AtomicU64::new(0),
            mouse_button: AtomicU8::new(0),
            click_type: AtomicU8::new(0),
            use_custom_pos: AtomicBool::new(false),
            pos_x: AtomicI32::new(0),
            pos_y: AtomicI32::new(0),
            wake_lock: Mutex::new(false),
            wake_cond: Condvar::new(),
        };
        state.sync_from_config(cfg);
        state
    }

    pub fn sync_from_config(&self, cfg: &AppConfig) {
        self.total_interval_ms
            .store(cfg.total_interval_ms(), Ordering::Relaxed);
        self.repeat_infinite
            .store(cfg.repeat_infinite, Ordering::Relaxed);
        self.repeat_times.store(cfg.repeat_times, Ordering::Relaxed);
        self.mouse_button.store(cfg.mouse_button, Ordering::Relaxed);
        self.click_type.store(cfg.click_type, Ordering::Relaxed);
        self.use_custom_pos
            .store(cfg.use_custom_pos, Ordering::Relaxed);
        self.pos_x.store(cfg.pos_x, Ordering::Relaxed);
        self.pos_y.store(cfg.pos_y, Ordering::Relaxed);
    }

    #[inline]
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    pub fn set_running(&self, running: bool) {
        self.is_running.store(running, Ordering::SeqCst);
        if let Ok(mut started) = self.wake_lock.lock() {
            *started = running;
            self.wake_cond.notify_all();
        } else {
            self.wake_cond.notify_all();
        }
        if !running {
            crate::memory::trim_memory();
        }
    }

    pub fn toggle(&self) {
        self.set_running(!self.is_running());
    }

    pub fn guard_hotkeys_for(&self, ms: u64) {
        self.hotkey_guard_until_ms
            .store(now_ms().saturating_add(ms), Ordering::SeqCst);
    }

    #[inline]
    pub fn hotkeys_guarded(&self) -> bool {
        let until = self.hotkey_guard_until_ms.load(Ordering::SeqCst);
        until != 0 && now_ms() < until
    }

    pub fn wait_for_start(&self) {
        if let Ok(mut started) = self.wake_lock.lock() {
            while !*started && !self.is_running.load(Ordering::SeqCst) {
                started = self
                    .wake_cond
                    .wait(started)
                    .unwrap_or_else(|poison| poison.into_inner());
            }
        }
    }

    pub fn sleep_interval(&self, total_ms: u64) {
        let spin_ms = if total_ms > 20 {
            let started = self
                .wake_lock
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            let _ = self.wake_cond.wait_timeout_while(
                started,
                Duration::from_millis(total_ms - 15),
                |started| *started,
            );
            15
        } else {
            total_ms
        };

        if spin_ms > 0 && self.is_running.load(Ordering::SeqCst) {
            spin_sleep::sleep(Duration::from_millis(spin_ms));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Instant;

    fn state_running() -> Arc<SharedState> {
        let state = Arc::new(SharedState::from_config(&AppConfig::default()));
        state.set_running(true);
        state
    }

    #[test]
    fn stop_is_instant_inside_long_interval() {
        let state = state_running();
        let worker_state = Arc::clone(&state);
        let worker = std::thread::spawn(move || worker_state.sleep_interval(5_000));

        std::thread::sleep(Duration::from_millis(100));
        let start = Instant::now();
        state.set_running(false);
        worker.join().expect("worker thread must not panic");

        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(500),
            "STOP must wake the worker immediately instead of waiting {elapsed:?}"
        );
    }

    #[test]
    fn short_interval_stays_precise() {
        let state = state_running();
        let start = Instant::now();
        state.sleep_interval(10);
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(9),
            "slept too little: {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_millis(40),
            "slept too long: {elapsed:?}"
        );
    }

    #[test]
    fn does_not_wait_after_stop() {
        let state = state_running();
        state.set_running(false);
        let start = Instant::now();
        state.sleep_interval(5_000);
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(200),
            "must not wait after STOP: {elapsed:?}"
        );
    }

    #[test]
    fn hotkey_guard_active_then_expires() {
        let state = SharedState::from_config(&AppConfig::default());
        assert!(!state.hotkeys_guarded(), "guard must be off by default");

        state.guard_hotkeys_for(50);
        assert!(
            state.hotkeys_guarded(),
            "guard must be active right after it is set"
        );

        std::thread::sleep(Duration::from_millis(120));
        assert!(!state.hotkeys_guarded(), "guard must expire on its own");
    }
}
