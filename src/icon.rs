use eframe::egui;

const SIZE: u32 = 32;
const RGBA: &[u8] = include_bytes!("../assets/app_icon.rgba");

pub fn app_icon() -> egui::IconData {
    debug_assert_eq!(RGBA.len(), (SIZE * SIZE * 4) as usize);
    egui::IconData {
        rgba: RGBA.to_vec(),
        width: SIZE,
        height: SIZE,
    }
}
