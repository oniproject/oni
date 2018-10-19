use oni::crypto::{KEY, XNONCE as NPUB, HMAC as TAG};
use oni::crypto::{open, seal, AutoNonce};


fn xopen(c: &mut [u8], ad: &[u8], t: [u8; TAG], n: &[u8; NPUB], k: &[u8; KEY]) -> Result<(), ()> {
    let (n, k) = AutoNonce(*n).split(k);
    open(c, Some(ad), &t, &n, &k)
}

fn xseal(m: &mut [u8], ad: &[u8], n: &[u8; NPUB], k: &[u8; KEY]) -> [u8; TAG] {
    let (n, k) = AutoNonce(*n).split(k);
    seal(m, Some(ad), &n, &k)
}

#[test]
fn smoke_xchacha20poly1305() {
    const MLEN: usize = 114;
    const ADLEN: usize = 12;

    static FIRST_KEY: [u8; KEY] = [
        0x80, 0x81, 0x82, 0x83, 0x84, 0x85, 0x86, 0x87,
        0x88, 0x89, 0x8a, 0x8b, 0x8c, 0x8d, 0x8e, 0x8f,
        0x90, 0x91, 0x92, 0x93, 0x94, 0x95, 0x96, 0x97,
        0x98, 0x99, 0x9a, 0x9b, 0x9c, 0x9d, 0x9e, 0x9f,
    ];

    static MESSAGE: &[u8; MLEN] = b"Ladies and Gentlemen of the class of '99: \
    If I could offer you only one tip for the future, sunscreen would be it.";

    static NONCE: [u8; NPUB] = [
        0x07, 0x00, 0x00, 0x00, 0x40, 0x41, 0x42, 0x43,
        0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b,
        0x4c, 0x4d, 0x4e, 0x4f, 0x50, 0x51, 0x52, 0x53,
    ];
    static AD: [u8; ADLEN] = [
        0x50, 0x51, 0x52, 0x53, 0xc0, 0xc1, 0xc2, 0xc3,
        0xc4, 0xc5, 0xc6, 0xc7,
    ];

    static EXPECTED_MESSAGE: &[u8; MLEN] = &[
        0xf8, 0xeb, 0xea, 0x48, 0x75, 0x04, 0x40, 0x66,
        0xfc, 0x16, 0x2a, 0x06, 0x04, 0xe1, 0x71, 0xfe,
        0xec, 0xfb, 0x3d, 0x20, 0x42, 0x52, 0x48, 0x56,
        0x3b, 0xcf, 0xd5, 0xa1, 0x55, 0xdc, 0xc4, 0x7b,
        0xbd, 0xa7, 0x0b, 0x86, 0xe5, 0xab, 0x9b, 0x55,
        0x00, 0x2b, 0xd1, 0x27, 0x4c, 0x02, 0xdb, 0x35,
        0x32, 0x1a, 0xcd, 0x7a, 0xf8, 0xb2, 0xe2, 0xd2,
        0x50, 0x15, 0xe1, 0x36, 0xb7, 0x67, 0x94, 0x58,
        0xe9, 0xf4, 0x32, 0x43, 0xbf, 0x71, 0x9d, 0x63,
        0x9b, 0xad, 0xb5, 0xfe, 0xac, 0x03, 0xf8, 0x0a,
        0x19, 0xa9, 0x6e, 0xf1, 0x0c, 0xb1, 0xd1, 0x53,
        0x33, 0xa8, 0x37, 0xb9, 0x09, 0x46, 0xba, 0x38,
        0x54, 0xee, 0x74, 0xda, 0x3f, 0x25, 0x85, 0xef,
        0xc7, 0xe1, 0xe1, 0x70, 0xe1, 0x7e, 0x15, 0xe5,
        0x63, 0xe7,
    ];

    static EXPECTED_TAG: &[u8; TAG] = &[
        0x76, 0x01, 0xf4, 0xf8, 0x5c, 0xaf, 0xa8, 0xe5,
        0x87, 0x76, 0x14, 0xe1, 0x43, 0xe6, 0x84, 0x20,
    ];

    let mut buf: Vec<_> = MESSAGE[..].to_owned();
    let tag = xseal(&mut buf, &AD, &NONCE, &FIRST_KEY);

    assert_eq!(&buf[..], &EXPECTED_MESSAGE[..]);
    assert_eq!(&tag[..], &EXPECTED_TAG[..]);

    xopen(&mut buf, &AD, tag, &NONCE, &FIRST_KEY).unwrap();
    assert_eq!(&buf[..], &MESSAGE[..]);
}

#[test]
fn vector_xchacha20poly1305() {
    use std::fs::File;
    use std::io::{BufReader, BufRead};

    const FILENAME: &str = "tests/xchacha20poly1305.vector";

    #[derive(Default, Clone)]
    struct Vector {
        key: Vec<u8>,
        ad: Vec<u8>,
        nonce: Vec<u8>,

        input: Vec<u8>,
        output: Vec<u8>, // with tag
    }

    let vectors = {
        let mut vectors = Vec::new();

        let file = File::open(FILENAME).unwrap();
        let file = BufReader::new(&file);
        let mut current = Vector::default();

        for line in file.lines() {
            let line = line.unwrap();
            let line = line.trim();
            if line.is_empty() || line.starts_with("#") {
                continue;
            }

            if let Some(mid) = line.find('=') {
                let (key, value) = line.split_at(mid);
                let (key, value) = (key.trim(), value.trim());
                let key: &str = &key.to_ascii_lowercase();

                let value = hex2bin(value);

                match key {
                    "key"   => current.key = value,
                    "ad"    => current.ad = value,
                    "nonce" => current.nonce = value,
                    "in"    => current.input = value,
                    "out"   => {
                        current.output = value;
                        vectors.push(current.clone());
                    }
                    _ => (),
                }
            }
        }
        vectors
    };


    for vector in &vectors {
        let ad = &vector.ad;
        let key = unsafe { &*(vector.key.as_ptr() as *const [u8; KEY]) };
        let nonce = unsafe { &*(vector.nonce.as_ptr()  as *const [u8; NPUB]) };

        let mut buf = vector.input.clone();
        let tag = xseal(&mut buf, ad, nonce, &key);

        let n = vector.output.len() - TAG;
        assert_eq!(&buf[..], &vector.output[..n]);
        assert_eq!(&tag[..], &vector.output[n..]);

        xopen(&mut buf, ad, tag, nonce, &key).unwrap();
        assert_eq!(&buf[..], &vector.input[..]);
    }

}

fn hex2bin(bin: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for (i, c) in bin.chars().enumerate() {
        if c == '=' || c == ' ' {
            continue;
        }
        buf.push(c);
        if i % 2 == 1 {
            println!("buf: '{}'", buf);
            out.push(u8::from_str_radix(&buf, 16).unwrap());
            buf.clear();
        }
    }
    out
}
