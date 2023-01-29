mod gui;
mod server;
use std::{fs, thread};

use tokio::sync::mpsc::{channel, Receiver, Sender};

use gui::{MultiPlot, WIDTH};
use server::{start_threaded_server, QueryParam, QUERY_SENDER};
fn main() {
    let config = fs::read_to_string("config.json").unwrap();
    let mut plots_with_cfg: MultiPlot = serde_json::from_str(&config).unwrap();
    let summary: Vec<_> = plots_with_cfg
        .plots
        .iter()
        .map(|p| format!("[name={},x={},y={}]", &p.name, &p.x_axis, &p.y_axis))
        .collect();
    plots_with_cfg.append_log(format!("Loaded from config.json, port={}:{:?}", plots_with_cfg.port,summary).as_str());
    let (sender, recv) = channel::<QueryParam>(512);
    plots_with_cfg.set_recv(recv);

    *QUERY_SENDER.lock().unwrap() = Some(sender);
    let _server_handle = start_threaded_server(plots_with_cfg.port);
    let options = eframe::NativeOptions {
        min_window_size: Some((WIDTH * 3.0 + 50.0, WIDTH * 2.0 + 50.0).into()),
        ..Default::default()
    };

    eframe::run_native(
        "Stormworks Telemetry Panel v0.1.0",
        options,
        Box::new(|_cc| Box::new(plots_with_cfg)),
    )
}
