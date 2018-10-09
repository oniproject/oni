pub struct Bandwidth {
    kbps: f32,
    smoothing_factor: f32,
}

impl Bandwidth {
    pub fn calculate(&mut self) {
        let base: u32 = (self.sent_packets.sequence - self.config.sent_packets_buffer_size + 1) + 0xFFFF;
        let count = self.config.sent_packets_buffer_size / 2;

        let mut start = std::f64::MAX;
        let mut finish = 0.0;

        let mut bytes = 0;
        for seq in (0..count).map(|i| (base + i) as u16) {
            if let Some(packet) = self.packets.find(seq) {
                let time = packet.time();
                bytes += packet.len();
                if  start > time {
                    start = time;
                }
                if  finish < time {
                    finish = time;
                }
            }
        }
        if start != std::f64::MAX && finish != 0.0 {
            let kbps = ((bytes as f64 ) / (finish - start) * 8.0 / 1000.0) as f32;
            if (self.kbps - kbps).abs() > 0.00001 {
                self.kbps += (kbps - self.kbps) * self.smoothing_factor;
            } else {
                self.kbps = kbps;
            }
        }
    }
}

pub struct PacketLoss {
    loss: f32,
    smoothing_factor: f32,
}

impl PacketLoss
    fn calculate(&mut self) {
        let base: u32 = (self.sent_packets.sequence - self.config.sent_packets_buffer_size + 1) + 0xFFFF;
        let count = self.config.sent_packets_buffer_size / 2;

        let mut dropped = 0;
        for seq in (0..count).map(|i| (base + i) as u16) {
            if self.packets.find(seq).filter(|p| p.acked()).is_some() {
                dropped += 1;
            }
        }

        let loss = (dropped as f32) / (count as f32) * 100.0;
        if (self.loss - loss).abs() > 0.00001 {
            self.loss += (loss - self.loss) * self.config.smoothing_factor;
        } else {
            self.loss = loss;
        }
    }
}


    pub fn update(&mut self, time: f64) {
        self.time = time;
        self.calculate_packet_loss();
        self.calculate_sent_bandwidth();
        self.calculate_received_bandwidth();
        self.calculate_acked_bandwidth();
    }


    fn calculate_received_bandwidth(&mut self) {
        unimplemented!();
        /*
        let base: u32 = ( self.received_packets.sequence - self.config.received_packets_buffer_size + 1 ) + 0xFFFF;
        let num_samples = self.config.received_packets_buffer_size / 2;

        let mut start_time = std::f64::MAX;
        let mut finish_time = 0.0;
        let mut bytes_sent = 0;
        for i in 0..num_samples; ++i  {
            uint16_t sequence = (uint16_t) ( base_sequence + i );
            struct received_packet_data_t * received_packet_data = (struct received_packet_data_t*)
                sequence_buffer_find( self.received_packets, sequence );
            if !received_packet_data {
                continue;
            }
            bytes_sent += packet.packet_bytes;
            if  start > packet.time {
                start = packet.time;
            }
            if  finish < packet.time {
                finish = packet.time;
            }
        }
        if ( start_time != FLT_MAX && finish_time != 0.0 )
        {
            float received_bandwidth_kbps = (float) ( ( (double) bytes_sent ) / ( finish_time - start_time ) * 8.0f / 1000.0f );
            if ( fabs( self.received_bandwidth_kbps - received_bandwidth_kbps ) > 0.00001 )
            {
                self.received_bandwidth_kbps += ( received_bandwidth_kbps - self.received_bandwidth_kbps ) * self.config.bandwidth_smoothing_factor;
            }
            else
            {
                self.received_bandwidth_kbps = received_bandwidth_kbps;
            }
        }
        */
    }

    fn calculate_acked_bandwidth(&mut self) {
        unimplemented!();
        /*
        uint32_t base_sequence = ( self.sent_packets.sequence - self.config.sent_packets_buffer_size + 1 ) + 0xFFFF;
        int i;
        int bytes_sent = 0;
        double start = FLT_MAX;
        double finish = 0.0;
        int num_samples = self.config.sent_packets_buffer_size / 2;
        for ( i = 0; i < num_samples; ++i )
        {
            uint16_t sequence = (uint16_t) ( base_sequence + i );
            struct sent_packet_data_t * sent_packet_data = (struct sent_packet_data_t*)
                sequence_buffer_find( self.sent_packets, sequence );
            if ( !sent_packet_data || !sent_packet_data.acked )
            {
                continue;
            }
            bytes_sent += sent_packet_data.packet_bytes;
            if  start > packet.time {
                start = packet.time;
            }
            if  finish < packet.time {
                finish = packet.time;
            }
        }

        if start != std::f64::MAX && finish != 0.0 {
            let kbps = ((bytes_sent as f64) / (finish - start) * 8.0 / 1000.0) as f32;
            if (self.acked_bandwidth_kbps - kbps).abs() > 0.00001 {
                self.acked_bandwidth_kbps += (kbps - self.acked_bandwidth_kbps) * self.config.bandwidth_smoothing_factor;
            } else {
                self.acked_bandwidth_kbps = kbps;
            }
        }
        */
    }
