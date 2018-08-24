#![allow(dead_code)]

use std::time::Duration;
use nalgebra::{Point2, Vector2, dot};

pub const fn duration_to_secs(duration: Duration) -> f32 {
    duration.as_secs() as f32 + (duration.subsec_nanos() as f32 / 1.0e9)
}

pub const fn secs_to_duration(secs: f32) -> Duration {
    let nanos = (secs as u64) * 1_000_000_000 + ((secs % 1.0) * 1.0e9) as u64;
    Duration::from_nanos(nanos)
}

pub struct Segment {
    pub start: Point2<f32>,
    pub end: Point2<f32>,
}

impl Segment {
    pub fn new(start: Point2<f32>, end: Point2<f32>) -> Self {
        Self { start, end }
    }
}

pub struct Circle {
    pub center: Point2<f32>,
    pub radius: f32,
}

impl Circle {
    pub fn new(center: Point2<f32>, radius: f32) -> Self {
        Self { center, radius }
    }
    pub fn raycast(&self, ray: Segment) -> bool {
        let d = ray.end - ray.start;
        let f = ray.start - self.center;

        let a = dot(&d, &d);
        let b = 2.0 * dot(&f, &d);
        let c = dot(&f, &f) - self.radius * self.radius;

        let discriminant = b * b - 4.0 * a * c;
        if discriminant < 0.0 {
            return false;
        }

        let discriminant = discriminant.sqrt();

        let t1 = (-b - discriminant) / (2.0 * a);
        let t2 = (-b + discriminant) / (2.0 * a);

        t1 >= 0.0 && t1 <= 1.0 || t2 >= 0.0 && t2 <= 1.0
    }
}

pub fn dcubic_hermite(p0: f32, v0: f32, p1: f32, v1: f32, t: f32) -> f32 {
    let tt = t * t;
    let dh00 =  6.0 * tt - 6.0 * t;
    let dh10 =  3.0 * tt - 4.0 * t + 1.0;
    let dh01 = -6.0 * tt + 6.0 * t;
    let dh11 =  3.0 * tt - 2.0 * t;

    dh00 * p0 + dh10 * v0 + dh01 * p1 + dh11 * v1
}

pub fn cubic_hermite(p0: f32, v0: f32, p1: f32, v1: f32, t: f32) -> f32 {
    let ti = t - 1.0;
    let t2 = t * t;
    let ti2 = ti * ti;
    let h00 = (1.0 + 2.0 * t) * ti2;
    let h10 = t * ti2;
    let h01 = t2 * (3.0 - 2.0 * t);
    let h11 = t2 * ti;

    h00 * p0 + h10 * v0 + h01 * p1 + h11 * v1
}

pub fn hermite2(p0: Point2<f32>, v0: Vector2<f32>, p1: Point2<f32>, v1: Vector2<f32>, t: f32) -> Point2<f32> {
    let x = cubic_hermite(p0.x, v0.x, p1.x, v1.x, t);
    let y = cubic_hermite(p0.y, v0.y, p1.y, v1.y, t);
    Point2::new(x, y)
}

pub const fn color(c: u32) -> [f32; 3] {
    let c = c.to_le();
    [
        ((c >> 16) & 0xFF) as f32 / 0xFF as f32,
        ((c >>  8) & 0xFF) as f32 / 0xFF as f32,
        ((c >>  0) & 0xFF) as f32 / 0xFF as f32,
    ]
}
