use std::time::{Duration, Instant};
use specs::prelude::*;
use specs::saveload::Marker;
use kiss2d::Canvas;
use nalgebra::{
    UnitComplex,
    Point2,
    Vector2,
    Translation2,
    Isometry2,
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

    pub fn dispatch(&mut self, now: Instant) {
        let dt = secs_to_duration(1.0 / self.update_rate);

        if self.time + dt <= now {
            self.time += dt;
            self.dispatcher.dispatch();
            self.dispatched = true;
        }
    }

    pub fn run(&mut self, win: &mut Canvas) {
        {
            oni_trace::scope![wait];
            if self.dispatched {
                self.dispatched = false;
                //self.dispatcher.wait();
                self.dispatcher.mut_res().maintain();
            }
        }

        if self.second <= Instant::now() {
            self.second += Duration::from_secs(1);
            let res = &mut self.dispatcher.mut_res().res;
            let client = res.try_fetch::<oni::Client<oni::SimulatedSocket>>();
            let server = res.try_fetch::<oni::Server<oni::SimulatedSocket>>();
            match (client, server) {
                (Some(client), None)  => {
                    let socket = client.socket();
                    self.recv = Kbps(socket.take_recv_bytes());
                    self.send = Kbps(socket.take_send_bytes());
                }
                (None, Some(server)) => {
                    let socket = server.socket();
                    self.recv = Kbps(socket.take_recv_bytes());
                    self.send = Kbps(socket.take_send_bytes());
                }
                _ => unreachable!(),
            }
        }

        {
            let mut view = self.view(win);
            let world = self.dispatcher.mut_res();

            if let Some(me) = world.read_resource::<NetNode>().me() {
                if let Some(actor) = world.write_storage::<Actor>().get_mut(me) {
                    if let Some(ai) = world.res.try_fetch_mut::<AI>().as_mut() {
                        ai.debug_draw(view, actor);
                    }
                }
            }

            if let Some(stick) = world.res.try_fetch_mut::<Stick>().as_mut() {
                let mouse = stick.get_mouse().coords;
                let mouse = mouse + Vector2::new(-0.01, 0.01);
                let tr = Translation2::from_vector(mouse);
                let color = RED.into();
                view.x(Isometry2::identity() * tr, 0.04, 0.04, color);
            }
        }

        self.render_nodes(win);
    }

    pub fn client_fire(&mut self, fire: bool) {
        if let Some(stick) = self.dispatcher.mut_res().res.try_fetch_mut::<Stick>().as_mut() {
            stick.fire(fire);
        }
    }

    pub fn client_mouse(&mut self, win: &mut Canvas, mouse: Point2<f32>) {
        let mut view = self.view(win);
        if let Some(stick) = self.dispatcher.mut_res().res.try_fetch_mut::<Stick>().as_mut() {
            stick.mouse(view.from_screen(mouse).into());
        }
    }

    pub fn client_wasd(&mut self, canvas: &mut Canvas) {
        let mut stick = self.dispatcher.mut_res().res.try_fetch_mut::<Stick>();
        if let Some(stick) = stick.as_mut() {
            stick.wasd(canvas);
        }
    }

    pub fn client_status(&mut self, text: &mut Text, color: u32, msg: &str) {
        let mut status = msg.to_string();
        status += &format!("{: >3} fps", self.update_rate);

        let world = self.dispatcher.mut_res();

        if let Some(me) = world.read_resource::<NetNode>().me() {
            let count = world.read_resource::<Reconciliation>().non_acknowledged();

            status += &format!("  #{}", me.id());
            status += &format!("\n non-acknowledged inputs: {}", count);

            let actors = world.read_storage::<Actor>();
            if let Some(actor) = actors.get(me) {
                status += &format!("\n pos: {}", actor.position);
            }
        } else {
            status += "\n DISCONNECTED";
        }

        let at = Point2::new(10.0, self.start);
        status += &format!("\n recv: {}\n send: {}", self.recv, self.send);
        text.draw(at, color, &status);
    }

    pub fn server_status(&mut self, text: &mut Text, color: u32) {
        let count = {
            let world = &mut self.dispatcher.mut_res();
            let clients = world.read_storage::<InputBuffer>();
            (&clients).join().count()
        };

        let at = Point2::new(10.0, self.start);
        let s = format!("Server {: >3} fps\n connected: {}\n recv: {}\n send: {}",
                self.update_rate, count, self.recv, self.send);
        text.draw(at, color, &s);

        /*
        status += "\n Last acknowledged input:";
        let clients = (&clients).join().map(|c| c.seq);
        for (i, last_processed_input) in clients.enumerate() {
            let lpi: u8 = last_processed_input.into();
            if false {
                status += &format!("\n  [{}: #{:0>2X}]", i, lpi);
            }
        }
        */
    }

    pub fn update_view(&mut self, start: f32, height: f32) {
        self.start = start;
        self.height = height;
        self.middle = start + height / 2.0;
    }

    fn render_nodes(&mut self, win: &mut Canvas) {
        let mut view = self.view(win);
        let world = self.dispatcher.mut_res();
        let entities = world.entities();
        let actors = world.read_storage::<Actor>();
        let mark = world.read_storage::<NetMarker>();

        let states = world.read_storage::<StateBuffer>();
        let lazy = world.read_resource::<LazyUpdate>();
        let mut nodes = world.write_storage::<Node>();

        for (e, _) in (&*entities, !&nodes).join() {
            lazy.insert(e, Node::new());
        }

        // draw history
        for states in (&states).join() {
            view.curve(MAROON, states.iter().map(|s| s.position));
            /*
            for state in states.iter() {
                draw_body(&mut view, state.transform(), MAROON);
            }
            */
        }

        for (mark, a, node) in (&mark, &actors, &mut nodes).join() {
            let iso = a.transform();

            let color = match (a.damage, mark.id()) {
                (true, _) => RED,
                (false, 0) => CURRENT,
                (false, 1) => ANOTHER,
                (false, _) => OTHERS,
            };

            //view.circ(iso, FIRE_RADIUS, MAROON.into());
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

    fn view<'w>(&self, win: &'w mut Canvas) -> View<'w> {
        View::new(win, self.start, self.height)
    }
}

fn draw_body<'w>(view: &mut View<'w>, iso: Isometry2<f32>, color: u32) {
    use std::f32::consts::FRAC_PI_2;
    let iso = iso * UnitComplex::from_angle(-FRAC_PI_2);
    view.curve_in(iso, color, true, &[
        Point2::new(0.0, 0.20),
        Point2::new(0.15, -0.10),
        Point2::new(0.0, 0.0),
        Point2::new(-0.15, -0.10),
    ]);
}
