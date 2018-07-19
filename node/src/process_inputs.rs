use specs::prelude::*;
use mio::net::TcpStream;

use components::*;
use connection::*;

pub struct ProcessInputs;

impl<'a> System<'a> for ProcessInputs {
    type SystemData = (
        WriteStorage<'a, Connection<TcpStream>>,
        WriteStorage<'a, Position>,
        ReadStorage<'a, Velocity>,
        WriteStorage<'a, LastProcessedInput>,
    );

    fn run(&mut self, mut data: Self::SystemData) {
        // Process all pending messages from clients.
        for (client, pos, vel, last_processed_input) in (&mut data.0, &mut data.1, &data.2, &mut data.3).join() {
            // Update the state of the entity, based on its input.
            // We just ignore inputs that don't look valid
            // this is what prevents clients from cheating.
            for m in client.take(5) {
                if m.validate() {
                    let dt = m.press_time;

                    pos.0 += m.velocity.0 * dt;
                    pos.1 += m.velocity.1 * dt;

                    last_processed_input.0 = m.input_sequence_number;
                }
            }
        }

        /*
        for other in (&mut data.0).join() {
            for m in &messages {
                other.send_chat(&m);
            }
        }
        */
    }
}
