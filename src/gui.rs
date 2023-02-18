use std::collections::HashMap;

use chrono::{DateTime, Local};
use eframe::Theme;
use egui::plot::{Corner, Legend, Line, MarkerShape, Plot, PlotPoints, PlotUi, Points};
use serde::{Deserialize, Serialize};

pub(crate) const WIDTH: f32 = 200.0;

/// support multiple lines in one plot
#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct RealTimePlot {
    #[serde(skip)]
    line: Vec<Vec<[f64; 2]>>,
    #[serde(skip)]
    latest: Vec<[f64; 2]>,
    #[serde(skip)]
    name2idx: HashMap<String, usize>,
    pub name: String,
    pub x_axis: String,
    pub y_axis: Vec<String>,
}

impl RealTimePlot {
    fn init(&mut self) {
        for (k, v) in self.y_axis.iter().enumerate() {
            self.name2idx.insert(v.clone(), k);
        }
        let len = self.y_axis.len();
        self.line.resize(len, Default::default());
        self.latest.resize(len, Default::default());
    }
    fn clean_data(&mut self) {
        self.line.drain(..);
        self.latest.drain(..);
        self.init();
    }
    /// plot line(s) with latest point marked with a cross
    fn plot(&self, plot_ui: &mut PlotUi) {
        for (idx, (line, latest)) in self.line.iter().zip(&self.latest).enumerate() {
            let len = line.len();
            let yrange = if len < 10_0000 {
                0..len
            } else {
                len - 10_000..len
            };
            let y_axis_name = self.y_axis[idx].clone();
            let line = &line[yrange];
            plot_ui.line(Line::new(PlotPoints::new(Vec::from(line))).name(y_axis_name));
            plot_ui.points(
                Points::new(PlotPoints::new(vec![*latest]))
                    .shape(MarkerShape::Cross)
                    .radius(5.0),
            );
        }
    }
    /// push a point into plots
    fn push(&mut self, axis_name: &str, pt: [f64; 2]) {
        let idx = self.name2idx[axis_name];
        self.line[idx].push(pt);
        self.latest[idx] = pt;
    }

    /// append a Vec of list into plot
    fn append(&mut self, name: &str, mut pts: Vec<[f64; 2]>) {
        if let Some(&idx) = self.name2idx.get(name) {
            let (line, latest) = (&mut self.line[idx], &mut self.latest[idx]);
            if let Some(last) = pts.last() {
                *latest = last.to_owned();
            }
            line.append(&mut pts);
        }
    }

    fn all_axis_names(&self) -> Vec<String> {
        let mut r = vec![self.x_axis.clone()];
        r.append(&mut self.y_axis.clone());
        r
    }

    fn update_points(&mut self, new_points: &HashMap<String, Vec<f64>>) {
        if let Some(new_x) = new_points.get(&self.x_axis) {
            for y_axis in self.y_axis.clone() {
                if let Some(new_y) = new_points.get(&y_axis) {
                    let pts: Vec<[f64; 2]> =
                        new_x.iter().zip(new_y).map(|(x, y)| [*x, *y]).collect();
                    self.append(&y_axis, pts);
                }
            }
        }
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
    fn reset(&mut self) {
        for plot in &mut self.plots{
            plot.clean_data()
        }
    }
    pub(crate) fn init(&mut self) {
        for plot in &mut self.plots {
            plot.init();
        }
    }
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
            for plot in &mut self.plots {
                plot.update_points(&new_points)
            }
            // TODO: check for nonexist axis name
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

    fn plot_panel(&mut self, ui: &mut egui::Ui) {
        for i in [0, 1] {
            ui.horizontal(|ui| {
                for j in 0..3 {
                    let x_axis = self.plots[i * 3 + j].x_axis.clone();
                    Plot::new(format!("{i}_{j}"))
                        .view_aspect(1.0)
                        .width(WIDTH)
                        .legend(Legend::default().position(Corner::LeftBottom))
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
        let table = TableBuilder::new(ui)
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
                if ui
                    .button("reset")
                    .on_hover_text("Clean all plot data")
                    .clicked()
                {
                    self.reset()
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
