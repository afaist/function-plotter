mod app;
mod parser;
mod evaluator;
mod renderer;
mod export;
mod session;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Function Plotter",
        options,
        Box::new(|_cc| Ok(Box::new(app::PlotApp::default()))),
    )
}
