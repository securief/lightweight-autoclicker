#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    pub struct Picker;

    impl Picker {
        pub fn new() -> Self {
            Self
        }

        pub fn sample(&self) -> (i32, i32, bool) {
            let mut pt = POINT { x: 0, y: 0 };
            let ok = unsafe { GetCursorPos(&mut pt) } != 0;
            let (x, y) = if ok { (pt.x, pt.y) } else { (0, 0) };
            let left = unsafe { GetAsyncKeyState(VK_LBUTTON as i32) as u16 & 0x8000 } != 0;
            (x, y, left)
        }
    }

    impl Default for Picker {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use device_query::{DeviceQuery, DeviceState};

    pub struct Picker {
        dev: DeviceState,
    }

    impl Picker {
        pub fn new() -> Self {
            Self {
                dev: DeviceState::new(),
            }
        }

        pub fn sample(&self) -> (i32, i32, bool) {
            let mouse = self.dev.get_mouse();
            let left = mouse.button_pressed.get(1).copied().unwrap_or(false);
            (mouse.coords.0, mouse.coords.1, left)
        }
    }

    impl Default for Picker {
        fn default() -> Self {
            Self::new()
        }
    }
}

pub use imp::Picker;

#[cfg(all(test, windows))]
mod tests {
    use super::Picker;

    #[test]
    fn picker_reads_sane_position() {
        let picker = Picker::new();
        let (x, y, _left) = picker.sample();
        assert!(
            (-100_000..100_000).contains(&x),
            "unreasonable X coordinate: {x}"
        );
        assert!(
            (-100_000..100_000).contains(&y),
            "unreasonable Y coordinate: {y}"
        );
        let (x2, y2, _left2) = picker.sample();
        assert!((-100_000..100_000).contains(&x2));
        assert!((-100_000..100_000).contains(&y2));
    }
}
