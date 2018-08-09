mod data;
use crate::data::*;

use typenum::U8;
use rooms::{SpatialIndex, SpatialHashMap, Shim32};

#[test]
fn range() {
    let mut index: SpatialHashMap<U8, U8, Shim32> = SpatialHashMap::new();

    for (i, &pt) in POINTS.iter().enumerate() {
        index.insert(pt, i as u32);
    }

    let mut result = Vec::new();
    index.range(RANGE_MIN, RANGE_MAX, |idx| {
        result.push(idx);
        let p = POINTS[idx as usize];
        assert!(test_range(p),
            "result point {:?} not in range {:?} {:?}",
            p, RANGE_MIN, RANGE_MAX);
    });

    let mut brute: Vec<_> = brute_range().collect();
    result.sort();
    brute.sort();
    assert_eq!(&result[..], &brute[..]);
}

#[test]
fn within() {
    let mut index: SpatialHashMap<U8, U8, Shim32> = SpatialHashMap::new();

    for (i, &pt) in POINTS.iter().enumerate() {
        index.insert(pt, i as u32);
    }

    let mut result = Vec::new();
    index.within(WITHIN_CENTER, WITHIN_RADIUS, |idx| {
        result.push(idx);
        let p = POINTS[idx as usize];
        assert!(test_within(p),
            "result point {:?} not in range {:?} {:?}",
            p, WITHIN_CENTER, WITHIN_RADIUS);
    });

    let mut brute: Vec<_> = brute_within().collect();
    result.sort();
    brute.sort();
    assert_eq!(&result[..], &brute[..]);
}
