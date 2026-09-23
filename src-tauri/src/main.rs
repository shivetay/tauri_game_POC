#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;
use map_tests::app::MapApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("map_tests"),
        ..Default::default()
    };
    eframe::run_native(
        "map_tests",
        options,
        Box::new(|_cc| Ok(Box::new(MapApp::new()))),
    )
}
