static void check_handler( RELIABLE_CONST char * condition, 
                           RELIABLE_CONST char * function,
                           RELIABLE_CONST char * file,
                           int line )
{
    printf( "check failed: ( %s ), function %s, file %s, line %d\n", condition, function, file, line );
#ifndef NDEBUG
    #if defined( __GNUC__ )
        __builtin_trap();
    #elif defined( _MSC_VER )
        __debugbreak();
    #endif
#endif
    exit( 1 );
}

#define check( condition )                                                                                      \
do                                                                                                              \
{                                                                                                               \
    if ( !(condition) )                                                                                         \
    {                                                                                                           \
        check_handler( #condition, (RELIABLE_CONST char*) __FUNCTION__, __FILE__, __LINE__ );                   \
    }                                                                                                           \
} while(0)

static void test_endian()
{
    uint32_t value = 0x11223344;

    char * bytes = (char*) &value;

#if RELIABLE_LITTLE_ENDIAN

    check( bytes[0] == 0x44 );
    check( bytes[1] == 0x33 );
    check( bytes[2] == 0x22 );
    check( bytes[3] == 0x11 );

#else // #if RELIABLE_LITTLE_ENDIAN

    check( bytes[3] == 0x44 );
    check( bytes[2] == 0x33 );
    check( bytes[1] == 0x22 );
    check( bytes[0] == 0x11 );

#endif // #if RELIABLE_LITTLE_ENDIAN
}

struct test_sequence_data_t
{
    uint16_t sequence;
};

#define TEST_SEQUENCE_BUFFER_SIZE 256

static void test_sequence_buffer()
{
    struct sequence_buffer_t * sequence_buffer = sequence_buffer_create( TEST_SEQUENCE_BUFFER_SIZE, 
                                                                                           sizeof( struct test_sequence_data_t ), 
                                                                                           NULL, 
                                                                                           NULL, 
                                                                                           NULL );

    check( sequence_buffer );
    check( sequence_buffer.sequence == 0 );
    check( sequence_buffer.num_entries == TEST_SEQUENCE_BUFFER_SIZE );
    check( sequence_buffer.entry_stride == sizeof( struct test_sequence_data_t ) );

    int i;
    for ( i = 0; i < TEST_SEQUENCE_BUFFER_SIZE; ++i )
    {
        check( sequence_buffer_find( sequence_buffer, ((uint16_t)i) ) == NULL );
    }                                                                      

    for ( i = 0; i <= TEST_SEQUENCE_BUFFER_SIZE*4; ++i )
    {
        struct test_sequence_data_t * entry = (struct test_sequence_data_t*) sequence_buffer_insert( sequence_buffer, ((uint16_t)i) );
        check( entry );
        entry.sequence = (uint16_t) i;
        check( sequence_buffer.sequence == i + 1 );
    }

    for ( i = 0; i <= TEST_SEQUENCE_BUFFER_SIZE; ++i )
    {
        struct test_sequence_data_t * entry = (struct test_sequence_data_t*) sequence_buffer_insert( sequence_buffer, ((uint16_t)i) );
        check( entry == NULL );
    }    

    int index = TEST_SEQUENCE_BUFFER_SIZE * 4;
    for ( i = 0; i < TEST_SEQUENCE_BUFFER_SIZE; ++i )
    {
        struct test_sequence_data_t * entry = (struct test_sequence_data_t*) sequence_buffer_find( sequence_buffer, (uint16_t) index );
        check( entry );
        check( entry.sequence == (uint32_t) index );
        index--;
    }

    sequence_buffer_reset( sequence_buffer );

    check( sequence_buffer );
    check( sequence_buffer.sequence == 0 );
    check( sequence_buffer.num_entries == TEST_SEQUENCE_BUFFER_SIZE );
    check( sequence_buffer.entry_stride == sizeof( struct test_sequence_data_t ) );

    for ( i = 0; i < TEST_SEQUENCE_BUFFER_SIZE; ++i )
    {
        check( sequence_buffer_find( sequence_buffer, (uint16_t) i ) == NULL );
    }

    sequence_buffer_destroy( sequence_buffer );
}

static void test_generate_ack_bits()
{
    struct sequence_buffer_t * sequence_buffer = sequence_buffer_create( TEST_SEQUENCE_BUFFER_SIZE, 
                                                                                           sizeof( struct test_sequence_data_t ), 
                                                                                           NULL, 
                                                                                           NULL, 
                                                                                           NULL );

    uint16_t ack = 0;
    uint32_t ack_bits = 0xFFFFFFFF;

    sequence_buffer_generate_ack_bits( sequence_buffer, &ack, &ack_bits );
    check( ack == 0xFFFF );
    check( ack_bits == 0 );

    int i;
    for ( i = 0; i <= TEST_SEQUENCE_BUFFER_SIZE; ++i )
    {
        sequence_buffer_insert( sequence_buffer, (uint16_t) i );
    }

    sequence_buffer_generate_ack_bits( sequence_buffer, &ack, &ack_bits );
    check( ack == TEST_SEQUENCE_BUFFER_SIZE );
    check( ack_bits == 0xFFFFFFFF );

    sequence_buffer_reset( sequence_buffer );

    uint16_t input_acks[] = { 1, 5, 9, 11 };
    int input_num_acks = sizeof( input_acks ) / sizeof( uint16_t );
    for ( i = 0; i < input_num_acks; ++i )
    {
        sequence_buffer_insert( sequence_buffer, input_acks[i] );
    }

    sequence_buffer_generate_ack_bits( sequence_buffer, &ack, &ack_bits );

    check( ack == 11 );
    check( ack_bits == ( 1 | (1<<(11-9)) | (1<<(11-5)) | (1<<(11-1)) ) );

    sequence_buffer_destroy( sequence_buffer );
}

struct test_context_t
{
    int drop;
    struct endpoint_t * sender;
    struct endpoint_t * receiver;
};

static void test_transmit_packet_function( void * _context, int index, uint16_t sequence, uint8_t * packet_data, int packet_bytes )
{
    (void) sequence;

    struct test_context_t * context = (struct test_context_t*) _context;

    if ( context.drop )
    {
        return;
    }

    if ( index == 0 )
    {
        endpoint_receive_packet( context.receiver, packet_data, packet_bytes );
    }
    else if ( index == 1 )
    {
        endpoint_receive_packet( context.sender, packet_data, packet_bytes );
    }
}

static int test_process_packet_function( void * _context, int index, uint16_t sequence, uint8_t * packet_data, int packet_bytes )
{
    struct test_context_t * context = (struct test_context_t*) _context;

    (void) context;
    (void) index;
    (void) sequence;
    (void) packet_data;
    (void) packet_bytes;

    return 1;
}

#define TEST_ACKS_NUM_ITERATIONS 256

static void test_acks()
{
    double time = 100.0;

    struct test_context_t context;
    memset( &context, 0, sizeof( context ) );
    
    struct config_t sender_config;
    struct config_t receiver_config;

    default_config( &sender_config );
    default_config( &receiver_config );

    sender_config.context = &context;
    sender_config.index = 0;
    sender_config.transmit_packet_function = &test_transmit_packet_function;
    sender_config.process_packet_function = &test_process_packet_function;

    receiver_config.context = &context;
    receiver_config.index = 1;
    receiver_config.transmit_packet_function = &test_transmit_packet_function;
    receiver_config.process_packet_function = &test_process_packet_function;

    context.sender = endpoint_create( &sender_config, time );
    context.receiver = endpoint_create( &receiver_config, time );

    double delta_time = 0.01;

    int i;
    for ( i = 0; i < TEST_ACKS_NUM_ITERATIONS; ++i )
    {
        uint8_t dummy_packet[8];
        memset( dummy_packet, 0, sizeof( dummy_packet ) );

        endpoint_send_packet( context.sender, dummy_packet, sizeof( dummy_packet ) );
        endpoint_send_packet( context.receiver, dummy_packet, sizeof( dummy_packet ) );

        endpoint_update( context.sender, time );
        endpoint_update( context.receiver, time );

        time += delta_time;
    }

    uint8_t sender_acked_packet[TEST_ACKS_NUM_ITERATIONS];
    memset( sender_acked_packet, 0, sizeof( sender_acked_packet ) );
    int sender_num_acks;
    uint16_t * sender_acks = endpoint_get_acks( context.sender, &sender_num_acks );
    for ( i = 0; i < sender_num_acks; ++i )
    {
        if ( sender_acks[i] < TEST_ACKS_NUM_ITERATIONS )
        {
            sender_acked_packet[sender_acks[i]] = 1;
        }
    }
    for ( i = 0; i < TEST_ACKS_NUM_ITERATIONS / 2; ++i )
    {
        check( sender_acked_packet[i] == 1 );
    }

    uint8_t receiver_acked_packet[TEST_ACKS_NUM_ITERATIONS];
    memset( receiver_acked_packet, 0, sizeof( receiver_acked_packet ) );
    int receiver_num_acks;
    uint16_t * receiver_acks = endpoint_get_acks( context.sender, &receiver_num_acks );
    for ( i = 0; i < receiver_num_acks; ++i )
    {
        if ( receiver_acks[i] < TEST_ACKS_NUM_ITERATIONS )
            receiver_acked_packet[receiver_acks[i]] = 1;
    }
    for ( i = 0; i < TEST_ACKS_NUM_ITERATIONS / 2; ++i )
    {
        check( receiver_acked_packet[i] == 1 );
    }

    endpoint_destroy( context.sender );
    endpoint_destroy( context.receiver );
}

static void test_acks_packet_loss()
{
    double time = 100.0;

    struct test_context_t context;
    memset( &context, 0, sizeof( context ) );
    
    struct config_t sender_config;
    struct config_t receiver_config;

    default_config( &sender_config );
    default_config( &receiver_config );

    sender_config.context = &context;
    sender_config.index = 0;
    sender_config.transmit_packet_function = &test_transmit_packet_function;
    sender_config.process_packet_function = &test_process_packet_function;

    receiver_config.context = &context;
    receiver_config.index = 1;
    receiver_config.transmit_packet_function = &test_transmit_packet_function;
    receiver_config.process_packet_function = &test_process_packet_function;

    context.sender = endpoint_create( &sender_config, time );
    context.receiver = endpoint_create( &receiver_config, time );

    const double delta_time = 0.1f;

    int i;
    for ( i = 0; i < TEST_ACKS_NUM_ITERATIONS; ++i )
    {
        uint8_t dummy_packet[8];
        memset( dummy_packet, 0, sizeof( dummy_packet ) );

        context.drop = ( i % 2 );

        endpoint_send_packet( context.sender, dummy_packet, sizeof( dummy_packet ) );
        endpoint_send_packet( context.receiver, dummy_packet, sizeof( dummy_packet ) );

        endpoint_update( context.sender, time );
        endpoint_update( context.receiver, time );

        time += delta_time;
    }

    uint8_t sender_acked_packet[TEST_ACKS_NUM_ITERATIONS];
    memset( sender_acked_packet, 0, sizeof( sender_acked_packet ) );
    int sender_num_acks;
    uint16_t * sender_acks = endpoint_get_acks( context.sender, &sender_num_acks );
    for ( i = 0; i < sender_num_acks; ++i )
    {
        if ( sender_acks[i] < TEST_ACKS_NUM_ITERATIONS )
        {
            sender_acked_packet[sender_acks[i]] = 1;
        }
    }
    for ( i = 0; i < TEST_ACKS_NUM_ITERATIONS / 2; ++i )
    {
        check( sender_acked_packet[i] == (i+1) % 2 );
    }

    uint8_t receiver_acked_packet[TEST_ACKS_NUM_ITERATIONS];
    memset( receiver_acked_packet, 0, sizeof( receiver_acked_packet ) );
    int receiver_num_acks;
    uint16_t * receiver_acks = endpoint_get_acks( context.sender, &receiver_num_acks );
    for ( i = 0; i < receiver_num_acks; ++i )
    {
        if ( receiver_acks[i] < TEST_ACKS_NUM_ITERATIONS )
        {
            receiver_acked_packet[receiver_acks[i]] = 1;
        }
    }
    for ( i = 0; i < TEST_ACKS_NUM_ITERATIONS / 2; ++i )
    {
        check( receiver_acked_packet[i] == (i+1) % 2 );
    }

    endpoint_destroy( context.sender );
    endpoint_destroy( context.receiver );
}

#define TEST_MAX_PACKET_BYTES (4*1024)

static int generate_packet_data( uint16_t sequence, uint8_t * packet_data )
{
    int packet_bytes = ( ( (int)sequence * 1023 ) % ( TEST_MAX_PACKET_BYTES - 2 ) ) + 2;
    assert( packet_bytes >= 2 );
    assert( packet_bytes <= TEST_MAX_PACKET_BYTES );
    packet_data[0] = (uint8_t) ( sequence & 0xFF );
    packet_data[1] = (uint8_t) ( (sequence>>8) & 0xFF );
    int i;
    for ( i = 2; i < packet_bytes; ++i )
    {
        packet_data[i] = (uint8_t) ( ( (int)i + sequence ) % 256 );
    }
    return packet_bytes;
}

static void validate_packet_data( uint8_t * packet_data, int packet_bytes )
{
    assert( packet_bytes >= 2 );
    assert( packet_bytes <= TEST_MAX_PACKET_BYTES );
    uint16_t sequence = 0;
    sequence |= (uint16_t) packet_data[0];
    sequence |= ( (uint16_t) packet_data[1] ) << 8;
    check( packet_bytes == ( ( (int)sequence * 1023 ) % ( TEST_MAX_PACKET_BYTES - 2 ) ) + 2 );
    int i;
    for ( i = 2; i < packet_bytes; ++i )
    {
        check( packet_data[i] == (uint8_t) ( ( (int)i + sequence ) % 256 ) );
    }
}

static int test_process_packet_function_validate( void * context, int index, uint16_t sequence, uint8_t * packet_data, int packet_bytes )
{
    assert( packet_data );
    assert( packet_bytes > 0 );
    assert( packet_bytes <= TEST_MAX_PACKET_BYTES );

    (void) context;
    (void) index;
    (void) sequence;

    validate_packet_data( packet_data, packet_bytes );

    return 1;
}

void test_packets()
{
    double time = 100.0;

    struct test_context_t context;
    memset( &context, 0, sizeof( context ) );
    
    struct config_t sender_config;
    struct config_t receiver_config;

    default_config( &sender_config );
    default_config( &receiver_config );

    sender_config.fragment_above = 500;
    receiver_config.fragment_above = 500;

#if defined(_MSC_VER)
    strcpy_s( sender_config.name, sizeof( sender_config.name ), "sender" );
#else
    strcpy( sender_config.name, "sender" );
#endif
    sender_config.context = &context;
    sender_config.index = 0;
    sender_config.transmit_packet_function = &test_transmit_packet_function;
    sender_config.process_packet_function = &test_process_packet_function_validate;

#if defined(_MSC_VER)
    strcpy_s( receiver_config.name, sizeof( receiver_config.name ), "receiver" );
#else
    strcpy( receiver_config.name, "receiver" );
#endif
    receiver_config.context = &context;
    receiver_config.index = 1;
    receiver_config.transmit_packet_function = &test_transmit_packet_function;
    receiver_config.process_packet_function = &test_process_packet_function_validate;

    context.sender = endpoint_create( &sender_config, time );
    context.receiver = endpoint_create( &receiver_config, time );

    double delta_time = 0.1;

    int i;
    for ( i = 0; i < 16; ++i )
    {
        {
            uint8_t packet_data[TEST_MAX_PACKET_BYTES];
            uint16_t sequence = endpoint_next_packet_sequence( context.sender );
            int packet_bytes = generate_packet_data( sequence, packet_data );
            endpoint_send_packet( context.sender, packet_data, packet_bytes );
        }

        {
            uint8_t packet_data[TEST_MAX_PACKET_BYTES];
            uint16_t sequence = endpoint_next_packet_sequence( context.sender );
            int packet_bytes = generate_packet_data( sequence, packet_data );
            endpoint_send_packet( context.sender, packet_data, packet_bytes );
        }

        endpoint_update( context.sender, time );
        endpoint_update( context.receiver, time );

        endpoint_clear_acks( context.sender );
        endpoint_clear_acks( context.receiver );

        time += delta_time;
    }

    endpoint_destroy( context.sender );
    endpoint_destroy( context.receiver );
}
