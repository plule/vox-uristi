use eframe::egui;

use crate::VERSION;

mod app;

const ICON: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/icon"));

pub fn run() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([320.0, 240.0])
            .with_icon(egui::IconData {
                rgba: ICON.to_vec(),
                width: 256,
                height: 256,
            }),
        ..Default::default()
    };
    match eframe::run_native(
        format!("Vox Uristi v{VERSION}").as_str(),
        options,
        Box::new(|cc| Ok(Box::<app::App>::new(app::App::new(cc)))),
    ) {
        Ok(_) => Ok(()),
        Err(e) => Err(anyhow::format_err!("{}", e)),
    }
}
