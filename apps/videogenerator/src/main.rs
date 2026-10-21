use std::sync::Arc;

use videogenerator_engine_host::MockEngine;
use videogenerator_ui_egui::VideoGeneratorApp;

fn main() -> eframe::Result {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("VideoGeneratorAI")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "VideoGeneratorAI",
        native_options,
        Box::new(|cc| {
            Ok(Box::new(VideoGeneratorApp::new(
                &cc.egui_ctx,
                Arc::new(MockEngine::default()),
            )))
        }),
    )
}
