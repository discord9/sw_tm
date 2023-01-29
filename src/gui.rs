use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
    time::Instant,
};

use chrono::{DateTime, Local, NaiveDate, TimeZone};
use eframe::Theme;
use egui::plot::{Legend, Line, MarkerShape, Plot, PlotPoints, PlotUi, Points};
use serde::{Deserialize, Serialize};

pub(crate) const WIDTH: f32 = 200.0;
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct RealTimePlot {
    #[serde(skip)]
    line: Vec<[f64; 2]>,
    #[serde(skip)]
    latest: [f64; 2],
    pub name: String,
    pub x_axis: String,
    pub y_axis: String,
}

impl RealTimePlot {
    fn clean_data(&mut self) {
        self.line.drain(..);
        self.latest = [0.0, 0.0];
    }
    /// plot a line with latest point marked with a cross
    fn plot(&self, plot_ui: &mut PlotUi) {
        plot_ui.line(Line::new(PlotPoints::new(self.line.clone())).name(&self.y_axis));
        plot_ui.points(
            Points::new(PlotPoints::new(vec![self.latest]))
                .shape(MarkerShape::Cross)
                .radius(5.0),
        );
    }
    /// push a point into plots
    fn push(&mut self, pt: [f64; 2]) {
        self.line.push(pt);
        self.latest = pt;
    }

    /// append a Vec of list into plot
    fn append(&mut self, pts: &mut Vec<[f64; 2]>) {
        if let Some(last) = pts.last() {
            self.latest = last.to_owned();
        }
        self.line.append(pts);
    }
}

#[derive(Debug, PartialEq, Eq, Default)]
enum Panel {
    #[default]
    Plots,
    Log,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct MultiPlot {
    pub(crate) port: u16,
    pub(crate) plots: [RealTimePlot; 6],
    #[serde(skip)]
    time: f64,
    #[serde(skip)]
    recv: Option<tokio::sync::mpsc::Receiver<Vec<(String, String)>>>,
    #[serde(skip)]
    open_panel: Panel,
    #[serde(skip)]
    logs: Vec<(DateTime<Local>, String)>,
}

#[test]
fn test_split() {
    let a = "123,456";
    dbg!(a
        .split(&[',', ' '])
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<f64>())
        .collect::<Vec<_>>());
}

impl Default for MultiPlot {
    fn default() -> Self {
        Self {
            port: 14514,
            time: 0.0,
            plots: Default::default(),
            recv: None,
            open_panel: Default::default(),
            logs: vec![(Local::now(), "Place Holder Message".to_string())],
        }
    }
}

impl MultiPlot {
    pub(crate) fn set_recv(&mut self, recv: tokio::sync::mpsc::Receiver<Vec<(String, String)>>) {
        self.recv = Some(recv)
    }
    /// receive as many as possible of those datapoints, add them to the plots, return total received data point numbers if uccess
    fn check_and_update(&mut self) -> Result<usize, String> {
        if let Some(recv) = &mut self.recv {
            let mut new_points = HashMap::new();
            let mut cnt = 0;
            while let Ok(msg) = recv.try_recv() {
                for (column_name, column_num) in msg {
                    let nums: Vec<_> = column_num
                        .split(&[',', ' '])
                        .filter(|s| !s.is_empty())
                        .map(|s| s.parse::<f64>())
                        .collect::<Result<_, _>>()
                        .map_err(|err| err.to_string())?;
                    cnt += nums.len();
                    new_points.insert(column_name, nums);
                }
            }
            let mut is_in = HashSet::new();
            for plot in &mut self.plots {
                if let (Some(col_x), Some(col_y)) =
                    (new_points.get(&plot.x_axis), new_points.get(&plot.y_axis))
                {
                    is_in.insert(plot.x_axis.clone());
                    is_in.insert(plot.y_axis.clone());
                    let mut new_line = col_x
                        .iter()
                        .zip(col_y)
                        .map(|pt| [*pt.0, *pt.1])
                        .collect::<Vec<_>>();
                    plot.append(&mut new_line);
                }
            }
            if is_in.len() != new_points.len() {
                let not_in: HashSet<_> =
                    new_points.keys().filter(|k| !is_in.contains(*k)).collect();
                self.append_log(&format!("Unknown column name: {not_in:?}"));
            }
            Ok(cnt)
        } else {
            Err("no channel is found".to_string())
        }
    }
    pub(crate) fn with_recv(recv: tokio::sync::mpsc::Receiver<Vec<(String, String)>>) -> Self {
        Self {
            recv: Some(recv),
            ..Default::default()
        }
    }

    pub(crate) fn append_log(&mut self, s: &str) {
        self.logs.push((Local::now(), s.to_string()))
    }
    fn mock_append_points(&mut self) {
        let time = self.time;
        for p in &mut self.plots {
            p.push([time, time.sin()]);
        }
    }

    fn plot_panel(&mut self, ui: &mut egui::Ui) {
        for i in [0, 1] {
            ui.horizontal(|ui| {
                for j in 0..3 {
                    let x_axis = self.plots[i * 3 + j].x_axis.clone();
                    Plot::new(format!("{i}_{j}"))
                        .view_aspect(1.0)
                        .width(WIDTH)
                        .legend(Legend::default())
                        .x_axis_formatter(move |x, _| format!("{x_axis}:{x}"))
                        .show(ui, |plot_ui| {
                            self.plots[i * 3 + j].plot(plot_ui);
                        });
                }
            });
            ui.end_row()
        }
    }
    fn logs_panel(&mut self, ui: &mut egui::Ui) {
        use egui_extras::{Column, TableBuilder};
        let mut table = TableBuilder::new(ui)
            .striped(true)
            .column(Column::auto())
            .column(Column::remainder().resizable(true));
        table
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.strong("Time");
                });
                header.col(|ui| {
                    ui.strong("Error Logs");
                });
            })
            .body(|mut body| {
                let len = self.logs.len();
                body.rows(30.0, len, |row_index, mut row| {
                    let (time, msg) = &self.logs[len - 1 - row_index];
                    row.col(|ui| {
                        ui.label(time.format("%H:%M:%S%.3f").to_string());
                    });
                    row.col(|ui| {
                        ui.label(msg);
                    });
                })
            });
    }
}

impl eframe::App for MultiPlot {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_visuals(Theme::Light.egui_visuals());
        ctx.request_repaint();
        match self.check_and_update() {
            Err(err) => self.logs.push((Local::now(), err)),
            Ok(_cnt) => (),
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            self.time += ui.input().unstable_dt.min(1.0 / 30.0) as f64;
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.open_panel, Panel::Plots, "plots");
                ui.selectable_value(&mut self.open_panel, Panel::Log, "logs");
                ui.add_space(20.0);
                if ui.button("reset").on_hover_text("Clean all plot data").clicked() {
                    for plot in &mut self.plots {
                        plot.clean_data();
                    }
                }
            });
            ui.separator();
            match self.open_panel {
                Panel::Plots => self.plot_panel(ui),
                Panel::Log => self.logs_panel(ui),
            }
        });
    }
}
