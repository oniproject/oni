#![allow(dead_code)]
#![allow(unused_imports)]

use std::{
    rc::Rc,
    time::{Duration, Instant},
    net::SocketAddr,
};
use oni::simulator::Socket;
use specs::prelude::*;
use specs::saveload::{Marker, MarkerAllocator};
use kiss3d::{
    window::Window,
    text::Font,
    planar_camera::{PlanarCamera, FixedView},
    event::{Action, Key},
};
use alga::linear::Transformation;
use nalgebra::{
    UnitComplex,
    Point2,
    Vector2,
    Translation2,
    Isometry2,
    Point3 as Color,

    Matrix3, Vector3,
};
use crate::{
    ai::*,
    components::*,
    input::*,
    client::*,
    consts::*,
};





mod app;
mod demo;
mod view;

pub use self::app::AppState;
pub use self::demo::Demo;
pub use self::view::View;

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
}


#[derive(Clone, Copy)]
pub struct Kbps(pub usize);

impl std::fmt::Display for Kbps {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "{: >6.1} kbit/s", bytes_to_kb(self.0))
    }
}

fn bytes_to_kb(bytes: usize) -> f32 {
    ((bytes * 8) as f32) / 1000.0
}
