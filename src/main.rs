mod app;
mod pane;
mod fs;

use app::FileManagerApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "xx_File_Commander_Vibez_xx",
        options,
        Box::new(|cc| Ok(Box::new(FileManagerApp::new(cc)))),
    )
}
