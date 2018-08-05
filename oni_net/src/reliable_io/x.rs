


void fragment_reassembly_data_cleanup( void * data, void * allocator_context, void (*free_function)(void*,void*) )

{
    assert( free_function );
    struct fragment_reassembly_data_t * reassembly_data = (struct fragment_reassembly_data_t*) data;
    if ( reassembly_data.packet_data )
    {
        free_function( allocator_context, reassembly_data.packet_data );
        reassembly_data.packet_data = NULL;
    }
}

// ---------------------------------------------------------------

int read_fragment_header(
    uint8_t * packet_data, 
    int packet_bytes, 
    int max_fragments, 
    int fragment_size, 
    int * fragment_id, 
    int * num_fragments, 
    int * fragment_bytes, 
    uint16_t * sequence, 
    uint16_t * ack, 
    uint32_t * ack_bits )
{
    if packet_bytes < RELIABLE_FRAGMENT_HEADER_BYTES {
        return -1;
    }

    uint8_t * p = packet_data;

    uint8_t prefix_byte =read_uint8( &p );
    if ( prefix_byte != 1 )
    {
        printf( RELIABLE_LOG_LEVEL_ERROR, "[%s] prefix byte is not a fragment\n", name );
        return -1;
    }
    
    *sequence = read_uint16( &p );
    *fragment_id = (int) read_uint8( &p );
    *num_fragments = ( (int) read_uint8( &p ) ) + 1;

    if *num_fragments > max_fragments {
        return -1;
    }

    if *fragment_id >= *num_fragments  {
        return -1;
    }

    *fragment_bytes = packet_bytes - RELIABLE_FRAGMENT_HEADER_BYTES;

    uint16_t packet_sequence = 0;
    uint16_t packet_ack = 0;
    uint32_t packet_ack_bits = 0;

    if ( *fragment_id == 0 )
    {
        int packet_header_bytes = read_packet_header( name, 
                                                               packet_data + RELIABLE_FRAGMENT_HEADER_BYTES, 
                                                               packet_bytes, 
                                                               &packet_sequence, 
                                                               &packet_ack, 
                                                               &packet_ack_bits );

        if ( packet_header_bytes < 0 )
        {
            printf( RELIABLE_LOG_LEVEL_ERROR, "[%s] bad packet header in fragment\n", name );
            return -1;
        }

        if ( packet_sequence != *sequence )
        {
            printf( RELIABLE_LOG_LEVEL_ERROR, "[%s] bad packet sequence in fragment. expected %d, got %d\n", name, *sequence, packet_sequence );
            return -1;
        }

        *fragment_bytes = packet_bytes - packet_header_bytes - RELIABLE_FRAGMENT_HEADER_BYTES;
    }

    *ack = packet_ack;
    *ack_bits = packet_ack_bits;

    if ( *fragment_bytes > fragment_size )
    {
        printf( RELIABLE_LOG_LEVEL_ERROR, "[%s] fragment bytes %d > fragment size %d\n", name, *fragment_bytes, fragment_size );
        return - 1;
    }

    if ( *fragment_id != *num_fragments - 1 && *fragment_bytes != fragment_size )
    {
        printf( RELIABLE_LOG_LEVEL_ERROR, "[%s] fragment %d is %d bytes, which is not the expected fragment size %d\n", 
            name, *fragment_id, *fragment_bytes, fragment_size );
        return -1;
    }

    return (int) ( p - packet_data );
}

impl Frag {
    fn store_fragment_data(
        &mut self,
        header: Header,
        id: usize,
        size: usize,
        data: &[u8],
        bytes: usize,
        )
    {
        if fragment_id == 0 {
            let packet_header = [0u8; MAX_PACKET_HEADER_BYTES];
            self.packet_header_bytes = Header::write(&mut packet_header[..], seq, ack, ack_bits);
            memcpy(
                self.packet_data + MAX_PACKET_HEADER_BYTES - self.packet_header_bytes, 
                    packet_header,
                    self.packet_header_bytes,
                );

            data += self.packet_header_bytes;
            bytes -= self.packet_header_bytes;
        }

        if id == self.total - 1 {
            self.packet_bytes = ( reassembly_data.num_fragments_total - 1 ) * fragment_size + fragment_bytes;
        }

        memcpy( reassembly_data.packet_data + RELIABLE_MAX_PACKET_HEADER_BYTES + fragment_id * fragment_size, fragment_data, fragment_bytes );
    }
}
