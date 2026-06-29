pub mod core;
pub mod models;
pub mod ui;

use ui::LauncherApp;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize Tokio multi-threaded runtime
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    // 2. Enter runtime context so tokio::spawn and other features work smoothly inside the GUI threads
    let _guard = rt.enter();

    // 3. Configure GUI window parameters
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([550.0, 480.0])
            .with_min_inner_size([500.0, 400.0])
            .with_resizable(true),
        ..Default::default()
    };

    // 4. Run GUI native window loop
    eframe::run_native(
        "TitanHost Configurator",
        options,
        Box::new(|cc| Box::new(LauncherApp::new(cc))),
    )
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    Ok(())
}
