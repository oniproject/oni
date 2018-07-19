use serde_cbor;
use specs::prelude::*;

#[derive(Serialize, Deserialize, Debug)]
pub enum Event {
    /// Actor
    A {
        id: u16,
        pos: (f32, f32),
        vel: (f32, f32),
    },
    /// Enemy
    E {
        id: u16,
        pos: (f32, f32),
        vel: (f32, f32),
    },
}

pub struct Replication {
    temp_buf: Vec<u8>,
}

impl<'a> System<'a> for Replication {
    type SystemData = (
        ReadStorage<'a, Vel>,
        ReadStorage<'a, Pos>,
        ReadStorage<'a, WS>,
    );

    fn run(&mut self, (ref pos, ref vel, ref ws): Self::SystemData) {
        let state: Vec<_> = (pos, vel).join()
            .map(|(p, v)| Event::A {
                id: 5,
                pos: (p.0, p.1),
                vel: (v.0, v.1),
            })
            .collect();

        self.temp_buf.clear();
        serde_cbor::to_writer(&mut self.temp_buf, &state).unwrap();

        for ws in ws.join() {
            ws.0.send(self.temp_buf.clone()).unwrap();
        }
    }
}
