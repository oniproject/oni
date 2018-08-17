#![allow(dead_code)]

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
    planar_camera::PlanarCamera,
    event::{Action, Key},
};
use nalgebra::{Point2, Vector2, UnitComplex};
use crate::{
    net_marker::*,
    actor::*,
    input::*,
    server::*,
    client::*,
    consts::*,
};

pub fn duration_to_secs(duration: Duration) -> f32 {
    duration.as_secs() as f32 + (duration.subsec_nanos() as f32 / 1.0e9)
}

pub fn secs_to_duration(secs: f32) -> Duration {
    Duration::new(secs as u64, ((secs % 1.0) * 1.0e9) as u32)
}

pub struct Demo {
    pub world: World,
    pub dispatcher: Dispatcher<'static, 'static>,
    pub time: Instant,
    pub update_rate: f32,

    pub start: f32,
    pub middle: f32,
    pub end: f32,

    pub second: Instant,
    pub recv: Kbps,
    pub send: Kbps,

    pub spawn_idx: usize,
}

impl Demo {
    pub fn new(update_rate: f32, world: World, dispatcher: Dispatcher<'static, 'static>) -> Self {
        Self {
            world, dispatcher,
            time: Instant::now(),
            update_rate,

            start: 0.0,
            middle: 0.0,
            end: 0.0,

            second: Instant::now(),
            recv: Kbps(0),
            send: Kbps(0),

            spawn_idx: 0,
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let dt = secs_to_duration(1.0 / self.update_rate);

        if self.time + dt <= now {
            self.time += dt;
            self.dispatcher.dispatch(&mut self.world.res);
            self.world.maintain();
        }

        if self.second <= Instant::now() {
            self.second += Duration::from_secs(1);
            let socket = self.world.read_resource::<Socket>();
            self.recv = Kbps(socket.take_recv_bytes());
            self.send = Kbps(socket.take_send_bytes());
        }
    }

    pub fn client_bind(&mut self, id: u16) {
        let me: Entity = unsafe { std::mem::transmute((id as u32, 1)) };
        self.world.add_resource(me);
    }

    pub fn client_fire(&mut self, fire: bool) {
        let me: Entity = *self.world.read_resource();
        let mut actors = self.world.write_storage::<Actor>();
        if let Some(node) = actors.get_mut(me).and_then(|e| e.node.as_mut()) {
            node.fire = fire
        }
    }

    pub fn client_rotation(&mut self, win: &mut Window, mouse: Point2<f32>) -> Option<()> {
        let me: Entity = *self.world.read_resource();
        let mut actors = self.world.write_storage::<Actor>();
        let mut stick = self.world.write_resource::<Option<Stick>>();

        let stick = stick.as_mut()?;
        let actor = actors.get_mut(me)?;

        let mut pos = position_to_screen(win, actor.position);
        pos.y -= self.middle * ACTOR_RADIUS;
        let m = (mouse - pos).normalize();
        actor.rotation = UnitComplex::from_cos_sin_unchecked(m.x, m.y);
        stick.rotate(actor.rotation.angle());

        Some(())
    }

    pub fn client_wasd(&mut self, key: Key, action: Action) {
        let mut stick = self.world.write_resource::<Option<Stick>>();
        if let Some(stick) = stick.as_mut() {
            stick.wasd(key, action);
        }
    }

    pub fn client_arrows(&mut self, key: Key, action: Action) {
        let mut stick = self.world.write_resource::<Option<Stick>>();
        if let Some(stick) = stick.as_mut() {
            stick.arrows(key, action);
        }
    }

    pub fn client_status(&mut self, text: &mut Text, color: [f32; 3], msg: &str) {
        let world = &mut self.world;
        let me: Entity = *world.read_resource();

        let count = world.read_resource::<Reconciliation>().non_acknowledged();

        let mut status = msg.to_string();
        status += &format!("\n recv bitrate: {}", self.recv);
        status += &format!("\n send bitrate: {}", self.send);
        status += &format!("\n update  rate: {: >5} fps", self.update_rate);
        status += &format!("\n ID: {}", me.id());
        status += &format!("\n Non-acknowledged inputs: {}", count);

        let at = Point2::new(10.0, FONT_SIZE * self.start);
        text.draw(at, color, &status);
    }

    pub fn server_status(&mut self, text: &mut Text, color: [f32; 3]) {
        let world = &mut self.world;
        let clients = world.read_storage::<LastProcessedInput>();
        let clients = (&clients).join().map(|c| c.0);

        let mut status = "Server".to_string();
        status += &format!("\n recv bitrate: {}", self.recv);
        status += &format!("\n send bitrate: {}", self.send);
        status += &format!("\n update  rate: {: >5} fps", self.update_rate);
        status += "\n Last acknowledged input:";
        for (i, last_processed_input) in clients.enumerate() {
            let lpi: u8 = last_processed_input.into();
            status += &format!("\n  [{}: #{:0>2X}]", i, lpi);
        }

        let at = Point2::new(10.0, FONT_SIZE * self.start);
        text.draw(at, color, &status);
    }

    pub fn server_connect(&mut self, addr: SocketAddr) -> u16 {
        // Set the initial state of the Entity (e.g. spawn point)
        let spawn_points = [
            Point2::new(-3.0, 0.0),
            Point2::new( 3.0, 0.0),
        ];

        let pos = spawn_points[self.spawn_idx];
        self.spawn_idx += 1;

        // Create a new Entity for self Client.
        let e = self.world.create_entity()
            // TODO .marked::<NetMarker>()
            .with(Conn(addr))
            .with(LastProcessedInput(0.into()))
            .with(Actor::spawn(pos))
            .build();

        let mut alloc = self.world.write_resource::<NetNode>();
        alloc.by_addr.insert(addr, e);
        let storage = &mut self.world.write_storage::<NetMarker>();
        let e = alloc.mark(e, storage).unwrap();

        assert!(e.1);
        e.0.id()
    }

    pub fn render_nodes<C>(&mut self, win: &mut Window, camera: &C)
        where C: PlanarCamera
    {
        let e = self.world.entities();
        let mut a = self.world.write_storage::<Actor>();
        for (e, a) in (&*e, &mut a).join() {
            a.render(win, self.middle, e.id(), camera)
        }
    }

    pub fn update_view(&mut self, start: f32, height: f32) {
        self.start = start;
        self.middle = start + height / 2.0;
        self.end = start + height;
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
