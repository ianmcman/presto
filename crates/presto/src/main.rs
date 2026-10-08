//! Entry point: CLI -> launch config -> Backend -> eframe.
use clap::Parser;
use presto::{app::App, backend::Backend, cli::Cli, launch};
use presto_core::paths::Paths;
use std::process::exit;

fn main() -> eframe::Result {
    let cli = Cli::parse();
    let cfg = if cli.demo {
        let mock = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|p| p.to_path_buf()))
            .ok_or_else(|| "cannot locate the executable".to_string())
            .and_then(|dir| launch::mock_path(&dir, std::env::var_os("PRESTO_ENGINE_MOCK")))
            .unwrap_or_else(|e| {
                eprintln!("{e}");
                exit(2)
            });
        let Some(rt) = std::env::var_os("XDG_RUNTIME_DIR") else {
            eprintln!("XDG_RUNTIME_DIR is not set");
            exit(2)
        };
        launch::demo_config(&Paths::from_env().state, rt.as_ref(), mock, launch::demo_extra(&cli.faults))
    } else {
        launch::real_config(&cli.engine_dir)
    };
    let backend = cfg.and_then(Backend::start).unwrap_or_else(|e| {
        eprintln!("presto: {e}");
        exit(1)
    });
    let demo = cli.demo;
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(if demo { "Presto (Demo)" } else { "Presto" })
            .with_app_id("presto")
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([900.0, 600.0]),
        // Demo geometry must not land in the real profile.
        persist_window: !demo,
        ..Default::default()
    };
    eframe::run_native("presto", options, Box::new(move |cc| Ok(Box::new(App::new(backend, demo, &cc.egui_ctx)))))
}
