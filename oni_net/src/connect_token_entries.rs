const MAX_CONNECT_TOKEN_ENTRIES: usize = MAX_CLIENTS * 8;

struct ConnectTokenEntry {
    time: f64,
    mac: [u8; MAC_BYTES],
    address: SocketAddr,
}

pub struct ConnectTokenEntries {
    map: HashMap<[u8; MAC_BYTES], (f64, SocketAddr)>,
}

void netcode_connect_token_entries_reset( struct netcode_connect_token_entry_t * connect_token_entries )
{
    int i;
    for ( i = 0; i < NETCODE_MAX_CONNECT_TOKEN_ENTRIES; ++i )
    {
        connect_token_entries[i].time = -1000.0;
        memset( connect_token_entries[i].mac, 0, NETCODE_MAC_BYTES );
        memset( &connect_token_entries[i].address, 0, sizeof( struct netcode_address_t ) );
    }
}

int connect_token_entries_find_or_add(
    struct netcode_connect_token_entry_t * connect_token_entries,
    address: SocketAddr,
    uint8_t * mac,
    double time )
{
    // find the matching entry for the token mac and the oldest token entry.
    // constant time worst case.
    // This is intentional!

    let mut matching_token_index = -1;
    let mut oldest_token_index = -1;
    let mut oldest_token_time = 0.0;

    for i in 0..MAX_CONNECT_TOKEN_ENTRIES {
        if mac == connect_token_entries[i].mac {
            matching_token_index = i;
        }

        if oldest_token_index == -1 || connect_token_entries[i].time < oldest_token_time {
            oldest_token_time = connect_token_entries[i].time;
            oldest_token_index = i;
        }
    }

    // if no entry is found with the mac,
    // this is a new connect token.
    // replace the oldest token entry.

    assert!(oldest_token_index != -1);

    if matching_token_index == -1 {
        connect_token_entries[oldest_token_index].time = time;
        connect_token_entries[oldest_token_index].address = *address;
        connect_token_entries[oldest_token_index].mac = mac;
        return true;
    }

    // allow connect tokens we have already seen from the same address
    assert!(matching_token_index >= 0);
    assert!(matching_token_index < MAX_CONNECT_TOKEN_ENTRIES);

    connect_token_entries[matching_token_index].address == address
}
