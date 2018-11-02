use kiss2d::Canvas;
use alga::linear::Transformation;
use nalgebra::{
    Point2,
    Vector2,
    Isometry2,
};
use crate::consts::*;

#[derive(Clone, Copy)]
pub struct View<'w> {
    win: &'w Canvas,
    midpoint: Vector2<f32>,
}

impl<'w> View<'w> {
    pub fn new(win: &'w Canvas, start: f32, height: f32) -> Self {
        let w = win.size().0 as f32;
        let midpoint = Vector2::new(w / 2.0, start + height / 2.0);
        Self { win, midpoint }
    }

    pub fn from_screen(&mut self, coord: Point2<f32>) -> [f32; 2] {
        let v = (coord - self.midpoint) / VIEW_SCALE;
        [v.x, v.y]
    }

    pub fn to_screen(&mut self, position: Point2<f32>) -> [f32; 2] {
        let v = (position.coords * VIEW_SCALE) + self.midpoint;
        [v.x, v.y]
    }

    pub fn line(&mut self, a: Point2<f32>, b: Point2<f32>, color: u32) {
        let a = self.to_screen(a);
        let b = self.to_screen(b);
        let a = (a[0] as isize, a[1] as isize);
        let b = (b[0] as isize, b[1] as isize);
        unsafe {
            // XXX: because
            let win: &mut Canvas = &mut *(self.win as *const _ as *mut _);
            win.line(a, b, color)
        }
    }

    pub fn ray(&mut self, iso: Isometry2<f32>, len: f32, color: u32) {
        self.ray_to(iso, Point2::new(len, 0.0), color);
    }

    pub fn ray_to(&mut self, iso: Isometry2<f32>, to: Point2<f32>, color: u32) {
        let a = iso.transform_point(&Point2::new(0.0, 0.0));
        let b = iso.transform_point(&to);
        self.line(a, b, color);
    }

    pub fn circ(&mut self, iso: Isometry2<f32>, radius: f32, color: u32) {
        use std::f32::consts::PI;
        let nsamples = 16;

        for i in 0..nsamples {
            let a = ( i      as f32) / (nsamples as f32) * PI * 2.0;
            let b = ((i + 1) as f32) / (nsamples as f32) * PI * 2.0;

            let a = Point2::new(a.cos(), a.sin()) * radius;
            let b = Point2::new(b.cos(), b.sin()) * radius;

            let a = iso.transform_point(&a);
            let b = iso.transform_point(&b);

            self.line(a, b, color);
        }
    }

    pub fn curve<I>(&mut self, color: u32, pts: I)
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

    pub fn curve_loop<I>(&mut self, color: u32, pts: I)
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

    pub fn curve_in<'a, I>(&mut self, iso: Isometry2<f32>, color: u32, looped: bool, pts: I)
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

    pub fn rect(&mut self, iso: Isometry2<f32>, w: f32, h: f32, color: u32) {
        self.rect_lines(iso, w, h, color, &[
            (0, 2), (0, 3),
            (1, 2), (1, 3),
        ]);
    }

    pub fn rect_x(&mut self, iso: Isometry2<f32>, w: f32, h: f32, color: u32) {
        self.rect_lines(iso, w, h, color, &[
            (0, 1), (2, 3),

            (0, 2), (0, 3),
            (1, 2), (1, 3),
        ]);
    }

    pub fn x(&mut self, iso: Isometry2<f32>, w: f32, h: f32, color: u32) {
        self.rect_lines(iso, w, h, color, &[
            (0, 1), (2, 3),
        ]);
    }

    fn rect_lines(
        &mut self,
        iso: Isometry2<f32>,
        w: f32, h: f32,
        color: u32,
        lines: &[(usize, usize)])
    {
        let points = [
            Point2::new(-w, -h),
            Point2::new( w,  h),
            Point2::new(-w,  h),
            Point2::new( w, -h),
        ];

        for &(n, m) in lines.iter() {
            let a = iso.transform_point(&points[n]);
            let b = iso.transform_point(&points[m]);
            self.line(a, b, color);
        }
    }
}
