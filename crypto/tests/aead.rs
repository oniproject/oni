use crypto::aead;

const ABYTES: usize = 32;

#[test]
fn original() {
    const MLEN: usize = 10;
    const ADLEN: usize = 10;
    const CLEN: usize = MLEN + ABYTES;

    let firstkey: [u8; ABYTES] = [
            0x42, 0x90, 0xbc, 0xb1, 0x54, 0x17, 0x35, 0x31,
            0xf3, 0x14, 0xaf, 0x57, 0xf3, 0xbe, 0x3b, 0x50,
            0x06, 0xda, 0x37, 0x1e, 0xce, 0x27, 0x2a, 0xfa,
            0x1b, 0x5d, 0xbd, 0xd1, 0x10, 0x0a, 0x10, 0x07,
    ];
    let m = [0x86, 0xd0, 0x99, 0x74, 0x84, 0x0b, 0xde, 0xd2, 0xa5, 0xca];
    let nonce = [0xcd, 0x7c, 0xf6, 0x7b, 0xe3, 0x9c, 0x79, 0x4a];
    let ad = [0x87, 0xe2, 0x29, 0xd4, 0x50, 0x08, 0x45, 0xa0, 0x79, 0xc0];

    let mut c = vec![0u8; MLEN];
    let mac = aead::seal(c.as_mut_ptr(), &m, &ad, nonce, &firstkey);

    assert_eq!(&c[..], &[0xe3,0xe4,0x46,0xf7,0xed,0xe9,0xa1,0x9b,0x62,0xa4]);
    assert_eq!(&mac[..], &[0x67,0x7d,0xab,0xf4,0xe3,0xd2,0x4b,0x87,0x6b,0xb2,0x84,0x75,0x38,0x96,0xe1,0xd6]);

    aead::verify(&c, &mac, &ad, nonce, &firstkey).unwrap();

    let mut dst = vec![0u8; MLEN];
    aead::open(&mut dst, &c, &mac, &ad, nonce, &firstkey).unwrap();
    assert_eq!(&dst, &m);
}
