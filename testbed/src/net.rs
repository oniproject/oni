use std::{
    rc::Rc,
    time::{Instant, Duration},
};
use kiss3d::{
    window::{State, Window},
    text::Font,
    event::{Action, WindowEvent, Key, MouseButton},
    scene::PlanarSceneNode,
    camera::{Camera, FixedView},
    planar_camera::PlanarCamera,
    post_processing::PostProcessingEffect,
};
use nalgebra::{
    Translation2,
    UnitComplex,
    Point2,
    Point3,
    Point3 as Color,
};
use crate::{
    actor::Actor,
    client::Client,
    server::Server,
    lag::LagNetwork,
    util::*,
    consts::*,
};

#[derive(Clone, Copy)]
pub struct Input {
    pub press_time: f32,
    pub sequence: usize,
    pub entity_id: usize,
}

#[derive(Clone, Copy)]
pub struct WorldState {
    pub entity_id: usize,
    pub position: Point2<f32>,
    pub last_processed_input: usize,
}

pub struct Simulator {
    font: Rc<Font>,
    player1: (Client, Instant),
    player2: (Client, Instant),
    server: (Server, Instant),

    ch: (LagNetwork<Input>, LagNetwork<Vec<WorldState>>, LagNetwork<Vec<WorldState>>),
    ch_t: (usize, usize, usize),

    camera: FixedView,

    second: Instant,

    mouse: PlanarSceneNode,
    mouse_x: f64,
    mouse_y: f64,
}

impl Simulator {
    pub fn new(font: Rc<Font>, mouse: PlanarSceneNode) -> Self {
        let now = Instant::now();

        // Setup a server,
        // the player's client,
        // and another player.

        let ch0: LagNetwork<Input> = LagNetwork::new(DEFAULT_LAG);
        let ch1: LagNetwork<Vec<WorldState>> = LagNetwork::new(DEFAULT_LAG);
        let ch2: LagNetwork<Vec<WorldState>> = LagNetwork::new(DEFAULT_LAG);

        let mut player1 = Client::new(ch0.clone(), ch1.clone());
        let mut player2 = Client::new(ch0.clone(), ch2.clone());

        let mut server = Server::new(ch0.clone());

        // Connect the clients to the server.
        // Give the Client enough data to identify itself.
        player1.bind(server.connect(ch1.clone()));
        player2.bind(server.connect(ch2.clone()));

        Self {
            font,
            player1: (player1, now),
            player2: (player2, now),
            server: (server, now),
            second: now,
            camera: FixedView::new(),

            ch: (ch0, ch1, ch2),
            ch_t: (0, 0, 0),

            mouse,
            mouse_x: 0.0,
            mouse_y: 0.0,
        }
    }

    fn events(&mut self, win: &mut Window) {
        for mut event in win.events().iter() {
            match event.value {
                WindowEvent::Key(Key::Left, action, _)  => { event.inhibited = true; self.player2.0.key_left  = action == Action::Press }
                WindowEvent::Key(Key::Right, action, _) => { event.inhibited = true; self.player2.0.key_right = action == Action::Press }

                WindowEvent::Key(Key::W, action, _) => { event.inhibited = true; self.player1.0.key_left  = action == Action::Press }
                WindowEvent::Key(Key::S, action, _) => { event.inhibited = true; self.player1.0.key_right = action == Action::Press }

                WindowEvent::Key(Key::A, action, _) => { event.inhibited = true; self.player1.0.key_left  = action == Action::Press }
                WindowEvent::Key(Key::D, action, _) => { event.inhibited = true; self.player1.0.key_right = action == Action::Press }

                WindowEvent::MouseButton(MouseButton::Button1, action, _) => {
                    event.inhibited = true;
                    if let Some(mut node) = self.player1.0.entities.get_mut(&0).and_then(|e| e.node.as_mut()) {
                        node.fire = action == Action::Press
                    }
                }

                WindowEvent::CursorPos(x, y, _) => {
                    //event.inhibited = true;
                    self.mouse_x = x;
                    self.mouse_y = y;
                }

                _ => (),
            }
        }
    }
}

impl State for Simulator {
    fn cameras_and_effect(&mut self) -> (Option<&mut Camera>, Option<&mut PlanarCamera>, Option<&mut PostProcessingEffect>) {
        (Some(&mut self.camera), None, None)
    }
    fn step(&mut self, win: &mut Window) {
        self.events(win);

        let pt1 = self.player1.1 + secs_to_duration(1.0 / CLIENT_UPDATE_RATE);
        let pt2 = self.player1.1 + secs_to_duration(1.0 / CLIENT_UPDATE_RATE);
        let server = self.server.1 + secs_to_duration(1.0 / SERVER_UPDATE_RATE);

        let now = Instant::now();
        let p = &mut self.player1;
        if pt1 <= now {
            p.1 = now;
            p.0.update();
        }
        let p = &mut self.player2;
        if pt2 <= now {
            p.1 = now;
            p.0.update();
        }

        let p = &mut self.server;
        if server <= now {
            p.1 = now;
            p.0.update();
        }

        if self.second <= now {
            self.second += Duration::from_secs(1);
            let ch0 = self.ch.0.sum_bytes();
            let ch1 = self.ch.1.sum_bytes();
            let ch2 = self.ch.2.sum_bytes();
            self.ch_t = (ch0, ch1, ch2);
        }

        let (w, h) = (win.width() as f32, win.height() as f32);
        let (x, y) = (self.mouse_x as f32, self.mouse_y as f32);
        let (x, y) = (x - w * 0.5, -y + h * 0.5);
        let mouse = Point2::new(x, y);
        self.mouse.set_local_translation(Translation2::new(x - 0.5, y + 0.5));

        let height = (win.height() as f32) / 3.0 / ACTOR_RADIUS;
        let start2 = height * 0.0;
        let start0 = height * 1.0;
        let start1 = height * 2.0;

        let section0 = View::new(start0, height);
        let section1 = View::new(start1, height);
        let section2 = View::new(start2, height);

        section0.render_nodes(win, mouse, self.server.0.clients.iter_mut()
            .map(|c| &mut c.entity));
        section1.render_nodes(win, mouse, self.player1.0.entities.values_mut());
        section2.render_nodes(win, mouse, self.player2.0.entities.values_mut());

        let mut t = Text::new(win, self.font.clone());

        let info = Point2::new(500.0, 10.0);
        t.info(info, &format!("Update rate [server: {:?}] [client: {:?}], lag: {:?}",
            SERVER_UPDATE_RATE, CLIENT_UPDATE_RATE, DEFAULT_LAG,
        ));

        let status0 = Point2::new(10.0, section0.middle * FONT_SIZE);
        let status1 = Point2::new(10.0, section1.middle * FONT_SIZE);
        let status2 = Point2::new(10.0, section2.middle * FONT_SIZE);
        t.server (status0, &format!("Server\n recv:{}\n\n {}",
            Kbps(self.ch_t.0), &self.server.0.status ));
        t.current(status1, &format!("Current player [WASD + Mouse]\n recv:{}\n\n {}",
            Kbps(self.ch_t.1), &self.player1.0.status));
        t.another(status2, &format!("Another player [Arrows]\n recv: {}\n\n {}",
            Kbps(self.ch_t.2), &self.player2.0.status));
    }
}

pub fn render_nodes<'a>(win: &mut Window, mouse: Point2<f32>, y: f32, actors: impl Iterator<Item=&'a mut Actor>) {
    for e in actors {
        e.render(win, y, mouse)
    }
}

fn draw_bounding_box(win: &mut Window, max: f32) {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(0.0, 0.0, max);
        let c = Point3::new(max, 0.0, max);
        let d = Point3::new(max, 0.0, 0.0);

        let e = Point3::new(0.0, max, 0.0);
        let f = Point3::new(0.0, max, max);
        let g = Point3::new(max, max, max);
        let h = Point3::new(max, max, 0.0);

        let colour = &Point3::new(0.3, 0.3, 0.3);

        win.draw_line(&a, &b, &colour);
        win.draw_line(&b, &c, &colour);
        win.draw_line(&c, &d, &colour);
        win.draw_line(&d, &a, &colour);

        win.draw_line(&e, &f, &colour);
        win.draw_line(&f, &g, &colour);
        win.draw_line(&g, &h, &colour);
        win.draw_line(&h, &e, &colour);

        win.draw_line(&a, &e, &colour);
        win.draw_line(&b, &f, &colour);
        win.draw_line(&c, &g, &colour);
        win.draw_line(&d, &h, &colour);
}
