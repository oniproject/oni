var N = null;var searchIndex = {};
searchIndex["oni"]={"doc":"A simple protocol for secure client/server connections over UDP.","items":[[3,"ReplayProtection","oni","",N,N],[3,"Client","","",N,N],[3,"Server","","",N,N],[3,"Connection","","",N,N],[3,"ServerList","","",N,N],[3,"Incoming","","",N,N],[3,"SimulatedSocket","","A simulated socket.",N,N],[3,"SimulatorConfig","","",N,N],[12,"latency","","",0,N],[12,"jitter","","",0,N],[12,"loss","","",0,N],[4,"State","","",N,N],[13,"Disconnected","","",1,N],[13,"Connecting","","",1,N],[13,"Connected","","",1,N],[13,"Failed","","",1,N],[4,"ConnectingState","","",N,N],[13,"SendingRequest","","",2,N],[13,"SendingResponse","","",2,N],[4,"Error","","",N,N],[13,"ConnectTokenExpired","","",3,N],[13,"InvalidConnectToken","","",3,N],[13,"ConnectionTimedOut","","",3,N],[13,"ConnectionResponseTimedOut","","",3,N],[13,"ConnectionRequestTimedOut","","",3,N],[13,"ConnectionDenied","","",3,N],[5,"config_socket","","",N,[[["socketaddr"],["socketaddr"],["option",["simulatorconfig"]]]]],[5,"unix_time","","",N,[[],["u64"]]],[11,"new","","",4,[[["u64"],["publictoken"],["socketaddr"]],["result"]]],[11,"simulated","","",4,[[["u64"],["publictoken"]],["self"]]],[11,"with_socket","","",4,[[["u64"],["publictoken"],["s"]],["result"]]],[11,"state","","",4,[[["self"]],["state"]]],[11,"is_connected","","",4,[[["self"]],["bool"]]],[11,"local_addr","","",4,[[["self"]],["result",["socketaddr"]]]],[11,"connect","","",4,[[["self"],["socketaddr"]],["result"]]],[11,"recv","","",4,[[["self"]],["option"]]],[11,"close","","",4,[[["self"]]]],[11,"update","","",4,[[["self"]]]],[11,"send","","",4,N],[11,"id","","",5,[[["self"]],["u64"]]],[11,"addr","","",5,[[["self"]],["socketaddr"]]],[11,"is_closed","","",5,[[["self"]],["bool"]]],[11,"close","","",5,[[["self"]]]],[11,"recv","","",5,N],[11,"send","","",5,N],[11,"new","","",6,N],[11,"simulated","","",6,N],[11,"with_socket","","",6,N],[11,"local_addr","","",6,[[["self"]],["socketaddr"]]],[11,"update","","",6,[[["self"],["f"]]]],[11,"new","","",7,[[],["self"]]],[11,"push","","",7,[[["self"],["socketaddr"]],["result",["socketaddr"]]]],[11,"contains","","",7,[[["self"],["socketaddr"]],["bool"]]],[11,"as_slice","","",7,N],[11,"deserialize","","",7,N],[11,"serialize","","",7,[[["self"]],["option"]]],[11,"serialize_noalloc","","",7,[[["self"],["vec"]],["option"]]],[11,"new","","",8,N],[11,"open_request","","",8,[[["self"],["request"]],["result"]]],[11,"open_response","","",8,N],[11,"gen_challenge","","",8,N],[11,"remove","","",8,[[["self"],["socketaddr"]],["option",["keypair"]]]],[11,"insert","","",8,[[["self"],["socketaddr"],["u64"],["privatetoken"]]]],[11,"add_token_history","","",8,N],[11,"update","","",8,[[["self"]]]],[11,"new","","",9,[[],["self"]]],[11,"already_received","","",9,[[["self"],["u64"]],["bool"]]],[11,"new","","",10,[[],["self"]]],[11,"take_send_bytes","","Takes the value of the counter sent bytes and clear counter.",10,[[["self"]],["usize"]]],[11,"take_recv_bytes","","Takes the value of the counter received bytes and clear counter.",10,[[["self"]],["usize"]]],[11,"bind","","",10,[[["socketaddr"]],["result"]]],[11,"local_addr","","",10,[[["self"]],["socketaddr"]]],[11,"connect","","",10,[[["self"],["socketaddr"]]]],[11,"send","","",10,N],[11,"recv","","",10,N],[11,"send_to","","",10,N],[11,"recv_from","","",10,N],[0,"prefix_varint","","",N,N],[5,"read_z","oni::prefix_varint","",N,[[["u8"]],["u32"]]],[5,"read_varint64_unchecked","","`z >= 1 && z <= 9`",N,N],[5,"read_varint56_unchecked","","`z >= 1 && z <= 8`",N,N],[5,"read_varint","","",N,N],[5,"write_varint","","",N,N],[8,"WritePrefixVarint","","",N,N],[11,"write_prefix_varint","","",11,[[["self"],["u64"]],["result"]]],[11,"write_prefix_varint_custom","","",11,[[["self"],["u64"],["u32"]],["result"]]],[0,"bitset","oni","",N,N],[3,"BitSet","oni::bitset","",N,N],[6,"BitSet8","","",N,N],[6,"BitSet16","","",N,N],[6,"BitSet32","","",N,N],[6,"BitSet64","","",N,N],[6,"BitSet128","","",N,N],[6,"BitSet256","","",N,N],[6,"BitSet512","","",N,N],[6,"BitSet1024","","",N,N],[11,"new","","",12,[[],["self"]]],[11,"len","","",12,[[["self"]],["usize"]]],[11,"as_slice","","",12,N],[11,"get","","",12,[[["self"],["usize"]],["bool"]]],[11,"set","","",12,[[["self"],["usize"]]]],[11,"clear","","",12,[[["self"],["usize"]]]],[11,"get_unchecked","","",12,[[["self"],["usize"]],["bool"]]],[11,"set_unchecked","","",12,[[["self"],["usize"]]]],[11,"clear_unchecked","","",12,[[["self"],["usize"]]]],[11,"to_bytes","","",13,N],[11,"to_u8","","",13,[[["self"]],["u8"]]],[11,"to_bytes","","",14,N],[11,"to_u16","","",14,[[["self"]],["u16"]]],[11,"to_bytes","","",15,N],[11,"to_u32","","",15,[[["self"]],["u32"]]],[11,"to_bytes","","",16,N],[11,"to_u64","","",16,[[["self"]],["u64"]]],[11,"to_bytes","","",17,N],[11,"to_u128","","",17,[[["self"]],["u128"]]],[0,"token","oni","",N,N],[3,"ChallengeToken","oni::token","",N,N],[3,"PrivateToken","","",N,N],[3,"PublicToken","","Format:",N,N],[17,"DATA","","",N,N],[17,"USER","","",N,N],[17,"CHALLENGE_LEN","","",N,N],[17,"PRIVATE_LEN","","",N,N],[17,"PUBLIC_LEN","","",N,N],[11,"new","","",18,N],[11,"client_id","","",18,[[["self"]],["u64"]]],[11,"user","","",18,N],[11,"encode_packet","","",18,N],[11,"decode_packet","","",18,N],[11,"seal","","",18,N],[11,"open","","",18,N],[11,"generate","","",19,N],[11,"hmac","","",19,N],[11,"client_id","","",19,[[["self"]],["u64"]]],[11,"timeout","","",19,[[["self"]],["u32"]]],[11,"client_key","","",19,N],[11,"server_key","","",19,N],[11,"data","","",19,N],[11,"user","","",19,N],[11,"seal","","",19,N],[11,"open","","",19,N],[11,"protocol_id","","",20,[[["self"]],["u64"]]],[11,"create_timestamp","","",20,[[["self"]],["u64"]]],[11,"expire_timestamp","","",20,[[["self"]],["u64"]]],[11,"timeout_seconds","","",20,[[["self"]],["u32"]]],[11,"nonce","","",20,N],[11,"client_key","","",20,N],[11,"server_key","","",20,N],[11,"token","","",20,N],[11,"data","","",20,N],[11,"check_version","","",20,[[["self"]],["bool"]]],[11,"as_slice","","",20,N],[11,"into_vec","","",20,[[["self"]],["vec",["u8"]]]],[11,"generate","","",20,N],[0,"protocol","oni","Overview:",N,N],[3,"Request","oni::protocol","",N,N],[4,"Packet","","",N,N],[13,"Payload","","",21,N],[12,"buf","oni::protocol::Packet","Contains `[ciphertext]`.",21,N],[12,"seq","","Sequence number of this packet.",21,N],[12,"tag","","Contains `[hmac]`.",21,N],[13,"Handshake","oni::protocol","",21,N],[12,"prefix","oni::protocol::Packet","Prefix byte.",21,N],[12,"buf","","Contains `[ciphertext]`.",21,N],[12,"seq","","Sequence number of this packet.",21,N],[12,"tag","","Contains `[hmac]`.",21,N],[13,"Close","oni::protocol","",21,N],[12,"prefix","oni::protocol::Packet","Prefix byte.",21,N],[12,"seq","","Sequence number of this packet.",21,N],[12,"tag","","Contains `[hmac]`.",21,N],[13,"Request","oni::protocol","",21,N],[17,"VERSION","","Protocol version.",N,N],[17,"VERSION_LEN","","Protocol version length.",N,N],[17,"MTU","","Maximum Transmission Unit.",N,N],[17,"MIN_PACKET","","Minimum size of packet.",N,N],[17,"MAX_OVERHEAD","","Maximum overhead in bytes.",N,N],[17,"MAX_PAYLOAD","","Maximum length of payload in bytes.",N,N],[17,"NUM_DISCONNECT_PACKETS","","",N,N],[17,"PACKET_SEND_RATE","","",N,N],[17,"PACKET_SEND_DELTA","","",N,N],[11,"expire","","",22,[[["self"]],["u64"]]],[11,"open_token","","",22,N],[11,"is_valid","","",22,[[["self"],["u64"],["u64"]],["bool"]]],[11,"new","","",22,N],[11,"write","","",22,N],[11,"encode_close","","",21,N],[11,"encode_handshake","","",21,N],[11,"encode_keep_alive","","",21,N],[11,"encode_payload","","",21,N],[11,"decode","","",21,N],[11,"seal","","",21,N],[11,"open","","",21,N],[0,"crypto","oni","",N,N],[3,"ChaCha20","oni::crypto","",N,N],[12,"state","","",23,N],[3,"Poly1305","","",N,N],[3,"AutoNonce","","",N,N],[12,"0","","",24,N],[5,"hchacha20","","",N,N],[5,"seal","","Performs inplace encryption using ChaCha20Poly1305 IETF.",N,N],[5,"open","","Performs inplace decryption using ChaCha20Poly1305 IETF.",N,N],[5,"xopen","","",N,N],[5,"xseal","","",N,N],[5,"nonce_from_u64","","",N,[[["u64"]],["nonce"]]],[5,"crypto_random","","",N,N],[5,"keygen","","",N,N],[0,"aead","","",N,N],[5,"seal","oni::crypto::aead","",N,N],[5,"seal_inplace","","",N,N],[5,"verify","","",N,N],[5,"open","","",N,N],[5,"ietf_seal","","",N,N],[5,"ietf_verify","","",N,N],[5,"ietf_open","","",N,N],[18,"KEYBYTES","oni::crypto","",23,N],[18,"NONCEBYTES","","",23,N],[18,"IETF_NONCEBYTES","","",23,N],[11,"new","","",23,N],[11,"new_basic","","",23,N],[11,"new_ietf","","",23,N],[11,"stream","","",23,N],[11,"stream_xor","","",23,N],[11,"stream_ietf","","",23,N],[11,"ietf","","",23,N],[11,"stream_ietf_xor","","",23,N],[11,"inplace","","",23,N],[18,"BYTES","","",25,N],[18,"KEYBYTES","","",25,N],[11,"statebytes","","",25,[[],["usize"]]],[11,"bytes","","",25,[[],["usize"]]],[11,"keybytes","","",25,[[],["usize"]]],[11,"new","","",25,[[],["self"]]],[11,"with_key","","",25,N],[11,"sum","","",25,N],[11,"verify","","",25,N],[11,"init","","",25,N],[11,"finish","","",25,N],[11,"finish_verify","","",25,N],[11,"update","","",25,N],[11,"update_pad","","",25,N],[11,"update_u64","","",25,[[["self"],["u64"]]]],[11,"update_donna","","",25,N],[11,"init64","","",25,N],[11,"finish64","","",25,N],[6,"Nonce","","",N,N],[6,"Xnonce","","",N,N],[6,"Key","","",N,N],[6,"Tag","","",N,N],[17,"KEY","","Size of Key.",N,N],[17,"HMAC","","",N,N],[17,"NONCE","","Nonce size for ChaCha20Poly1305 IETF in bytes.",N,N],[17,"XNONCE","","Nonce size for XChaCha20Poly1305 IETF in bytes.",N,N],[11,"generate","","",24,[[],["self"]]],[11,"split","","",24,N],[8,"Socket","oni","",N,N],[10,"bind","","Creates a socket from the given address.",26,[[["socketaddr"]],["result"]]],[10,"local_addr","","Returns the socket address that this socket was created from.",26,[[["self"]],["result",["socketaddr"]]]],[10,"recv_from","","Receives a single datagram message on the socket. On success, returns the number of bytes read and the origin.",26,N],[10,"send_to","","Sends data on the socket to the given address. On success, returns the number of bytes written.",26,N],[10,"connect","","",26,[[["self"],["socketaddr"]],["result"]]],[10,"send","","",26,N],[10,"recv","","",26,N],[10,"set_nonblocking","","Simulated socket Does nothing.",26,[[["self"],["bool"]],["result"]]],[11,"from","","",9,[[["t"]],["t"]]],[11,"into","","",9,[[["self"]],["u"]]],[11,"try_from","","",9,[[["u"]],["result"]]],[11,"borrow","","",9,[[["self"]],["t"]]],[11,"get_type_id","","",9,[[["self"]],["typeid"]]],[11,"try_into","","",9,[[["self"]],["result"]]],[11,"borrow_mut","","",9,[[["self"]],["t"]]],[11,"from","","",4,[[["t"]],["t"]]],[11,"into","","",4,[[["self"]],["u"]]],[11,"try_from","","",4,[[["u"]],["result"]]],[11,"borrow","","",4,[[["self"]],["t"]]],[11,"get_type_id","","",4,[[["self"]],["typeid"]]],[11,"try_into","","",4,[[["self"]],["result"]]],[11,"borrow_mut","","",4,[[["self"]],["t"]]],[11,"from","","",6,[[["t"]],["t"]]],[11,"into","","",6,[[["self"]],["u"]]],[11,"try_from","","",6,[[["u"]],["result"]]],[11,"borrow","","",6,[[["self"]],["t"]]],[11,"get_type_id","","",6,[[["self"]],["typeid"]]],[11,"try_into","","",6,[[["self"]],["result"]]],[11,"borrow_mut","","",6,[[["self"]],["t"]]],[11,"from","","",5,[[["t"]],["t"]]],[11,"into","","",5,[[["self"]],["u"]]],[11,"try_from","","",5,[[["u"]],["result"]]],[11,"borrow","","",5,[[["self"]],["t"]]],[11,"get_type_id","","",5,[[["self"]],["typeid"]]],[11,"try_into","","",5,[[["self"]],["result"]]],[11,"borrow_mut","","",5,[[["self"]],["t"]]],[11,"from","","",7,[[["t"]],["t"]]],[11,"into","","",7,[[["self"]],["u"]]],[11,"try_from","","",7,[[["u"]],["result"]]],[11,"borrow","","",7,[[["self"]],["t"]]],[11,"get_type_id","","",7,[[["self"]],["typeid"]]],[11,"try_into","","",7,[[["self"]],["result"]]],[11,"borrow_mut","","",7,[[["self"]],["t"]]],[11,"from","","",8,[[["t"]],["t"]]],[11,"into","","",8,[[["self"]],["u"]]],[11,"try_from","","",8,[[["u"]],["result"]]],[11,"borrow","","",8,[[["self"]],["t"]]],[11,"get_type_id","","",8,[[["self"]],["typeid"]]],[11,"try_into","","",8,[[["self"]],["result"]]],[11,"borrow_mut","","",8,[[["self"]],["t"]]],[11,"from","","",10,[[["t"]],["t"]]],[11,"into","","",10,[[["self"]],["u"]]],[11,"try_from","","",10,[[["u"]],["result"]]],[11,"borrow","","",10,[[["self"]],["t"]]],[11,"get_type_id","","",10,[[["self"]],["typeid"]]],[11,"try_into","","",10,[[["self"]],["result"]]],[11,"borrow_mut","","",10,[[["self"]],["t"]]],[11,"from","","",0,[[["t"]],["t"]]],[11,"into","","",0,[[["self"]],["u"]]],[11,"to_owned","","",0,[[["self"]],["t"]]],[11,"clone_into","","",0,N],[11,"try_from","","",0,[[["u"]],["result"]]],[11,"borrow","","",0,[[["self"]],["t"]]],[11,"get_type_id","","",0,[[["self"]],["typeid"]]],[11,"try_into","","",0,[[["self"]],["result"]]],[11,"borrow_mut","","",0,[[["self"]],["t"]]],[11,"from","","",1,[[["t"]],["t"]]],[11,"into","","",1,[[["self"]],["u"]]],[11,"to_owned","","",1,[[["self"]],["t"]]],[11,"clone_into","","",1,N],[11,"try_from","","",1,[[["u"]],["result"]]],[11,"borrow","","",1,[[["self"]],["t"]]],[11,"get_type_id","","",1,[[["self"]],["typeid"]]],[11,"try_into","","",1,[[["self"]],["result"]]],[11,"borrow_mut","","",1,[[["self"]],["t"]]],[11,"from","","",2,[[["t"]],["t"]]],[11,"into","","",2,[[["self"]],["u"]]],[11,"to_owned","","",2,[[["self"]],["t"]]],[11,"clone_into","","",2,N],[11,"try_from","","",2,[[["u"]],["result"]]],[11,"borrow","","",2,[[["self"]],["t"]]],[11,"get_type_id","","",2,[[["self"]],["typeid"]]],[11,"try_into","","",2,[[["self"]],["result"]]],[11,"borrow_mut","","",2,[[["self"]],["t"]]],[11,"from","","",3,[[["t"]],["t"]]],[11,"into","","",3,[[["self"]],["u"]]],[11,"to_owned","","",3,[[["self"]],["t"]]],[11,"clone_into","","",3,N],[11,"try_from","","",3,[[["u"]],["result"]]],[11,"borrow","","",3,[[["self"]],["t"]]],[11,"get_type_id","","",3,[[["self"]],["typeid"]]],[11,"try_into","","",3,[[["self"]],["result"]]],[11,"borrow_mut","","",3,[[["self"]],["t"]]],[11,"from","oni::bitset","",12,[[["t"]],["t"]]],[11,"into","","",12,[[["self"]],["u"]]],[11,"to_owned","","",12,[[["self"]],["t"]]],[11,"clone_into","","",12,N],[11,"try_from","","",12,[[["u"]],["result"]]],[11,"borrow","","",12,[[["self"]],["t"]]],[11,"get_type_id","","",12,[[["self"]],["typeid"]]],[11,"try_into","","",12,[[["self"]],["result"]]],[11,"borrow_mut","","",12,[[["self"]],["t"]]],[11,"from","oni::token","",18,[[["t"]],["t"]]],[11,"into","","",18,[[["self"]],["u"]]],[11,"to_owned","","",18,[[["self"]],["t"]]],[11,"clone_into","","",18,N],[11,"try_from","","",18,[[["u"]],["result"]]],[11,"borrow","","",18,[[["self"]],["t"]]],[11,"get_type_id","","",18,[[["self"]],["typeid"]]],[11,"try_into","","",18,[[["self"]],["result"]]],[11,"borrow_mut","","",18,[[["self"]],["t"]]],[11,"from","","",19,[[["t"]],["t"]]],[11,"into","","",19,[[["self"]],["u"]]],[11,"to_owned","","",19,[[["self"]],["t"]]],[11,"clone_into","","",19,N],[11,"try_from","","",19,[[["u"]],["result"]]],[11,"borrow","","",19,[[["self"]],["t"]]],[11,"get_type_id","","",19,[[["self"]],["typeid"]]],[11,"try_into","","",19,[[["self"]],["result"]]],[11,"borrow_mut","","",19,[[["self"]],["t"]]],[11,"from","","",20,[[["t"]],["t"]]],[11,"into","","",20,[[["self"]],["u"]]],[11,"to_owned","","",20,[[["self"]],["t"]]],[11,"clone_into","","",20,N],[11,"try_from","","",20,[[["u"]],["result"]]],[11,"borrow","","",20,[[["self"]],["t"]]],[11,"get_type_id","","",20,[[["self"]],["typeid"]]],[11,"try_into","","",20,[[["self"]],["result"]]],[11,"borrow_mut","","",20,[[["self"]],["t"]]],[11,"from","oni::protocol","",22,[[["t"]],["t"]]],[11,"into","","",22,[[["self"]],["u"]]],[11,"try_from","","",22,[[["u"]],["result"]]],[11,"borrow","","",22,[[["self"]],["t"]]],[11,"get_type_id","","",22,[[["self"]],["typeid"]]],[11,"try_into","","",22,[[["self"]],["result"]]],[11,"borrow_mut","","",22,[[["self"]],["t"]]],[11,"from","","",21,[[["t"]],["t"]]],[11,"into","","",21,[[["self"]],["u"]]],[11,"try_from","","",21,[[["u"]],["result"]]],[11,"borrow","","",21,[[["self"]],["t"]]],[11,"get_type_id","","",21,[[["self"]],["typeid"]]],[11,"try_into","","",21,[[["self"]],["result"]]],[11,"borrow_mut","","",21,[[["self"]],["t"]]],[11,"from","oni::crypto","",23,[[["t"]],["t"]]],[11,"into","","",23,[[["self"]],["u"]]],[11,"to_owned","","",23,[[["self"]],["t"]]],[11,"clone_into","","",23,N],[11,"try_from","","",23,[[["u"]],["result"]]],[11,"borrow","","",23,[[["self"]],["t"]]],[11,"get_type_id","","",23,[[["self"]],["typeid"]]],[11,"try_into","","",23,[[["self"]],["result"]]],[11,"borrow_mut","","",23,[[["self"]],["t"]]],[11,"from","","",25,[[["t"]],["t"]]],[11,"into","","",25,[[["self"]],["u"]]],[11,"try_from","","",25,[[["u"]],["result"]]],[11,"borrow","","",25,[[["self"]],["t"]]],[11,"get_type_id","","",25,[[["self"]],["typeid"]]],[11,"try_into","","",25,[[["self"]],["result"]]],[11,"borrow_mut","","",25,[[["self"]],["t"]]],[11,"from","","",24,[[["t"]],["t"]]],[11,"into","","",24,[[["self"]],["u"]]],[11,"try_from","","",24,[[["u"]],["result"]]],[11,"borrow","","",24,[[["self"]],["t"]]],[11,"get_type_id","","",24,[[["self"]],["typeid"]]],[11,"try_into","","",24,[[["self"]],["result"]]],[11,"borrow_mut","","",24,[[["self"]],["t"]]],[11,"bind","oni","",10,[[["socketaddr"]],["result"]]],[11,"connect","","",10,[[["self"],["socketaddr"]],["result"]]],[11,"local_addr","","",10,[[["self"]],["result",["socketaddr"]]]],[11,"recv_from","","",10,N],[11,"send_to","","",10,N],[11,"send","","",10,N],[11,"recv","","",10,N],[11,"set_nonblocking","","",10,[[["self"],["bool"]],["result"]]],[11,"clone","","",2,[[["self"]],["connectingstate"]]],[11,"clone","","",3,[[["self"]],["error"]]],[11,"clone","","",1,[[["self"]],["state"]]],[11,"clone","","",0,[[["self"]],["simulatorconfig"]]],[11,"clone","oni::bitset","",12,[[["self"]],["bitset"]]],[11,"clone","oni::token","",18,[[["self"]],["challengetoken"]]],[11,"clone","","",19,[[["self"]],["privatetoken"]]],[11,"clone","","",20,[[["self"]],["publictoken"]]],[11,"clone","oni::crypto","",23,[[["self"]],["chacha20"]]],[11,"from","oni","",13,[[["u8"]],["self"]]],[11,"from","","",14,[[["u16"]],["self"]]],[11,"from","","",15,[[["u32"]],["self"]]],[11,"from","","",16,[[["u64"]],["self"]]],[11,"from","","",17,[[["u128"]],["self"]]],[11,"drop","","",10,[[["self"]]]],[11,"partial_cmp","","",2,[[["self"],["connectingstate"]],["option",["ordering"]]]],[11,"partial_cmp","","",3,[[["self"],["error"]],["option",["ordering"]]]],[11,"partial_cmp","","",1,[[["self"],["state"]],["option",["ordering"]]]],[11,"lt","","",1,[[["self"],["state"]],["bool"]]],[11,"le","","",1,[[["self"],["state"]],["bool"]]],[11,"gt","","",1,[[["self"],["state"]],["bool"]]],[11,"ge","","",1,[[["self"],["state"]],["bool"]]],[11,"default","","",7,[[],["serverlist"]]],[11,"default","","",9,[[],["replayprotection"]]],[11,"default","","",0,[[],["simulatorconfig"]]],[11,"default","oni::bitset","",12,[[],["bitset"]]],[11,"eq","oni","",2,[[["self"],["connectingstate"]],["bool"]]],[11,"eq","","",3,[[["self"],["error"]],["bool"]]],[11,"eq","","",1,[[["self"],["state"]],["bool"]]],[11,"ne","","",1,[[["self"],["state"]],["bool"]]],[11,"eq","","",5,[[["self"],["self"]],["bool"]]],[11,"eq","oni::protocol","",22,[[["self"],["self"]],["bool"]]],[11,"eq","","",21,[[["self"],["self"]],["bool"]]],[11,"fmt","oni","",2,[[["self"],["formatter"]],["result"]]],[11,"fmt","","",3,[[["self"],["formatter"]],["result"]]],[11,"fmt","","",1,[[["self"],["formatter"]],["result"]]],[11,"fmt","","",0,[[["self"],["formatter"]],["result"]]],[11,"fmt","oni::protocol","",22,[[["self"],["formatter"]],["result"]]],[11,"fmt","","",21,[[["self"],["formatter"]],["result"]]],[11,"hash","oni","",5,[[["self"],["h"]]]]],"paths":[[3,"SimulatorConfig"],[4,"State"],[4,"ConnectingState"],[4,"Error"],[3,"Client"],[3,"Connection"],[3,"Server"],[3,"ServerList"],[3,"Incoming"],[3,"ReplayProtection"],[3,"SimulatedSocket"],[8,"WritePrefixVarint"],[3,"BitSet"],[6,"BitSet8"],[6,"BitSet16"],[6,"BitSet32"],[6,"BitSet64"],[6,"BitSet128"],[3,"ChallengeToken"],[3,"PrivateToken"],[3,"PublicToken"],[4,"Packet"],[3,"Request"],[3,"ChaCha20"],[3,"AutoNonce"],[3,"Poly1305"],[8,"Socket"]]};
searchIndex["oni_simulator"]={"doc":"Example","items":[[3,"Config","oni_simulator","",N,N],[12,"latency","","",0,N],[12,"jitter","","",0,N],[12,"loss","","",0,N],[12,"duplicate","","",0,N],[3,"Simulator","","Network simulator.",N,N],[3,"Socket","","Simulated unreliable unordered connectionless UDP-like socket.",N,N],[11,"new","","Constructs a new, empty `Simulator`.",1,[[],["self"]]],[11,"with_capacity","","Constructs a new, empty `Simulator` with the specified capacity.",1,[[["usize"]],["self"]]],[11,"add_socket","","Creates a socket from the given address.",1,[[["self"],["socketaddr"]],["socket"]]],[11,"add_socket_with_name","","Creates a named socket from the given address.",1,[[["self"],["socketaddr"],["str"]],["socket"]]],[11,"add_mapping","","",1,[[["self"],["socketaddr"],["a"],["config"]]]],[11,"remove_mapping","","",1,[[["self"],["socketaddr"],["a"]]]],[11,"advance","","Advance network simulator time.",1,[[["self"]]]],[11,"clear","","Discard all payloads in the network simulator.",1,[[["self"]]]],[11,"take_send_bytes","","Takes the value of the counter sent bytes and clear counter.",2,[[["self"]],["usize"]]],[11,"take_recv_bytes","","Takes the value of the counter received bytes and clear counter.",2,[[["self"]],["usize"]]],[11,"local_addr","","Returns the socket address that this socket was created from.",2,[[["self"]],["socketaddr"]]],[11,"send_to","","Sends data on the socket to the given address. On success, returns the number of bytes written.",2,N],[11,"recv_from","","Receives a single datagram message on the socket. On success, returns the number of bytes read and the origin.",2,N],[6,"DefaultMTU","","By default MTU is 1200 bytes.",N,N],[11,"from","","",0,[[["t"]],["t"]]],[11,"into","","",0,[[["self"]],["u"]]],[11,"to_owned","","",0,[[["self"]],["t"]]],[11,"clone_into","","",0,N],[11,"try_from","","",0,[[["u"]],["result"]]],[11,"borrow","","",0,[[["self"]],["t"]]],[11,"get_type_id","","",0,[[["self"]],["typeid"]]],[11,"borrow_mut","","",0,[[["self"]],["t"]]],[11,"try_into","","",0,[[["self"]],["result"]]],[11,"from","","",1,[[["t"]],["t"]]],[11,"into","","",1,[[["self"]],["u"]]],[11,"to_owned","","",1,[[["self"]],["t"]]],[11,"clone_into","","",1,N],[11,"try_from","","",1,[[["u"]],["result"]]],[11,"borrow","","",1,[[["self"]],["t"]]],[11,"get_type_id","","",1,[[["self"]],["typeid"]]],[11,"borrow_mut","","",1,[[["self"]],["t"]]],[11,"try_into","","",1,[[["self"]],["result"]]],[11,"from","","",2,[[["t"]],["t"]]],[11,"into","","",2,[[["self"]],["u"]]],[11,"try_from","","",2,[[["u"]],["result"]]],[11,"borrow","","",2,[[["self"]],["t"]]],[11,"get_type_id","","",2,[[["self"]],["typeid"]]],[11,"borrow_mut","","",2,[[["self"]],["t"]]],[11,"try_into","","",2,[[["self"]],["result"]]],[11,"clone","","",0,[[["self"]],["config"]]],[11,"clone","","",1,[[["self"]],["simulator"]]],[11,"drop","","",2,[[["self"]]]],[11,"default","","",0,[[],["config"]]],[11,"fmt","","",0,[[["self"],["formatter"]],["result"]]]],"paths":[[3,"Config"],[3,"Simulator"],[3,"Socket"]]};
initSearch(searchIndex);
