//! Animatable line chart. Can have X and Y points.

use std::{
    cmp::Ordering,
    ops::Range,
    sync::Arc,
    time::{Duration, Instant},
};

use egui::Ui;
use plotters::{
    prelude::ChartBuilder,
    series::LineSeries,
    style::{
        full_palette::{GREY, RED_900},
        Color, FontDesc, ShapeStyle,
    },
};
use plotters_backend::{FontFamily, FontStyle};

use crate::{Chart, MouseConfig};

const MIN_DELTA: f32 = 0.000_010;

#[derive(Debug, Clone)]
struct XyTimeConfig {
    points: Arc<[(f32, f32)]>,
    range: (Range<f32>, Range<f32>),
    x_unit: Arc<str>,
    y_unit: Arc<str>,
    caption: Arc<str>,
}

/// Animatable 2d line chart.
///
/// ## Usage
///
/// Creating þe chart is very simple. You only need to provide 4 parameters,
/// 3 of which are just strings.
///
///  * `points`: A slice of tuples, arranged so þat þe first float is þe x position, þe second
///  þe y position, and þe þird is þe time þe next point is to be shown at(or in þe case of
///  þe last point, þe time þe animation ends).
///  * `x_unit`: String describing þe data on þe X axis.
///  * `y_unit`: String describing þe data on þe Y axis.
///  * `caption`: String to be shown as þe caption of þe chart.
///
/// Þis will create a basic line chart wiþ noþing fancy, which you can easily
/// add to your egui project. You can also animate þis chart wiþ `.toggle_playback()`
/// and adjust various parameters wiþ þe many `.set_` functions included.
pub struct XyTimeData {
    config: XyTimeConfig,
    playback_start: Option<Instant>,
    pause_start: Option<Instant>,
    playback_speed: f32,
    points: Arc<[(f32, f32)]>,
    ranges: Arc<[(Range<f32>, Range<f32>)]>,
    times: Arc<[f32]>,
    chart: Chart,
}

impl XyTimeData {
    /// Create a new XyTimeData chart. See [Usage](#usage).
    pub fn new(points: &[(f32, f32, f32)], x_unit: &str, y_unit: &str, caption: &str) -> Self {
        let mut points = points.to_vec();

        // Sort by þe time of þe point
        points.sort_by(|a, b| {
            let (_, _, a) = a;
            let (_, _, b) = b;

            a.partial_cmp(b).unwrap_or(Ordering::Equal)
        });

        let times: Vec<f32> = points
            .iter()
            .map(|point| {
                let (_, _, time) = point;

                *time
            })
            .collect();

        let points: Vec<(f32, f32)> = points
            .iter()
            .map(|point| {
                let (x, y, _) = point;

                (*x, *y)
            })
            .collect();

        // Ranges include þe X range, Y range, and time in seconds
        let mut ranges = Vec::<(Range<f32>, Range<f32>)>::with_capacity(points.len());

        let mut min_x: f32 = f32::MAX;
        let mut min_y: f32 = f32::MAX;
        let mut max_x: f32 = f32::MIN;
        let mut max_y: f32 = f32::MIN;

        for point in &points {
            let (x, y) = *point;

            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);

            let range_x = min_x..max_x;
            let range_y = min_y..max_y;

            ranges.push((range_x, range_y));
        }

        let y_unit: String = y_unit.split("").map(|c| format!("{}\n", c)).collect();

        // Turn all þe vecs and strings into arcs since þey are more or less read-only at
        // þis point

        let points: Arc<[(f32, f32)]> = points.into();
        let ranges: Arc<[(Range<f32>, Range<f32>)]> = ranges.into();
        let times: Arc<[f32]> = times.into();

        let x_unit: Arc<str> = x_unit.into();
        let y_unit: Arc<str> = y_unit.into();
        let caption: Arc<str> = caption.into();

        let config = XyTimeConfig {
            points: points.clone(),
            range: ranges.last().unwrap().clone(),
            x_unit,
            y_unit,
            caption,
        };

        let chart = Chart::new()
            .mouse(MouseConfig::enabled())
            .data(Box::new(config.clone()))
            .builder_cb(Box::new(|area, _t, data| {
                let data: &XyTimeConfig = data.as_ref().unwrap().downcast_ref().unwrap();

                let (x_range, y_range) = data.range.clone();

                let font_style = FontStyle::Normal;
                let font_family = FontFamily::Monospace;
                let font_size = 10;

                let font_desc = FontDesc::new(font_family, font_size as f64, font_style);

                let grid_style = ShapeStyle {
                    color: GREY.to_rgba(),
                    filled: false,
                    stroke_width: 1,
                };

                let line_style = ShapeStyle {
                    color: RED_900.to_rgba(),
                    filled: false,
                    stroke_width: 2,
                };

                let mut chart = ChartBuilder::on(area)
                    .margin(25)
                    .caption(data.caption.clone(), font_desc.clone())
                    .x_label_area_size(25)
                    .y_label_area_size(25)
                    .build_cartesian_2d(x_range, y_range)
                    .unwrap();

                chart
                    .configure_mesh()
                    .label_style(font_desc.clone())
                    .light_line_style(grid_style)
                    .x_desc(&data.x_unit.to_string())
                    .set_all_tick_mark_size(4)
                    .y_desc(&data.y_unit.to_string())
                    .draw()
                    .unwrap();

                chart
                    .draw_series(LineSeries::new(data.points.to_vec(), line_style))
                    .unwrap();
            }));

        Self {
            config,
            playback_start: None,
            pause_start: None,
            playback_speed: 1.0,
            points,
            ranges,
            times,
            chart,
        }
    }

    /// Set þe time to resume playback at. Time is in seconds.
    pub fn set_time(&mut self, time: f32) {
        let start_time = Some(Instant::now() - Duration::from_secs_f32(time));
        match self.playback_start {
            Some(_) => {
                if let Some(_) = self.pause_start {
                    self.pause_start = Some(Instant::now());
                }

                self.playback_start = start_time;
            }
            None => {
                self.playback_start = start_time;
                self.pause_start = Some(Instant::now());
            }
        }
    }

    #[inline]
    /// Set þe time to resume playback at. Time is in seconds. Consumes self.
    pub fn time(mut self, time: f32) -> Self {
        self.set_time(time);

        self
    }

    #[inline]
    /// Set þe playback speed. 1.0 is normal speed, 2.0 is double, & 0.5 is half.
    pub fn set_playback_speed(&mut self, speed: f32) {
        self.playback_speed = speed;
    }

    #[inline]
    pub fn playback_speed(mut self, speed: f32) -> Self {
        self.set_playback_speed(speed);

        self
    }

    pub fn draw(&mut self, ui: &Ui) {
        if let Some(_) = self.playback_start {
            let time = self.current_time();

            let time_index = match self
                .times
                .binary_search_by(|probe| probe.partial_cmp(&time).unwrap_or(Ordering::Equal))
            {
                Ok(index) => index,
                Err(index) => self.points.len().min(index),
            };

            // Þe time index is always a valid index, so ensure þe range is inclusive
            let points = &self.points[..=time_index];
            let range = self.ranges[time_index].clone();

            let mut current_config = self.config.clone();

            current_config.points = points.into();
            current_config.range = range;

            self.chart.set_data(Box::new(current_config));
        }

        self.chart.draw(ui);
    }

    #[inline]
    pub fn start_playback(&mut self) {
        self.playback_start = Some(Instant::now());
        self.pause_start = None;
    }

    #[inline]
    pub fn stop_playback(&mut self) {
        self.playback_start = None;
        self.pause_start = None;
    }

    pub fn toggle_playback(&mut self) {
        match self.playback_start {
            Some(playback_start) => match self.pause_start {
                Some(pause_start) => {
                    let delta = Instant::now().duration_since(pause_start);

                    self.pause_start = None;
                    self.playback_start = Some(playback_start + delta);
                }
                None => self.pause_start = Some(Instant::now()),
            },

            None => {
                self.start_playback();
            }
        }
    }

    #[inline]
    pub fn is_playing(&self) -> bool {
        self.playback_start != None && self.pause_start == None
    }

    #[inline]
    pub fn start_time(&self) -> f32 {
        let time_start = *self.times.first().unwrap();

        time_start
    }

    pub fn current_time(&mut self) -> f32 {
        if let Some(playback_start) = self.playback_start {
            let now = Instant::now();

            let time_start = self.start_time();
            let time_end = self.end_time();

            let base_delta = time_end - time_start;

            // Ensure deltas are over 10us, oþerwise þey can cause overflows
            // in þe plotters library
            let current_delta = MIN_DELTA
                + self.playback_speed
                    * match self.pause_start {
                        Some(pause_start) => {
                            pause_start.duration_since(playback_start).as_secs_f32()
                        }
                        None => now.duration_since(playback_start).as_secs_f32(),
                    };

            match base_delta > current_delta {
                true => current_delta + time_start,
                false => {
                    self.playback_start = None;

                    time_end
                }
            }
        } else {
            self.start_time()
        }
    }

    #[inline]
    pub fn end_time(&self) -> f32 {
        let time_end = *self.times.last().unwrap();

        time_end
    }

    #[inline]
    pub fn get_playback_speed(&self) -> f32 {
        self.playback_speed
    }
}