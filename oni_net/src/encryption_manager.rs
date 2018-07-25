const MAX_ENCRYPTION_MAPPINGS ( MAX_CLIENTS * 4 )

struct Entry {
    timeout: u32,
    expire_time: f64,
    last_access_time: f64,
    address: SocketAddr,
    send_key: Key,
    receive_key: Key,
}

impl Entry {
    fn expired(&self, index: usize, time: f64) -> bool {
        (self.timeout > 0 && (self.last_access_time + self.timeout) < time) ||
        (self.expire_time >= 0.0 && self.expire_time < time)
    }
}

pub struct EncryptionManager {
    int num_encryption_mappings;
    int timeout[MAX_ENCRYPTION_MAPPINGS];
    double expire_time[MAX_ENCRYPTION_MAPPINGS];
    double last_access_time[MAX_ENCRYPTION_MAPPINGS];
    struct address_t address[MAX_ENCRYPTION_MAPPINGS];
    uint8_t send_key[KEY_BYTES*MAX_ENCRYPTION_MAPPINGS];
    uint8_t receive_key[KEY_BYTES*MAX_ENCRYPTION_MAPPINGS];
}

impl EncryptionManager {
    pub fn reset(&mut self) {
        debug!"reset encryption manager");
        self.num_encryption_mappings = 0;
        int i;
        for ( i = 0; i < MAX_ENCRYPTION_MAPPINGS; ++i )
        {
            self.expire_time[i] = -1.0;
            self.last_access_time[i] = -1000.0;
            memset( &self.address[i], 0, sizeof( struct address_t ) );
        }

        memset( self.timeout, 0, sizeof( self.timeout ) );
        memset( self.send_key, 0, sizeof( self.send_key ) );
        memset( self.receive_key, 0, sizeof( self.receive_key ) );
    }

    pub fn entry_expired(&self, index: usize, time: f64) -> bool {
        (self.timeout[index] > 0 && (self.last_access_time[index] + self.timeout[index]) < time ) ||
        (self.expire_time[index] >= 0.0 && self.expire_time[index] < time)
    }

    int add_encryption_mapping(&mut self, 
        struct address_t * address,
        uint8_t * send_key,
        uint8_t * receive_key,
        double time,
        double expire_time,
        int timeout )
    {
        int i;
        for ( i = 0; i < self.num_encryption_mappings; ++i )
        {
            if ( address_equal( &self.address[i], address ) && !self_entry_expired( self, i, time ) )
            {
                self.timeout[i] = timeout;
                self.expire_time[i] = expire_time;
                self.last_access_time[i] = time;
                memcpy( self.send_key + i * KEY_BYTES, send_key, KEY_BYTES );
                memcpy( self.receive_key + i * KEY_BYTES, receive_key, KEY_BYTES );
                return 1;
            }
        }

        for ( i = 0; i < MAX_ENCRYPTION_MAPPINGS; ++i )
        {
            if ( self.address[i].type == ADDRESS_NONE || self_entry_expired( self, i, time ) )
            {
                self.timeout[i] = timeout;
                self.address[i] = *address;
                self.expire_time[i] = expire_time;
                self.last_access_time[i] = time;
                memcpy( self.send_key + i * KEY_BYTES, send_key, KEY_BYTES );
                memcpy( self.receive_key + i * KEY_BYTES, receive_key, KEY_BYTES );
                if ( i + 1 > self.num_encryption_mappings )
                    self.num_encryption_mappings = i + 1;
                return 1;
            }
        }

        return 0;
    }

    pub fn remove_encryption_mapping(&mut self, address: SocketAddr, time: f64) -> bool {
        for i in 0..self.num_encryption_mappings {
            if ( address_equal( &self.address[i], address ) )
            {
                self.expire_time[i] = -1.0;
                self.last_access_time[i] = -1000.0;
                memset( &self.address[i], 0, sizeof( struct address_t ) );
                memset( self.send_key + i * KEY_BYTES, 0, KEY_BYTES );
                memset( self.receive_key + i * KEY_BYTES, 0, KEY_BYTES );

                if ( i + 1 == self.num_encryption_mappings )
                {
                    int index = i - 1;
                    while ( index >= 0 )
                    {
                        if ( !self_entry_expired( self, index, time ) )
                        {
                            break;
                        }
                        self.address[index].type = ADDRESS_NONE;
                        index--;
                    }
                    self.num_encryption_mappings = index + 1;
                }

                return 1;
            }
        }

        return 0;
    }

    pub fn find_encryption_mapping(&self, address: SocketAddr, time: f64) -> Option<usize> {
        for i in 0..self.num_encryption_mappings {
            if self.address[i] == address && !self.entry_expired(i, time) {
                self.last_access_time[i] = time;
                return i;
            }
        }
        None
    }

    pub fn touch(&mut self, index: usize, address: SocketAddr, time: f64) -> bool {
        if !&self.address[index] != address {
            return false;
        }
        self.last_access_time[index] = time;
        true
    }

    pub fn set_expire_time(&mut self, index: usize, expire_time: f64) {
        self.expire_time[index] = expire_time;
    }

    pub fn get_send_key(&self, index: usize) -> Option<&Key> {
        self.send_key.get(index)
    }

    pub fn get_receive_key(&self, index: usize) -> Option<&Key> {
        self.receive_key.get(index)
    }

    pub fn get_timeout(&self, index: usize) -> Option<u32> {
        self.timeout.get(index)
    }
}
