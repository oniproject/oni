use std::{
    rc::Rc,
    sync::Arc,
    time::{Instant, Duration},
    net::SocketAddr,
};
use kiss2d::{Canvas, Font, Key, MouseButton};
use oni::SimulatedSocket as Socket;
use rayon::{ThreadPool, ThreadPoolBuilder};
use rayon::prelude::*;
use specs::prelude::*;
use nalgebra::{Point2, Vector2};
use crate::{
    client::new_client,
    server::new_server,
    util::*,
    consts::*,
};

use super::{Demo, Text};

pub struct AppState {
    font: Rc<Font<'static>>,
    player1: Demo,
    player2: Demo,
    server: Demo,
    worker: oni_trace::AppendWorker,
    mouse: Point2<f32>,
}

fn dos(server_addr: SocketAddr, num: usize) {
    let ticker = crossbeam_channel::tick(Duration::from_millis(33));
    let pool = new_pool("bots", 0, 666);
    let mut bots = Vec::new();

    loop {
        let _ = ticker.recv().unwrap();

        if bots.len() < num {
            for i in 0..5 {
                let id = i + bots.len() + i * 10_000;
                bots.push(crate::server::DDOSer::new(id as u64, server_addr));
            }
        }

        pool.install(|| {
            bots.par_iter_mut().for_each(|d| {
                oni_trace::scope![update bot];
                d.update()
            });
        });
    }
}


fn new_pool(name: &'static str, num_threads: usize, index: usize) -> Arc<ThreadPool> {
    use oni_trace::register_thread;
    Arc::new(ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .thread_name(move |n| format!("rayon #{} {}", n, name))
        .start_handler(move |_| register_thread(Some(index), Some(index)))
        .build()
        .unwrap())
}

fn new_dispatcher(name: &'static str, num_threads: usize, index: usize) -> DispatcherBuilder<'static, 'static> {
    DispatcherBuilder::new().with_pool(new_pool(name, num_threads, index))
}

impl AppState {
    pub fn new(font: Rc<Font<'static>>) -> Self {
        let name = "trace.json.gz";
        let sleep = std::time::Duration::from_millis(100);
        let worker = oni_trace::AppendWorker::new(name, sleep);

        // setup a server, the player's client, and another player.

        let (player1, player2, server) = {
            use std::io::Write;
            use oni::{
                protocol::MAX_PAYLOAD,
                token::{PublicToken, USER},
                crypto::keygen,
                Server,
                Client, State,
                ServerList,
            };

            let server = Server::simulated(PROTOCOL_ID, *PRIVATE_KEY);

            let mut server_list = ServerList::new();
            server_list.push(server.local_addr()).unwrap();

            let data = server_list.serialize().unwrap();
            let mut user = [0u8; USER];
            (&mut user[..]).write_all(b"some user data\0").unwrap();

            let connect_token1 = PublicToken::generate(
                data, user,
                CONNECT_TOKEN_EXPIRY,
                CONNECT_TOKEN_TIMEOUT,
                1,
                PROTOCOL_ID,
                &PRIVATE_KEY,
            );

            let connect_token2 = PublicToken::generate(
                data, user,
                CONNECT_TOKEN_EXPIRY,
                CONNECT_TOKEN_TIMEOUT,
                2,
                PROTOCOL_ID,
                &PRIVATE_KEY,
            );

            let player1 = Client::simulated(PROTOCOL_ID, &connect_token1);
            let player2 = Client::simulated(PROTOCOL_ID, &connect_token2);

            let s = server.local_addr();
            let p1 = player1.local_addr().unwrap();
            let p2 = player2.local_addr().unwrap();

            oni::config_socket(p1, s, Some(SIMULATOR_CONFIG));
            oni::config_socket(p2, s, Some(SIMULATOR_CONFIG));

            (player1, player2, server)
        };

        let server_addr = server.local_addr();
        std::thread::spawn(move || dos(server_addr, BOT_COUNT));

        Self {
            server: new_server(new_dispatcher("server", 1, 2), server),
            player1: new_client(new_dispatcher("player1", 1, 3), player1, server_addr, false),
            player2: new_client(new_dispatcher("player2", 1, 1), player2, server_addr, true),

            mouse: Point2::origin(),
            font,
            worker,
        }
    }

    fn events(&mut self, canvas: &mut Canvas) {
        let left = canvas.mouse_down(MouseButton::Left);
        let space = canvas.is_keydown(Key::Space);
        self.player1.client_fire(left | space);

        self.player1.client_wasd(canvas);
        //self.player2.client_arrows(canvas);

        if let Some(mouse) = canvas.mouse_pos() {
            self.mouse.x = mouse.0 as f32;
            self.mouse.y = mouse.1 as f32;
        }

            /* FIXME
        for event in win.events().iter() {
            match event.value {
                WindowEvent::Key(Key::Escape, _, _) | WindowEvent::Close => {
                    use std::sync::Once;
                    win.close();

                    static START: Once = Once::new();
                    START.call_once(|| {
                        self.worker.end();
                    });
                }
                _ => (),
            }
        }
        */

        self.player1.client_mouse(canvas, self.mouse);
    }

    pub fn render(&mut self, win: &mut Canvas) {
        oni_trace::scope![Window Step];

        self.events(win);

        let height = (win.size().1 as f32) / 3.0;
        self.server.update_view(height * 1.0, height);
        self.player1.update_view(height * 2.0, height);
        self.player2.update_view(height * 0.0, height);

        {
            oni_trace::scope![Run];
            self.server.run(win);
            self.player1.run(win);
            self.player2.run(win);
        }

        let mut text = Text::new(win, self.font.clone());

        //let info = Point2::new(800.0, 10.0);
        //t.info(info, &format!("Lag: {:?}", DEFAULT_LAG));

        self.server.server_status(&mut text, SERVER);
        self.player1.client_status(&mut text, CURRENT, "Current player [WASD+Mouse]");
        self.player2.client_status(&mut text, ANOTHER, "Another player [AI]");

        for y in (1..3).map(|i| height * i as f32) {
            win.hline(0, win.size().0 as isize, y as isize, NAVY)
        }

        {
            oni_trace::scope![dispatch];
            self.server.dispatch();
            self.player1.dispatch();
            self.player2.dispatch();
        }
    }
}
