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

use super::{Demo, Text};

pub struct AppState {
    font: Rc<Font>,
    player1: Demo,
    player2: Demo,
    server: Demo,

    camera: camera::FixedView,
    planar_camera: planar_camera::FixedView,

    network: Simulator,

    worker: oni::trace::AppendWorker,

    mouse: Point2<f64>,
}

fn new_pool(name: &'static str, num_threads: usize, index: usize) -> std::sync::Arc<rayon::ThreadPool> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .thread_name(move |n| format!("rayon #{} {}", n, name))
        .start_handler(move |_| oni::trace::register_thread(Some(index)))
        .build()
        .unwrap();

    std::sync::Arc::new(pool)
}

impl AppState {
    pub fn new(font: Rc<Font>) -> Self {
        let name = "trace.json.gz";
        let sleep = std::time::Duration::from_millis(200);
        let worker = oni::trace::AppendWorker::new(name, sleep);

        /*
        let pool = rayon::ThreadPoolBuilder::new()
            .thread_name(|n| format!("rayon #{}", n))
            .start_handler(|_| oni::trace::register_thread())
            .build()
            .unwrap();

        let pool = std::sync::Arc::new(pool);
        */

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

        let ch0 = network.add_socket_with_name(a0, "server");
        let ch1 = network.add_socket_with_name(a1, "current");
        let ch2 = network.add_socket_with_name(a2, "another");

        network.add_mapping(a0, a1, conf);
        network.add_mapping(a0, a2, conf);
        network.add_mapping(a1, a0, conf);
        network.add_mapping(a2, a0, conf);

        let mut player2 = new_client(new_pool("player2", 1, 1), ch2, a0, true);
        let mut server = new_server(new_pool("server", 1, 2), ch0);
        let mut player1 = new_client(new_pool("player1", 1, 3), ch1, a0, false);

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

            worker,

            network,
            mouse: Point2::origin(),
        }
    }

    fn events(&mut self, win: &mut Window) {
        oni::trace::scope![Events];

        for event in win.events().iter() {
            //event.inhibited = true;
            match event.value {
                WindowEvent::Key(Key::Escape, _, _) | WindowEvent::Close => {
                    use std::sync::Once;
                    win.close();

                    static START: Once = Once::new();
                    START.call_once(|| {
                        self.worker.end();
                    });
                }

                WindowEvent::Key(Key::Space, action, _) |
                WindowEvent::MouseButton(MouseButton::Button1, action, _) => {
                    self.player1.client_fire(action == Action::Press);
                    //event.inhibited = true;
                }

                WindowEvent::Key(key, action, _) => {
                    match key {
                        Key::Up | Key::Down | Key::Left | Key::Right =>
                            self.player2.client_arrows(key, action),
                        Key::W | Key::A | Key::S | Key::D =>
                            self.player1.client_wasd(key, action),
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
        oni::trace::scope![Window Step];

        self.events(win);

        let height = (win.height() as f32) / 3.0;
        self.server.update_view(height * 1.0, height);
        self.player1.update_view(height * 2.0, height);
        self.player2.update_view(height * 0.0, height);

        self.network.advance();

        {
            oni::trace::scope![dispatch];
            self.server.dispatch();
            self.player1.dispatch();
            self.player2.dispatch();
        }

        {
            oni::trace::scope![Run server];
            self.server.run(win, &self.planar_camera);
        }
        {
            oni::trace::scope![Run player1];
            self.player1.run(win, &self.planar_camera);
        }
        {
            oni::trace::scope![Run player2];
            self.player2.run(win, &self.planar_camera);
        }

        let mut text = Text::new(win, self.font.clone());

        //let info = Point2::new(800.0, 10.0);
        //t.info(info, &format!("Lag: {:?}", DEFAULT_LAG));

        {
            oni::trace::scope![Show info];
            self.server.server_status(&mut text, SERVER);
            self.player1.client_status(&mut text, CURRENT, "Current player [WASD+Mouse]");
            self.player2.client_status(&mut text, ANOTHER, "Another player [AI]");
        }

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
