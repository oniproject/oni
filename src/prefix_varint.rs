


/*
#[inline]
unsafe fn prefix_len(p: *) -> u8 {
  return 1 + count_trailing_zeros_32(*p | 0x100);
}
*/

#[inline]
unsafe fn prefix_read(buf: &[u8]) -> Option<u64> {
    assert!(cfg!(target_endian = "little"), "big endian doesn't support yet");
    if buf.len() == 0 { return None; }

    let prefix = buf[0];
    let z = prefix.trailing_zeros() + 1;
    debug_assert!(z >= 1 && z <= 9, "bad prefix: {}", z);

    if buf.len() < z as usize {
        return None
    }

    let p = buf.as_ptr() as *const u64;
    Some(if z == 9 {
        unsafe { p.add(1).read_unaligned() }
    } else {
        let u = 64 - 8 * z;
        (unsafe { p.read_unaligned() } << u) >> (u + z)
    })
}


#[inline]
unsafe fn prefix_write(out: &mut Vec<u8>, mut x: u64) {
    let bits = 64 - (x | 1).leading_zeros();
    let mut bytes = 1 + (bits - 1) / 7;
    if bits > 56 {
        out.push(0);
        bytes = 8;
    } else {
        x = (2 * x + 1) << (bytes - 1);
    }
    for _ in 0..bytes {
        out.push((x & 0xff) as u8);
        x >>= 8;
    }
}

#[test]
#[ignore]
fn prefix_varint() {
    let tests: &[(u64, usize)] = &[
        (0x0000_0000_0000_0000, 1),
        (0x0000_0000_0000_0001, 1),
        (0x0000_0000_0000_007F, 1),
        (0x0000_0000_0000_0080, 2),
        (0x0000_0000_0000_3FFF, 2),
        (0x0000_0000_0000_4000, 3),
        (0x0000_0000_001F_FFFF, 3),
        (0x0000_0000_0020_0000, 4),
        (0x0000_0000_0FFF_FFFF, 4),
        (0x0000_0000_1000_0000, 5),
        (0x0000_0007_FFFF_FFFF, 5),
        (0x0000_0008_0000_0000, 6),
        (0x0000_03FF_FFFF_FFFF, 6),
        (0x0000_0400_0000_0000, 7),
        (0x0001_FFFF_FFFF_FFFF, 7),
        (0x0002_0000_0000_0000, 8),
        (0x00FF_FFFF_FFFF_FFFF, 8),
        (0x0100_0000_0000_0000, 9),
        (0xFFFF_FFFF_FFFF_FFFe, 9),

        (0xFFFF_FFFF_FFFF_FFFF, 9),
    ];

    unsafe {
        for (value, expected_len) in tests.iter().cloned() {
            let mut out = Vec::new();
            prefix_write(&mut out, value);
            assert_eq!(out.len(), expected_len);

            assert_eq!(prefix_read(&out), Some(value));
        }
    }

    /*
    for (value, expected_len) in tests.iter().cloned() {
        let buf = &mut [0u8; 9];
        let bytes = prefix_encode(value, buf);
        assert_eq!(bytes, expected_len);

        let input = &buf[..bytes];
        {
            let input = input.as_ptr();
            let len = unsafe { prefix_length(input) };
            assert_eq!(len, expected_len);

            let out = unsafe { prefix_get(input, len) };
            assert_eq!(out, value);
        }

        assert_eq!(prefix_decode(&input), Some(value));
    }
    */
}
