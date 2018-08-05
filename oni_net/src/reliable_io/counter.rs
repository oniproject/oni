use std::time::Instant;

#[derive(Default)]
pub struct Counters {
    pub rtt: f32,
    pub packet_loss: f32,
    pub sent_bandwidth_kbps: f32,
    pub recv_bandwidth_kbps: f32,
    pub acked_bandwidth_kbps: f32,

    pub rtt_smoothing_factor: f32,

    pub num_packets_sent: u64,
    pub num_packets_received: u64,
    pub num_packets_acked: u64,
    pub num_packets_stale: u64,
    pub num_packets_invalid: u64,
    pub num_packets_too_large_to_send: u64,
    pub num_packets_too_large_to_receive: u64,

    pub num_fragments_sent: u64,
    pub num_fragments_received: u64,
    pub num_fragments_invalid: u64,
}

/*
impl Default for Counters {
    rtt: 0.0,
    packet_loss: 0.0,
    sent_bandwidth_kbps: 0.0,
    recv_bandwidth_kbps: 0.0,
    acked_bandwidth_kbps: 0.0,
}
*/

impl Counters {
    pub fn ack_packet(&mut self, time: Instant, dt: Instant) {
        self.num_packets_acked += 1;
        unimplemented!()
        /*
        let rtt = 1000.0 * (time - dt as f64) as f32;
        assert!(rtt >= 0.0);
        if (self.rtt == 0.0 && rtt > 0.0) || (self.rtt - rtt).abs() < 0.00001 {
            self.rtt = rtt;
        } else {
            self.rtt += (rtt - self.rtt) * self.rtt_smoothing_factor;
        }
        */
    }
    /*
    pub fn rtt(&self) -> f32 { self.rtt }
    pub fn packet_loss(&self) -> f32 { self.packet_loss }
    pub fn sent_bandwidth_kbps(&self) -> f32 { self.sent_bandwidth_kbps }
    pub fn recv_bandwidth_kbps(&self) -> f32 { self.recv_bandwidth_kbps }
    pub fn acked_bandwidth_kbps(&self) -> f32 { self.acked_bandwidth_kbps }
    pub fn counters(&self) -> &Counters { &self.counters }
    */
}
