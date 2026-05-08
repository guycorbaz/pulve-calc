mod calc;
mod config;
mod db;
mod pdf;
mod ui;

fn main() -> eframe::Result<()> {
    let (cfg, config_warning) = config::Config::load();
    let database = db::Database::open().expect("Impossible d'ouvrir la base de données");

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1080.0, 780.0])
            .with_min_inner_size([840.0, 600.0])
            .with_title("Pulve-Calc — Calculateur de pulvérisation"),
        ..Default::default()
    };

    eframe::run_native(
        "Pulve-Calc",
        options,
        Box::new(move |cc| {
            let mut style = (*cc.egui_ctx.style()).clone();
            style.text_styles.iter_mut().for_each(|(_, font_id)| {
                font_id.size *= 1.45;
            });
            style.spacing.button_padding = eframe::egui::vec2(10.0, 6.0);
            style.spacing.interact_size.y = 28.0;
            cc.egui_ctx.set_style(style);

            let mut app = ui::PulveApp::new(cfg, database);
            if let Some(warning) = config_warning {
                app.set_config_warning(warning);
            }
            Ok(Box::new(app))
        }),
    )
}
