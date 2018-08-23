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
};

pub const fn duration_to_secs(duration: Duration) -> f32 {
    duration.as_secs() as f32 + (duration.subsec_nanos() as f32 / 1.0e9)
}

pub const fn secs_to_duration(secs: f32) -> Duration {
    let nanos = (secs as u64) * 1_000_000_000 + ((secs % 1.0) * 1.0e9) as u64;
    Duration::from_nanos(nanos)
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
        world.register::<StateBuffer>();
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

    pub fn run(&mut self, win: &mut Window, camera: &FixedView) {
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

        if let Some(me) = self.world.res.try_fetch::<Entity>() {
            let mut actor = self.world.write_storage::<Actor>();
            let mut ai = self.world.res.try_fetch_mut::<AI>();
            if let (Some(actor), Some(ai)) = (actor.get_mut(*me), ai.as_mut()) {
                ai.debug_draw(self.view(win, camera), actor);
            }
        }

        let mut stick = self.world.res.try_fetch_mut::<Stick>();
        if let Some(stick) = stick.as_mut() {
            let mut view = self.view(win, camera);
            let tr = Translation2::from_vector(stick.mouse.coords);
            let color = color(0xCC0000).into();
            view.circ(Isometry2::identity() * tr, 0.2, color);
        }
    }

    pub fn client_bind(&mut self, id: u16) {
        let me: Entity = unsafe { std::mem::transmute((id as u32, 1)) };
        self.world.add_resource(me);
    }

    pub fn client_fire(&mut self, fire: bool) {
        let me: Entity = *self.world.read_resource();
        let mut stick = self.world.res.try_fetch_mut::<Stick>();
        let mut actors = self.world.write_storage::<Actor>();
        if let (Some(a), Some(stick)) = (actors.get_mut(me), stick.as_mut()) {
            a.fire = fire;
            stick.fire(fire);
        }
    }

    pub fn client_mouse(&mut self, win: &mut Window, camera: &FixedView, mouse: Point2<f32>) {
        let mut stick = self.world.res.try_fetch_mut::<Stick>();
        if let Some(stick) = stick.as_mut() {
            let mut view = self.view(win, camera);
            stick.mouse = view.from_screen(mouse).into();
        }
    }

    pub fn client_rotation(&mut self, win: &mut Window, mouse: Point2<f32>, camera: &FixedView)
        -> Option<()>
    {
        let me: Entity = *self.world.read_resource();
        let mut actors = self.world.write_storage::<Actor>();
        let mut stick = self.world.res.try_fetch_mut::<Stick>();

        let stick = stick.as_mut()?;
        let actor = actors.get_mut(me)?;

        let pos: Point2<_> = self.view(win, camera)
            .to_screen(actor.position).into();
        let m = (mouse - pos).normalize();
        let rotation = UnitComplex::from_cos_sin_unchecked(m.x, m.y).angle();
        stick.rotate(rotation);

        Some(())
    }

    pub fn client_wasd(&mut self, key: Key, action: Action) {
        let mut stick = self.world.res.try_fetch_mut::<Stick>();
        if let Some(stick) = stick.as_mut() {
            stick.wasd(key, action);
        }
    }

    pub fn client_arrows(&mut self, key: Key, action: Action) {
        let mut stick = self.world.res.try_fetch_mut::<Stick>();
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
        let world = &mut self.world;
        let me: Entity = *world.read_resource();

        let count = world.read_resource::<Reconciliation>().non_acknowledged();

        let mut status = msg.to_string();
        self.base_status(&mut status);
        status += &format!("\n ID: {}", me.id());
        status += &format!("\n non-acknowledged inputs: {}", count);

        let me: Entity = *self.world.read_resource();
        let actors = self.world.read_storage::<Actor>();
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

        let world = &mut self.world;
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

        // Create a new Entity for self Client.
        let e = self.world.create_entity()
            // TODO .marked::<NetMarker>()
            .with(Conn(addr))
            .with(InputBuffer::new())
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

    fn render_nodes(&mut self, win: &mut Window, camera: &FixedView) {
        let entities = self.world.entities();
        let actors = self.world.read_storage::<Actor>();

        let states = self.world.read_storage::<StateBuffer>();
        let lazy = self.world.read_resource::<LazyUpdate>();
        let mut nodes = self.world.write_storage::<Node>();

        let mut view = self.view(win, camera);

        for (e, _) in (&*entities, !&nodes).join() {
            lazy.insert(e, Node::new());
        }

        for states in (&states).join() {
            let color = color(0xCC0000 | 0x7FDBFF).into();
            for state in states.iter() {
                let iso = state.transform();
                view.rect(iso, 0.15, 0.15, color);
            }
        }

        for (e, a, node) in (&*entities, &actors, &mut nodes).join() {
            let color = if e.id() == 0 { CURRENT } else { ANOTHER };
            let color = color.into();

            let iso = a.transform();

            let gun = iso * Translation2::new(0.15, 0.0);
            view.rect(iso, 0.15, 0.15, color);
            view.rect(gun, 0.15, 0.05, GUN.into());

            if a.fire {
                node.fire_state += 1;
                node.fire_state %= 6;
                let color = if node.fire_state >= 3 { FIRE } else { LAZER };
                view.ray(iso, 2.0, color.into());
            } else {
                node.fire_state = 0;
            }
        }
    }

    fn view<'w, 'c>(&self, win: &'w mut Window, camera: &'c FixedView) -> View<'w, 'c> {
        View::new(win, camera, self.middle)
    }
}

#[derive(Clone, Copy)]
pub struct View<'w, 'c> {
    win: &'w Window,
    camera: &'c FixedView,
    middle: f32,
    size: Vector2<f32>,

    proj: Matrix3<f32>,
    inv_proj: Matrix3<f32>,
}

impl<'w, 'c> View<'w, 'c> {
    fn new(win: &'w Window, camera: &'c FixedView, middle: f32) -> Self {
        let size = win.size();
        let size = Vector2::new(size.x as f32, size.y as f32);

        let hidpi = win.hidpi_factor();
        let diag = Vector3::new(
            2.0 * (hidpi as f32) / size.x,
            2.0 * (hidpi as f32) / size.y,
            1.0,
        );
        let inv_diag = Vector3::new(1.0 / diag.x, 1.0 / diag.y, 1.0);
        let proj = Matrix3::from_diagonal(&diag);
        let inv_proj = Matrix3::from_diagonal(&inv_diag);

        Self { win, camera, middle, size, proj, inv_proj }
    }

    pub fn from_screen(&mut self, mut coord: Point2<f32>) -> Point2<f32> {
        coord.y += self.middle;
        coord.x -= self.size.x * 0.5;
        {
            // proj
        }
        coord.x /= 100.0;
        coord.y /= -100.0;

        coord
        /*
        let normalized_coords = Point2::new(
            2.0 * window_coord.x / self.size.x - 1.0,
            2.0 * -window_coord.y / self.size.y + 1.0,
        );
        let inv_proj: nalgebra::Matrix3<f32> = nalgebra::one();
        let unprojected_hom = inv_proj * normalized_coords.to_homogeneous();
        Point2::from_homogeneous(unprojected_hom).unwrap()
        */
    }

    pub fn to_screen(&mut self, position: Point2<f32>) -> [f32; 2] {
        if false {
            let pos = self.proj * position.to_homogeneous();
            let mut p = Point2::from_homogeneous(pos).unwrap();
            p.x += 1.0;
            p.y -= 1.0;

            p.x *= self.size.x;
            p.y *= self.size.y;

            p.x *= 0.5;
            p.y *= -0.5;

            p.x *= 0.5;
            p.y *= 0.5;

            //p.x -= self.size.x * 0.5;
            //p.y -= self.size.y * 0.5;

            //println!("{}", p);

            [p.x, p.y]
        } else {
            let pos = Point2::new(position.x * 100.0, position.y * -100.0);
            let v = {
                let normalized_coords = Point2::new(
                    2.0 * pos.x / self.size.x - 1.0,
                    2.0 * -pos.y / self.size.y + 1.0,
                );
                let unprojected_hom = self.inv_proj * normalized_coords.to_homogeneous();
                Point2::from_homogeneous(unprojected_hom).unwrap()
            };
            //let v = self.camera.unproject(&pos, &self.size);
            let p = [v.x + self.size.x * 0.5, v.y - self.middle];

            //println!("{:?}", p);
            p
        }
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

    pub fn curve_in<I>(&mut self, iso: Isometry2<f32>, color: Color<f32>, looped: bool, pts: I)
        where I: IntoIterator<Item=Point2<f32>>
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


pub const fn color(c: u32) -> [f32; 3] {
    [
        ((c >> 16) & 0xFF) as f32 / 0xFF as f32,
        ((c >>  8) & 0xFF) as f32 / 0xFF as f32,
        ((c >>  0) & 0xFF) as f32 / 0xFF as f32,
    ]
}
