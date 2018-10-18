/// Returns `1` if and only if the two slices, `a` and `b`, have equal contents.
///
/// The time taken is a function of the length of
/// the slices and is independent of the contents.
pub fn compare(a: &[u8], b: &[u8]) -> isize {
    if a.len() != b.len() { return 0; }
    let mut v = 0;
    for i in 0..a.len() {
        v |= a[i] ^ b[i];
    }
    byte_eq(v, 0)
}
/// Returns `a` if `v == 1` and `b` if `v == 0`.
///
/// Its behavior is undefined if v takes any other value.
pub fn select(v: isize, a: isize, b: isize) -> isize {
    (!(v-1)&a) | (v-1)&b
}
/// Returns `1` if `a == b` and `0` otherwise.
pub fn byte_eq(a: u8, b: u8) -> isize {
    (u32::from(a^b).wrapping_sub(1) >> 31) as isize
}
/// Returns `1` if `a == b` and `0` otherwise.
pub fn eq(a: u32, b: u32) -> isize {
    (u64::from(u32::from(a^b).wrapping_sub(1)) >> 63) as isize
}
/// Copies the contents of `src` into `dst` (a slice of equal length) if `v == 1`.
/// If `v == 0`, `dst` is left unchanged.
///
/// Its behavior is undefined if v takes any other value.
pub fn copy(v: isize, dst: &mut [u8], src: &[u8]) {
    assert_eq!(dst.len(), src.len(), "slices have different lengths");
    let xmask = ( (v - 1)) as u8;
    let ymask = (!(v - 1)) as u8;
    for i in 0..dst.len() {
        dst[i] = dst[i] & xmask | src[i] & ymask;
    }
}
/// ConstantTimeLessOrEq returns 1 if x <= y and 0 otherwise.
/// Its behavior is undefined if x or y are negative or > 2**31 - 1.
pub fn less_or_eq(x: isize, y: isize) -> isize {
    let (x, y) = (x as i32, y as i32);
    ((x.wrapping_sub(y).wrapping_sub(1) >> 31) & 1) as isize
}

/*
type TestConstantTimeCompareStruct struct {
    a, b []byte
    out  int
}
var testConstantTimeCompareData = []TestConstantTimeCompareStruct{
    {[]byte{}, []byte{}, 1},
    {[]byte{0x11}, []byte{0x11}, 1},
    {[]byte{0x12}, []byte{0x11}, 0},
    {[]byte{0x11}, []byte{0x11, 0x12}, 0},
    {[]byte{0x11, 0x12}, []byte{0x11}, 0},
}
func TestConstantTimeCompare(t *testing.T) {
    for i, test := range testConstantTimeCompareData {
        if r := ConstantTimeCompare(test.a, test.b); r != test.out {
            t.Errorf("#%d bad result (got %x, want %x)", i, r, test.out)
        }
    }
}
type TestConstantTimeByteEqStruct struct {
    a, b uint8
    out  int
}
var testConstandTimeByteEqData = []TestConstantTimeByteEqStruct{
    {0, 0, 1},
    {0, 1, 0},
    {1, 0, 0},
    {0xff, 0xff, 1},
    {0xff, 0xfe, 0},
}
func byteEq(a, b uint8) int {
    if a == b {
        return 1
    }
    return 0
}
func TestConstantTimeByteEq(t *testing.T) {
    for i, test := range testConstandTimeByteEqData {
        if r := ConstantTimeByteEq(test.a, test.b); r != test.out {
            t.Errorf("#%d bad result (got %x, want %x)", i, r, test.out)
        }
    }
    err := quick.CheckEqual(ConstantTimeByteEq, byteEq, nil)
    if err != nil {
        t.Error(err)
    }
}
func eq(a, b int32) int {
    if a == b {
        return 1
    }
    return 0
}
func TestConstantTimeEq(t *testing.T) {
    err := quick.CheckEqual(ConstantTimeEq, eq, nil)
    if err != nil {
        t.Error(err)
    }
}
func makeCopy(v int, x, y []byte) []byte {
    if len(x) > len(y) {
        x = x[0:len(y)]
    } else {
        y = y[0:len(x)]
    }
    if v == 1 {
        copy(x, y)
    }
    return x
}
func constantTimeCopyWrapper(v int, x, y []byte) []byte {
    if len(x) > len(y) {
        x = x[0:len(y)]
    } else {
        y = y[0:len(x)]
    }
    v &= 1
    ConstantTimeCopy(v, x, y)
    return x
}
func TestConstantTimeCopy(t *testing.T) {
    err := quick.CheckEqual(constantTimeCopyWrapper, makeCopy, nil)
    if err != nil {
        t.Error(err)
    }
}
var lessOrEqTests = []struct {
    x, y, result int
}{
    {0, 0, 1},
    {1, 0, 0},
    {0, 1, 1},
    {10, 20, 1},
    {20, 10, 0},
    {10, 10, 1},
}
func TestConstantTimeLessOrEq(t *testing.T) {
    for i, test := range lessOrEqTests {
        result := ConstantTimeLessOrEq(test.x, test.y)
        if result != test.result {
            t.Errorf("#%d: %d <= %d gave %d, expected %d", i, test.x, test.y, result, test.result)
        }
    }
}
var benchmarkGlobal uint8
func BenchmarkConstantTimeByteEq(b *testing.B) {
    var x, y uint8
    for i := 0; i < b.N; i++ {
        x, y = uint8(ConstantTimeByteEq(x, y)), x
    }
    benchmarkGlobal = x
}
func BenchmarkConstantTimeEq(b *testing.B) {
    var x, y int
    for i := 0; i < b.N; i++ {
        x, y = ConstantTimeEq(int32(x), int32(y)), x
    }
    benchmarkGlobal = uint8(x)
}
func BenchmarkConstantTimeLessOrEq(b *testing.B) {
    var x, y int
    for i := 0; i < b.N; i++ {
        x, y = ConstantTimeLessOrEq(x, y), x
    }
    benchmarkGlobal = uint8(x)
}
*/
