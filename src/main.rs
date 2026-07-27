// Prevents a console window from appearing alongside the app on Windows
// (unconditional: applies to debug builds too).
#![cfg_attr(windows, windows_subsystem = "windows")]

mod config;
mod fonts;
mod launcher;
mod model;
mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("LaunchX")
            .with_inner_size([1080.0, 720.0])
            .with_min_inner_size([640.0, 480.0]),
        // wgpu (DX12 on Windows) avoids the OpenGL-driver flickering
        // seen with the default glow renderer.
        renderer: eframe::Renderer::Wgpu,
        vsync: true,
        ..Default::default()
    };
    eframe::run_native(
        "LaunchX",
        options,
        Box::new(|cc| Ok(Box::new(ui::LaunchXApp::new(cc)))),
    )
}
