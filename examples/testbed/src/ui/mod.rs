#![allow(dead_code)]
#![allow(unused_imports)]

use std::{
    rc::Rc,
    time::{Duration, Instant},
    net::SocketAddr,
};
use oni::SimulatedSocket as Socket;
use specs::prelude::*;
use specs::saveload::{Marker, MarkerAllocator};
use kiss2d::{Canvas, Font};
use alga::linear::Transformation;
use nalgebra::{
    UnitComplex,
    Point2,
    Vector2,
    Translation2,
    Isometry2,
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
    font: Rc<Font<'static>>,
    canvas: &'a mut Canvas,
}

impl<'a> Text<'a> {
    pub fn new(canvas: &'a mut Canvas, font: Rc<Font<'static>>) -> Self {
        Self { canvas, font }
    }
    fn draw(&mut self, at: Point2<f32>, color: u32, msg: &str) {
        let at = (at.x, at.y);
        self.canvas.text(&self.font, FONT_SIZE, at, color, msg);
    }
}


#[derive(Clone, Copy)]
pub struct Kbps(pub usize);

impl std::fmt::Display for Kbps {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let b = (self.0 * 8) as f32;
        //let b = self.0 as f32;
        let kb = b / 1000.0;
        let mb = kb / 1000.0;
        let gb = mb / 1000.0;

        if b < 1000.0 {
            write!(f, "{: >6.1} bit/s", b)
        } else if kb < 1000.0 {
            write!(f, "{: >6.1} kbit/s", kb)
        } else if mb < 1000.0 {
            write!(f, "{: >6.1} mbit/s", mb)
        } else {
            write!(f, "{: >6.1} gbit/s", gb)
        }
    }
}
