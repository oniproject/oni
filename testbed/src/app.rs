use std::rc::Rc;
use kiss3d::{
    window::{State, Window},
    text::Font,
    event::{Action, WindowEvent, Key, MouseButton},
    scene::PlanarSceneNode,
    camera::{self, Camera},
    planar_camera::{self, PlanarCamera},
    post_processing::PostProcessingEffect,
};
use nalgebra::{Translation2, Point2, Vector2};
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

    camera: camera::FixedView,
    planar_camera: planar_camera::Sidescroll,

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

        let mut player1 = new_client(ch0.clone(), ch1.clone(), false);
        let mut player2 = new_client(ch0.clone(), ch2.clone(), true);

        let mut server = new_server(ch0.clone());

        // Connect the clients to the server.
        // Give the Client enough data to identify itself.
        player1.client_bind(server.server_connect(ch1.clone()));
        player2.client_bind(server.server_connect(ch2.clone()));

        Self {
            font,
            player1,
            player2,
            server,

            camera: camera::FixedView::new(),
            planar_camera: planar_camera::Sidescroll::new(),

            mouse,
            mouse_pos: Point2::origin(),
        }
    }

    fn events(&mut self, win: &mut Window) {
        let p1 = &mut self.player1;
        let p2 = &mut self.player2;
        for mut event in win.events().iter() {
            match event.value {
                WindowEvent::Key(Key::Escape, _, _) | WindowEvent::Close => { win.close() }

                WindowEvent::Key(key, action, _) => {
                    event.inhibited = true;
                    match key {
                        Key::Up | Key::Down | Key::Left | Key::Right =>
                            p2.client_arrows(key, action),
                        Key::W | Key::A | Key::S | Key::D =>
                            p1.client_wasd(key, action),
                        _ => (),
                    }
                }

                WindowEvent::MouseButton(MouseButton::Button1, action, _) => {
                    p1.client_fire(action == Action::Press);
                    //event.inhibited = true;
                }

                WindowEvent::CursorPos(x, y, _) => {
                    //event.inhibited = true;
                    self.mouse_pos.x = x;
                    self.mouse_pos.y = y;
                }

                _ => (),
            }
        }

        let (w, h) = (win.width() as f32, win.height() as f32);
        let (x, y) = (self.mouse_pos.x as f32, self.mouse_pos.y as f32);
        let mouse = self.planar_camera.unproject(
            &Point2::new(x, y),
            &Vector2::new(w, h),
        );

        self.player1.client_rotation(win, mouse);
        self.mouse.set_local_translation(Translation2::new(mouse.x, mouse.y));
    }
}

impl State for AppState {
    fn cameras_and_effect(&mut self) -> (Option<&mut Camera>, Option<&mut PlanarCamera>, Option<&mut PostProcessingEffect>) {
        (Some(&mut self.camera), Some(&mut self.planar_camera), None)
    }

    fn step(&mut self, win: &mut Window) {
        self.events(win);

        self.server.update();
        self.player1.update();
        self.player2.update();

        let height = (win.height() as f32) / 3.0 / ACTOR_RADIUS;
        self.server.update_view(height * 1.0, height);
        self.player1.update_view(height * 2.0, height);
        self.player2.update_view(height * 0.0, height);

        self.server.render_nodes(win, &self.planar_camera);
        self.player1.render_nodes(win, &self.planar_camera);
        self.player2.render_nodes(win, &self.planar_camera);

        let mut text = Text::new(win, self.font.clone());

        //let info = Point2::new(800.0, 10.0);
        //t.info(info, &format!("Lag: {:?}", DEFAULT_LAG));

        // Show some info.
        self.server.server_status(&mut text, SERVER);
        self.player1.client_status(&mut text, CURRENT, "Current player [WASD+Mouse]");
        self.player2.client_status(&mut text, ANOTHER, "Another player [Arrows]");
    }
}
