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
    util::*,
};

use super::{View, Text, Kbps};


pub struct Demo {
    pub dispatcher: AsyncDispatcher<'static, World>,
    pub dispatched: bool,
    pub time: Instant,
    pub update_rate: f32,

    pub start: f32,
    pub height: f32,
    pub middle: f32,

    pub second: Instant,
    pub recv: Kbps,
    pub send: Kbps,

    pub spawn_idx: usize,
}

impl Demo {
    pub fn new(update_rate: f32, mut world: World, dispatcher: DispatcherBuilder<'static, 'static>) -> Self {
        world.register::<Node>();
        let dispatcher = dispatcher.build_async(world);
        Self {
            dispatcher,
            dispatched: false,
            time: Instant::now(),
            update_rate,

            start: 0.0,
            middle: 0.0,
            height: 0.0,

            second: Instant::now(),
            recv: Kbps(0),
            send: Kbps(0),

            spawn_idx: 0,
        }
    }

    pub fn dispatch(&mut self) {
        let now = Instant::now();
        let dt = secs_to_duration(1.0 / self.update_rate);

        if self.time + dt <= now {
            self.time += dt;
            self.dispatcher.dispatch();
            self.dispatched = true;
        }
    }

    pub fn run(&mut self, win: &mut Window, camera: &FixedView) {
        {
            oni::trace::scope_force![wait];
            if self.dispatched {
                self.dispatched = false;
                self.dispatcher.wait();
                self.dispatcher.mut_res().maintain();
            }
        }

        if self.second <= Instant::now() {
            self.second += Duration::from_secs(1);
            let socket = self.dispatcher.mut_res().read_resource::<Socket>();
            self.recv = Kbps(socket.take_recv_bytes());
            self.send = Kbps(socket.take_send_bytes());
        }

        {
            let view = self.view(win, camera);
            let world = self.dispatcher.mut_res();
            for me in world.res.try_fetch::<Entity>() {
                for actor in world.write_storage::<Actor>().get_mut(*me) {
                    for ai in world.res.try_fetch_mut::<AI>().as_mut() {
                        ai.debug_draw(view, actor);
                    }
                }
            }
        }

        {
            let mut view = self.view(win, camera);
            for stick in self.dispatcher.mut_res().res.try_fetch_mut::<Stick>().as_mut() {
                let mouse = stick.get_mouse().coords;
                let mouse = mouse + Vector2::new(-0.01, 0.01);
                let tr = Translation2::from_vector(mouse);
                let color = RED.into();
                view.x(Isometry2::identity() * tr, 0.04, 0.04, color);
            }
        }

        self.render_nodes(win, camera);
    }

    pub fn client_bind(&mut self, id: u16) {
        let me: Entity = unsafe { std::mem::transmute((id as u32, 1)) };
        self.dispatcher.mut_res().add_resource(me);
    }

    pub fn client_fire(&mut self, fire: bool) {
        for stick in self.dispatcher.mut_res().res.try_fetch_mut::<Stick>().as_mut() {
            stick.fire(fire);
        }
    }

    pub fn client_mouse(&mut self, win: &mut Window, camera: &FixedView, mouse: Point2<f32>) {
        let mut view = self.view(win, camera);
        for stick in self.dispatcher.mut_res().res.try_fetch_mut::<Stick>().as_mut() {
            stick.mouse(view.from_screen(mouse).into());
        }
    }

    pub fn client_wasd(&mut self, key: Key, action: Action) {
        let mut stick = self.dispatcher.mut_res().res.try_fetch_mut::<Stick>();
        if let Some(stick) = stick.as_mut() {
            stick.wasd(key, action);
        }
    }

    pub fn client_arrows(&mut self, key: Key, action: Action) {
        let mut stick = self.dispatcher.mut_res().res.try_fetch_mut::<Stick>();
        if let Some(stick) = stick.as_mut() {
            stick.arrows(key, action);
        }
    }

    fn base_status(&mut self, status: &mut String) {
        *status += &format!("\n recv bitrate: {}", self.recv);
        *status += &format!("\n send bitrate: {}", self.send);
        *status += &format!("\n update  rate: {: >5} fps", self.update_rate);
    }

    pub fn client_status(&mut self, text: &mut Text, color: [f32; 3], msg: &str) {
        let world = &mut self.dispatcher.mut_res();
        let me: Entity = *world.read_resource();

        let count = world.read_resource::<Reconciliation>().non_acknowledged();

        let mut status = msg.to_string();
        self.base_status(&mut status);
        status += &format!("\n ID: {}", me.id());
        status += &format!("\n non-acknowledged inputs: {}", count);

        let me: Entity = *self.dispatcher.mut_res().read_resource();
        let actors = self.dispatcher.mut_res().read_storage::<Actor>();
        if let Some(actor) = actors.get(me) {
            status += &format!("\n pos: {}", actor.position);
        }

        let at = Point2::new(10.0, self.start * 2.0);
        text.draw(at, color, &status);
    }

    pub fn server_status(&mut self, text: &mut Text, color: [f32; 3]) {
        let mut status = "Server".to_string();
        self.base_status(&mut status);
        status += "\n Last acknowledged input:";

        let world = &mut self.dispatcher.mut_res();
        let clients = world.read_storage::<InputBuffer>();
        let clients = (&clients).join().map(|c| c.seq);
        for (i, last_processed_input) in clients.enumerate() {
            let lpi: u8 = last_processed_input.into();
            status += &format!("\n  [{}: #{:0>2X}]", i, lpi);
        }

        let at = Point2::new(10.0, self.start * 2.0);
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

        let world = self.dispatcher.mut_res();

        // Create a new Entity for self Client.
        let e = world.create_entity()
            // TODO .marked::<NetMarker>()
            .with(Conn::new(addr))
            .with(InputBuffer::new())
            .with(StateBuffer::new())
            .with(Actor::spawn(pos))
            .build();

        let mut alloc = world.write_resource::<NetNode>();
        alloc.by_addr.insert(addr, e);
        let storage = &mut world.write_storage::<NetMarker>();
        let e = alloc.mark(e, storage).unwrap();

        assert!(e.1);
        e.0.id()
    }

    pub fn update_view(&mut self, start: f32, height: f32) {
        self.start = start;
        self.height = height;
        self.middle = start + height / 2.0;
    }

    fn render_nodes(&mut self, win: &mut Window, camera: &FixedView) {
        oni::trace::scope_force![render nodes];

        let mut view = self.view(win, camera);
        let world = self.dispatcher.mut_res();
        let entities = world.entities();
        let actors = world.read_storage::<Actor>();

        let states = world.read_storage::<StateBuffer>();
        let lazy = world.read_resource::<LazyUpdate>();
        let mut nodes = world.write_storage::<Node>();

        for (e, _) in (&*entities, !&nodes).join() {
            lazy.insert(e, Node::new());
        }

        for states in (&states).join() {
            for state in states.iter() {
                draw_body(&mut view, state.transform(), MAROON);
            }
        }

        for (e, a, node) in (&*entities, &actors, &mut nodes).join() {
            let iso = a.transform();

            let color = if a.damage { RED } else if e.id() == 0 { CURRENT } else { ANOTHER };
            view.circ(iso, FIRE_RADIUS, MAROON.into());
            draw_body(&mut view, iso, color);

            if a.fire {
                node.fire_state += 1;
                node.fire_state %= 6;
                let color = if node.fire_state >= 3 { FIRE } else { LAZER };
                view.ray(iso, FIRE_LEN, color.into());
            } else {
                node.fire_state = 0;
            }
        }
    }

    fn view<'w, 'c>(&self, win: &'w mut Window, camera: &'c FixedView) -> View<'w, 'c> {
        View::new(win, camera, self.start, self.height)
    }
}

fn draw_body<'w, 'c>(view: &mut View<'w, 'c>, iso: Isometry2<f32>, color: [f32; 3]) {
    use std::f32::consts::FRAC_PI_2;
    let iso = iso * UnitComplex::from_angle(-FRAC_PI_2);
    view.curve_in(iso, color.into(), true, &[
        Point2::new(0.0, 0.20),
        Point2::new(0.15, -0.10),
        Point2::new(0.0, 0.0),
        Point2::new(-0.15, -0.10),
    ]);
}
