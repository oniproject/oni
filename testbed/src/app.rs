use std::rc::Rc;
use specs::prelude::*;
use kiss3d::{
    window::{State, Window},
    text::Font,
    event::{Action, WindowEvent, Key, MouseButton},
    scene::PlanarSceneNode,
    camera::{Camera, FixedView},
    planar_camera::PlanarCamera,
    post_processing::PostProcessingEffect,
};
use nalgebra::{Translation2, Point2};
use crate::{
    actor::Actor,
    prot::{Input, WorldState},
    client::Client,
    server::Server,
    lag::LagNetwork,
    util::*,
    consts::*,
};

pub struct AppState {
    font: Rc<Font>,
    player1: Client,
    player2: Client,
    server: Server,

    camera: FixedView,

    mouse: PlanarSceneNode,
    mouse_x: f64,
    mouse_y: f64,
}

impl AppState {
    pub fn new(font: Rc<Font>, mouse: PlanarSceneNode) -> Self {
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
            player1: player1,
            player2: player2,
            server: server,
            camera: FixedView::new(),

            mouse,
            mouse_x: 0.0,
            mouse_y: 0.0,
        }
    }

    fn events(&mut self, win: &mut Window) {
        let p1 = &mut self.player1;
        let p2 = &mut self.player2;
        for mut event in win.events().iter() {
            match event.value {
                WindowEvent::Key(Key::Left, action, _)  => { event.inhibited = true; p2.key_left (action == Action::Press) }
                WindowEvent::Key(Key::Right, action, _) => { event.inhibited = true; p2.key_right(action == Action::Press) }

                WindowEvent::Key(Key::W, action, _) => { event.inhibited = true; p1.key_left (action == Action::Press) }
                WindowEvent::Key(Key::S, action, _) => { event.inhibited = true; p1.key_right(action == Action::Press) }

                WindowEvent::Key(Key::A, action, _) => { event.inhibited = true; p1.key_left (action == Action::Press) }
                WindowEvent::Key(Key::D, action, _) => { event.inhibited = true; p1.key_right(action == Action::Press) }

                WindowEvent::MouseButton(MouseButton::Button1, action, _) => {
                    event.inhibited = true;
                    p1.fire(action == Action::Press)
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

impl State for AppState {
    fn cameras_and_effect(&mut self) -> (Option<&mut Camera>, Option<&mut PlanarCamera>, Option<&mut PostProcessingEffect>) {
        (Some(&mut self.camera), None, None)
    }

    fn step(&mut self, win: &mut Window) {
        self.events(win);

        self.server.update();
        self.player1.update();
        self.player2.update();

        let (w, h) = (win.width() as f32, win.height() as f32);
        let (x, y) = (self.mouse_x as f32, self.mouse_y as f32);
        let (x, y) = (x - w * 0.5, -y + h * 0.5);
        let mouse = Point2::new(x, y);
        self.mouse.set_local_translation(Translation2::new(x - 0.5, y + 0.5));

        let height = (win.height() as f32) / 3.0 / ACTOR_RADIUS;
        let section0 = View::new(height * 1.0, height);
        let section1 = View::new(height * 2.0, height);
        let section2 = View::new(height * 0.0, height);

        {
            let mut clients = self.server.world.write_storage::<crate::server::Connect>();
            let clients = (&mut clients).join().map(|c| &mut c.entity);

            section0.render_nodes(win, mouse, clients);
        }

        {
            let mut en1 = self.player1.world.write_storage::<Actor>();
            let mut en2 = self.player2.world.write_storage::<Actor>();
            section1.render_nodes(win, mouse, (&mut en1).join());
            section2.render_nodes(win, mouse, (&mut en2).join());
        }

        let mut t = Text::new(win, self.font.clone());

        let info = Point2::new(800.0, 10.0);
        t.info(info, &format!("Lag: {:?}", DEFAULT_LAG));

        // Show some info.
        {
            let at0 = Point2::new(10.0, section0.start * FONT_SIZE);
            let at1 = Point2::new(10.0, section1.start * FONT_SIZE);
            let at2 = Point2::new(10.0, section2.start * FONT_SIZE);

            t.server( at0, &self.server.status());
            t.current(at1, &self.player1.status());
            t.another(at2, &self.player2.status());
        }
    }
}
