use std::rc::Rc;
use kiss3d::{
    window::{State, Window},
    text::Font,
    event::{Action, WindowEvent, Key, MouseButton},
    camera::{self, Camera},
    planar_camera::{self, PlanarCamera},
    post_processing::PostProcessingEffect,
};
use nalgebra::{Point2, Vector2};
use oni::simulator::Simulator;
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
    planar_camera: planar_camera::FixedView,

    network: Simulator,

    mouse: Point2<f64>,
}

impl AppState {
    pub fn new(font: Rc<Font>) -> Self {
        // Setup a server,
        // the player's client,
        // and another player.

        let network = Simulator::new();

        let a0 = "[::1]:0000".parse().unwrap();
        let a1 = "[::1]:1111".parse().unwrap();
        let a2 = "[::1]:2222".parse().unwrap();

        let conf = oni::simulator::Config {
            latency: DEFAULT_LATENCY,
            jitter: DEFAULT_JITTER,
            loss: 0.0,
            duplicate: 0.0,
        };

        let ch0 = network.add_socket(a0);
        let ch1 = network.add_socket(a1);
        let ch2 = network.add_socket(a2);

        network.add_mapping(a0, a1, conf);
        network.add_mapping(a0, a2, conf);
        network.add_mapping(a1, a0, conf);
        network.add_mapping(a2, a0, conf);

        let mut server = new_server(ch0);
        let mut player1 = new_client(ch1, a0, false);
        let mut player2 = new_client(ch2, a0, true);

        // Connect the clients to the server.
        // Give the Client enough data to identify itself.
        player1.client_bind(server.server_connect(a1));
        player2.client_bind(server.server_connect(a2));

        Self {
            font,
            player1,
            player2,
            server,

            camera: camera::FixedView::new(),
            planar_camera: planar_camera::FixedView::new(),

            network,
            mouse: Point2::origin(),
        }
    }

    fn events(&mut self, win: &mut Window) {
        let p1 = &mut self.player1;
        let p2 = &mut self.player2;
        for mut event in win.events().iter() {
            //event.inhibited = true;
            match event.value {
                WindowEvent::Key(Key::Escape, _, _) | WindowEvent::Close => { win.close() }

                WindowEvent::Key(Key::Space, action, _) |
                WindowEvent::MouseButton(MouseButton::Button1, action, _) => {
                    p1.client_fire(action == Action::Press);
                    //event.inhibited = true;
                }

                WindowEvent::Key(key, action, _) => {
                    match key {
                        Key::Up | Key::Down | Key::Left | Key::Right =>
                            p2.client_arrows(key, action),
                        Key::W | Key::A | Key::S | Key::D =>
                            p1.client_wasd(key, action),
                        _ => (),
                    }
                }

                WindowEvent::CursorPos(x, y, _) => {
                    //event.inhibited = true;
                    self.mouse.x = x;
                    self.mouse.y = y;
                }

                _ => (),
            }
        }

        let (x, y) = (self.mouse.x as f32, self.mouse.y as f32);
        self.player1.client_mouse(win, &self.planar_camera, Point2::new(x, y));
    }
}

impl State for AppState {
    fn cameras_and_effect(&mut self) -> (Option<&mut Camera>, Option<&mut PlanarCamera>, Option<&mut PostProcessingEffect>) {
        (Some(&mut self.camera), Some(&mut self.planar_camera), None)
    }

    fn step(&mut self, win: &mut Window) {
        self.events(win);

        let height = (win.height() as f32) / 3.0;
        self.server.update_view(height * 1.0, height);
        self.player1.update_view(height * 2.0, height);
        self.player2.update_view(height * 0.0, height);

        self.network.advance();
        self.server.run(win, &self.planar_camera);
        self.player1.run(win, &self.planar_camera);
        self.player2.run(win, &self.planar_camera);

        let mut text = Text::new(win, self.font.clone());

        //let info = Point2::new(800.0, 10.0);
        //t.info(info, &format!("Lag: {:?}", DEFAULT_LAG));

        // Show some info.
        self.server.server_status(&mut text, SERVER);
        self.player1.client_status(&mut text, CURRENT, "Current player [WASD+Mouse]");
        self.player2.client_status(&mut text, ANOTHER, "Another player [AI]");

        let size = win.size();
        let size = Vector2::new(size.x as f32, size.y as f32);

        for i in 1..3 {
            let i = i as f32;

            let a = Point2::new(   0.0, height * i as f32);
            let b = Point2::new(size.x, height * i as f32);

            let a = self.planar_camera.unproject(&a, &size);
            let b = self.planar_camera.unproject(&b, &size);

            win.draw_planar_line(&a, &b, &NAVY.into())
        }
    }
}
