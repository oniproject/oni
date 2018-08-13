use std::{
    rc::Rc,
    time::{Instant, Duration},
};
use kiss3d::{
    window::{State, Window},
    text::Font,
    event::{Action, WindowEvent, Key, MouseButton},
    scene::PlanarSceneNode,
    camera::{Camera, FixedView},
    planar_camera::PlanarCamera,
    post_processing::PostProcessingEffect,
};
use nalgebra::{
    Translation2,
    UnitComplex,
    Point2,
    Point3,
    Point3 as Color,
};
use crate::{
    actor::Actor,
    client::Client,
    server::Server,
    lag::LagNetwork,
    consts::*,
};

pub fn duration_to_secs(duration: Duration) -> f32 {
    duration.as_secs() as f32 + (duration.subsec_nanos() as f32 / 1.0e9)
}

pub fn secs_to_duration(secs: f32) -> Duration {
    Duration::new(secs as u64, ((secs % 1.0) * 1.0e9) as u32)
}


pub struct View {
    pub start: f32,
    pub middle: f32,
    pub end: f32,
}

impl View {
    pub fn new(start: f32, height: f32) -> Self {
        Self {
            start,
            middle: start + height / 2.0,
            end: start + height,
        }
    }
    pub fn render_nodes<'a>(&self, win: &mut Window, mouse: Point2<f32>, actors: impl Iterator<Item=&'a mut Actor>) {
        for e in actors {
            e.render(win, self.middle, mouse)
        }
    }
}

pub struct Text<'a> {
    font: Rc<Font>,
    win: &'a mut Window,
}

impl<'a> Text<'a> {
    pub fn new(win: &'a mut Window, font: Rc<Font>) -> Self {
        Self { font, win }
    }
    fn draw(&mut self, at: Point2<f32>, color: [f32; 3], msg: &str) {
        self.win.draw_text(msg, &at, FONT_SIZE, &self.font, &color.into());
    }

    pub fn info(&mut self, at: Point2<f32>, msg: &str) {
        self.draw(at, INFO, msg)
    }
    pub fn server(&mut self, at: Point2<f32>, msg: &str) {
        self.draw(at, SERVER, msg)
    }
    pub fn current(&mut self, at: Point2<f32>, msg: &str) {
        self.draw(at, CURRENT, msg)
    }
    pub fn another(&mut self, at: Point2<f32>, msg: &str) {
        self.draw(at, ANOTHER, msg)
    }
}

pub struct Kbps(pub usize);

impl std::fmt::Display for Kbps {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{: >6.1}kbps", bytes_to_kb(self.0))
    }
}

fn bytes_to_kb(bytes: usize) -> f32 {
    ((bytes * 8) as f32) / 1024.0
}
