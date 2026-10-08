//! Temporary entry point; replaced by the full app wiring.

use clap::Parser;
use presto::{cli::Cli, theme};

struct App {
    ready: bool,
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.ready {
            theme::install(ctx);
            theme::apply(ctx, &theme::Palette::dark());
            self.ready = true;
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ui, |_| {});
    }
}

fn main() -> eframe::Result {
    let cli = Cli::parse();
    let title = if cli.demo { "Presto (Demo)" } else { "Presto" };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native("presto", options, Box::new(|_| Ok(Box::new(App { ready: false }))))
}
