use std::rc::Rc;
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
    client::new_client,
    server::new_server,
    util::*,
    consts::*,
};

pub struct AppState {
    font: Rc<Font>,
    player1: Demo,
    player2: Demo,
    server: Demo,

    camera: FixedView,

    mouse: PlanarSceneNode,
    mouse_pos: Point2<f64>,
}

impl AppState {
    pub fn new(font: Rc<Font>, mouse: PlanarSceneNode) -> Self {
        // Setup a server,
        // the player's client,
        // and another player.

        let ch0 = LagNetwork::new(DEFAULT_LAG);
        let ch1 = LagNetwork::new(DEFAULT_LAG);
        let ch2 = LagNetwork::new(DEFAULT_LAG);

        let mut player1 = new_client(ch0.clone(), ch1.clone());
        let mut player2 = new_client(ch0.clone(), ch2.clone());

        let mut server = new_server(ch0.clone());

        // Connect the clients to the server.
        // Give the Client enough data to identify itself.
        player1.client_bind(server.server_connect(ch1.clone()));
        player2.client_bind(server.server_connect(ch2.clone()));

        Self {
            font,
            player1: player1,
            player2: player2,
            server: server,
            camera: FixedView::new(),

            mouse,
            mouse_pos: Point2::origin(),
        }
    }

    fn events(&mut self, win: &mut Window) -> Point2<f32> {
        let p1 = &mut self.player1;
        let p2 = &mut self.player2;
        for mut event in win.events().iter() {
            event.inhibited = true;
            match event.value {
                WindowEvent::Key(Key::Escape, _, _) | WindowEvent::Close => { win.close() }

                WindowEvent::Key(Key::Up   , action, _) => { p2.client_arrows(Key::Up   , action) }
                WindowEvent::Key(Key::Down , action, _) => { p2.client_arrows(Key::Down , action) }
                WindowEvent::Key(Key::Left , action, _) => { p2.client_arrows(Key::Left , action) }
                WindowEvent::Key(Key::Right, action, _) => { p2.client_arrows(Key::Right, action) }

                WindowEvent::Key(Key::W, action, _) => { p1.client_wasd(Key::W, action) }
                WindowEvent::Key(Key::S, action, _) => { p1.client_wasd(Key::S, action) }
                WindowEvent::Key(Key::A, action, _) => { p1.client_wasd(Key::A, action) }
                WindowEvent::Key(Key::D, action, _) => { p1.client_wasd(Key::D, action) }

                WindowEvent::MouseButton(MouseButton::Button1, action, _) => {
                    p1.client_fire(action == Action::Press)
                }

                WindowEvent::CursorPos(x, y, _) => {
                    self.mouse_pos.x = x;
                    self.mouse_pos.y = y;
                }

                _ => (),
            }
        }

        let (w, h) = (win.width() as f32, win.height() as f32);
        let (x, y) = (self.mouse_pos.x as f32, self.mouse_pos.y as f32);
        let (x, y) = (x - w * 0.5, -y + h * 0.5);
        self.mouse.set_local_translation(Translation2::new(x - 0.5, y + 0.5));
        Point2::new(x, y)
    }
}

impl State for AppState {
    fn cameras_and_effect(&mut self) -> (Option<&mut Camera>, Option<&mut PlanarCamera>, Option<&mut PostProcessingEffect>) {
        (Some(&mut self.camera), None, None)
    }

    fn step(&mut self, win: &mut Window) {
        let mouse = self.events(win);

        self.server.update();
        self.player1.update();
        self.player2.update();

        let height = (win.height() as f32) / 3.0 / ACTOR_RADIUS;
        self.server.update_view(height * 1.0, height);
        self.player1.update_view(height * 2.0, height);
        self.player2.update_view(height * 0.0, height);

        self.server.render_nodes(win, mouse);
        self.player1.render_nodes(win, mouse);
        self.player2.render_nodes(win, mouse);

        let mut text = Text::new(win, self.font.clone());

        //let info = Point2::new(800.0, 10.0);
        //t.info(info, &format!("Lag: {:?}", DEFAULT_LAG));

        // Show some info.
        self.server.server_status(&mut text, SERVER);
        self.player1.client_status(&mut text, CURRENT, "Current player [WASD+Mouse]");
        self.player2.client_status(&mut text, ANOTHER, "Another player [Arrows]");
    }
}