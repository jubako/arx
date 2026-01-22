#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use app::ArxApp;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Arx Archive Viewer")
            .with_app_id("org.jubako.arx"),
        ..Default::default()
    };

    let archive = std::env::args().nth(1);

    eframe::run_native(
        "Arx Archive Viewer",
        options,
        Box::new(|cc| Ok(Box::new(ArxApp::new(cc, archive)))),
    )
}
