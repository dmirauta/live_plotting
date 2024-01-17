mod live_plot;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    // env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    eframe::run_native(
        "Live plot",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Box::<live_plot::LivePlot>::default()),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    // eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    // log::info!("Starting WASM app");
    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                "the_canvas_id",
                eframe::WebOptions::default(),
                Box::new(|_cc| Box::<live_plot::LivePlot>::default()),
            )
            .await
            .expect("failed to start eframe");
    });
}
