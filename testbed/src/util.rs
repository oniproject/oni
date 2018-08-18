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
use nalgebra::{Point2, Vector2, Translation2, UnitComplex};
use crate::{
    components::*,
    input::*,
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
    pub fn new(update_rate: f32, mut world: World, dispatcher: DispatcherBuilder<'static, 'static>) -> Self {
        world.register::<Node>();
        let dispatcher = dispatcher.build();
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

    pub fn run<C>(&mut self, win: &mut Window, camera: &C)
        where C: PlanarCamera
    {
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

        self.render_nodes(win, camera);
    }

    pub fn client_bind(&mut self, id: u16) {
        let me: Entity = unsafe { std::mem::transmute((id as u32, 1)) };
        self.world.add_resource(me);
    }

    pub fn client_fire(&mut self, fire: bool) {
        let me: Entity = *self.world.read_resource();
        let mut actors = self.world.write_storage::<Node>();
        if let Some(node) = actors.get_mut(me) {
            node.fire = fire
        }
    }

    pub fn client_rotation<C>(&mut self, win: &mut Window, mouse: Point2<f32>, camera: &C)
        -> Option<()>
        where C: PlanarCamera
    {
        let me: Entity = *self.world.read_resource();
        let mut actors = self.world.write_storage::<Actor>();
        let mut stick = self.world.write_resource::<Option<Stick>>();

        let stick = stick.as_mut()?;
        let actor = actors.get_mut(me)?;

        let pos = self.to_screen(win, camera, actor.position);
        let m = (mouse - pos.vector).coords.normalize();
        let rotation = UnitComplex::from_cos_sin_unchecked(m.x, m.y).angle();
        stick.rotate(rotation);

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

    pub fn update_view(&mut self, start: f32, height: f32) {
        self.start = start;
        self.middle = start + height / 2.0;
        self.end = start + height;
    }

    fn render_nodes<C>(&mut self, win: &mut Window, camera: &C)
        where C: PlanarCamera
    {
        let entities = self.world.entities();
        let actors = self.world.read_storage::<Actor>();
        let lazy = self.world.read_resource::<LazyUpdate>();
        let mut nodes = self.world.write_storage::<Node>();

        for (e, a, _) in (&*entities, &actors, !&nodes).join() {
            let color = if e.id() == 0 { CURRENT } else { ANOTHER };
            let mut node = Node::new(win, color);
            let pos = self.to_screen(win, camera, a.position);
            node.root.set_local_translation(pos);
            node.root.set_local_rotation(a.rotation);
            lazy.insert(e, node);
        }

        for (a, node) in (&actors, &mut nodes).join() {
            let pos = self.to_screen(win, camera, a.position);

            node.root.set_local_translation(pos);
            node.root.set_local_rotation(a.rotation);

            node.lazer.set_visible(node.fire);
            if node.fire {
                node.fire_state += 1;
                node.fire_state %= 6;
                if node.fire_state >= 3 {
                    node.lazer.set_color(FIRE[0], FIRE[1], FIRE[2]);
                } else {
                    node.lazer.set_color(LAZER[0], LAZER[1], LAZER[2]);
                }
            } else {
                node.fire_state = 0;
                node.lazer.set_color(LAZER[0], LAZER[1], LAZER[2]);
            }
        }
    }

    fn to_screen<C>(&self, win: &mut Window, _camera: &C, position: Point2<f32>) -> Translation2<f32>
        where C: PlanarCamera
    {
        let (w, h) = (win.width() as f32, win.height() as f32);
        let x = (position.x / 10.0) * w - w * 0.0;
        let y = (position.y / 10.0) * h + h * 0.5;
        let y = y - self.middle * ACTOR_RADIUS;
        Translation2::new(x, y)
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
