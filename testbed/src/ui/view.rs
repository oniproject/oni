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

#[derive(Clone, Copy)]
pub struct View<'w, 'c> {
    win: &'w Window,
    camera: &'c FixedView,
    size: Vector2<f32>,

    start: f32,
    height: f32,
    middle: f32,
}

impl<'w, 'c> View<'w, 'c> {
    pub fn new(win: &'w Window, camera: &'c FixedView, start: f32, height: f32) -> Self {
        let size = win.size();
        let size = Vector2::new(size.x as f32, size.y as f32);

        let middle = start + height / 2.0;
        Self { win, camera, size, start, height, middle }
    }

    pub fn from_screen(&mut self, mut coord: Point2<f32>) -> [f32; 2] {
        coord.y += self.size.y / 2.0;
        coord.y -= self.start + self.height / 2.0;

        let coord = self.camera.unproject(&coord, &self.size);

        let v = coord / 60.0;
        [v.x, v.y]
    }

    pub fn to_screen(&mut self, position: Point2<f32>) -> [f32; 2] {
        let middle = self.start + self.height / 2.0;
        let middle = Point2::new(0.0, middle);
        let middle = self.camera.unproject(&middle, &self.size).y;

        let v = position.coords * 60.0;
        [v.x , v.y + middle]
    }

    pub fn line(&mut self, a: Point2<f32>, b: Point2<f32>, color: Color<f32>) {
        let a = self.to_screen(a).into();
        let b = self.to_screen(b).into();
        unsafe {
            let win: &mut Window = &mut *(self.win as *const _ as *mut _);
            win.draw_planar_line(&a, &b, &color)
        }
    }

    pub fn ray(&mut self, iso: Isometry2<f32>, len: f32, color: Color<f32>) {
        self.ray_to(iso, Point2::new(len, 0.0), color);
    }

    pub fn ray_to(&mut self, iso: Isometry2<f32>, to: Point2<f32>, color: Color<f32>) {
        let a = iso.transform_point(&Point2::new(0.0, 0.0));
        let b = iso.transform_point(&to);
        self.line(a, b, color.into());
    }

    pub fn circ(&mut self, iso: Isometry2<f32>, radius: f32, color: Color<f32>) {
        use std::f32::consts::PI;
        let nsamples = 16;

        for i in 0..nsamples {
            let a = ((i + 0) as f32) / (nsamples as f32) * PI * 2.0;
            let b = ((i + 1) as f32) / (nsamples as f32) * PI * 2.0;

            let a = Point2::new(a.cos(), a.sin()) * radius;
            let b = Point2::new(b.cos(), b.sin()) * radius;

            let a = iso.transform_point(&a);
            let b = iso.transform_point(&b);

            self.line(a, b, color);
        }
    }

    pub fn curve<I>(&mut self, color: Color<f32>, pts: I)
        where I: IntoIterator<Item=Point2<f32>>
    {
        let mut pts = pts.into_iter();
        let first = if let Some(p) = pts.next() { p } else { return };

        let mut base = first;
        for p in pts {
            self.line(base, p, color);
            base = p;
        }
    }

    pub fn curve_loop<I>(&mut self, color: Color<f32>, pts: I)
        where I: IntoIterator<Item=Point2<f32>>
    {
        let mut pts = pts.into_iter();
        let first = if let Some(p) = pts.next() { p } else { return };

        let mut base = first;
        for p in pts {
            self.line(base, p, color);
            base = p;
        }

        self.line(base, first, color);
    }

    pub fn curve_in<'a, I>(&mut self, iso: Isometry2<f32>, color: Color<f32>, looped: bool, pts: I)
        where I: IntoIterator<Item=&'a Point2<f32>>
    {
        let mut pts = pts.into_iter();
        let first = if let Some(p) = pts.next() { p } else { return };

        let mut base = first;
        for p in pts {
            let a = iso.transform_point(&base);
            let b = iso.transform_point(&p);
            self.line(a, b, color);
            base = p;
        }

        if looped {
            let a = iso.transform_point(&base);
            let b = iso.transform_point(&first);
            self.line(a, b, color);
        }
    }

    pub fn rect(&mut self, iso: Isometry2<f32>, w: f32, h: f32, color: Color<f32>) {
        self.rect_lines(iso, w, h, color, &[
            (0, 2), (0, 3),
            (1, 2), (1, 3),
        ]);
    }

    pub fn rect_x(&mut self, iso: Isometry2<f32>, w: f32, h: f32, color: Color<f32>) {
        self.rect_lines(iso, w, h, color, &[
            (0, 1), (2, 3),

            (0, 2), (0, 3),
            (1, 2), (1, 3),
        ]);
    }

    pub fn x(&mut self, iso: Isometry2<f32>, w: f32, h: f32, color: Color<f32>) {
        self.rect_lines(iso, w, h, color, &[
            (0, 1), (2, 3),
        ]);
    }

    fn rect_lines(
        &mut self,
        iso: Isometry2<f32>,
        w: f32, h: f32,
        color: Color<f32>,
        lines: &[(usize, usize)])
    {
        let p = [
            Point2::new(-w, -h),
            Point2::new( w,  h),
            Point2::new(-w,  h),
            Point2::new( w, -h),
        ];

        for &(n, m) in lines.iter() {
            let a = iso.transform_point(&p[n]);
            let b = iso.transform_point(&p[m]);
            self.line(a, b, color);
        }
    }
}
