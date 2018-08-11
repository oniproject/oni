/*
mod data;

fn sq_dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    dx * dx + dy * dy
}

static IDS: &[u32] = &[
    97,74,95,30,77,38,76,27,80,55,72,90,88,48,43,46,
    65,39,62,93, 9,96,47, 8, 3,12,15,14,21,41,36,40,
    69,56,85,78,17,71,44,19,18,13,99,24,67,33,37,49,
    54,57,98,45,23,31,66,68, 0,32, 5,51,75,73,84,35,
    81,22,61,89, 1,11,86,52,94,16, 2, 6,25,92,42,20,
    60,58,83,79,64,10,59,53,26,87, 4,63,50, 7,28,82,
    70,29,34,91,
];

static COORDS: &[(f32, f32)] = &[
    (10.0,20.0),( 6.0,22.0),(10.0,10.0),( 6.0,27.0),(20.0,42.0),(18.0,28.0),
    (11.0,23.0),(13.0,25.0),( 9.0,40.0),(26.0, 4.0),(29.0,50.0),(30.0,38.0),
    (41.0,11.0),(43.0,12.0),(43.0, 3.0),(46.0,12.0),(32.0,14.0),(35.0,15.0),
    (40.0,31.0),(33.0,18.0),(43.0,15.0),(40.0,34.0),(32.0,38.0),(33.0,34.0),
    (33.0,54.0),( 1.0,61.0),(24.0,56.0),(11.0,91.0),( 4.0,98.0),(20.0,81.0),
    (22.0,93.0),(19.0,81.0),(21.0,67.0),( 6.0,76.0),(21.0,72.0),(21.0,73.0),
    (25.0,57.0),(44.0,64.0),(47.0,66.0),(29.0,69.0),(46.0,61.0),(38.0,74.0),
    (46.0,78.0),(38.0,84.0),(32.0,88.0),(27.0,91.0),(45.0,94.0),(39.0,94.0),
    (41.0,92.0),(47.0,21.0),(47.0,29.0),(48.0,34.0),(60.0,25.0),(58.0,22.0),
    (55.0, 6.0),(62.0,32.0),(54.0, 1.0),(53.0,28.0),(54.0, 3.0),(66.0,14.0),
    (68.0, 3.0),(70.0, 5.0),(83.0, 6.0),(93.0,14.0),(99.0, 2.0),(71.0,15.0),
    (96.0,18.0),(95.0,20.0),(97.0,21.0),(81.0,23.0),(78.0,30.0),(84.0,30.0),
    (87.0,28.0),(90.0,31.0),(65.0,35.0),(53.0,54.0),(52.0,38.0),(65.0,48.0),
    (67.0,53.0),(49.0,60.0),(50.0,68.0),(57.0,70.0),(56.0,77.0),(63.0,86.0),
    (71.0,90.0),(52.0,83.0),(71.0,82.0),(72.0,81.0),(94.0,51.0),(75.0,53.0),
    (95.0,39.0),(78.0,53.0),(88.0,62.0),(84.0,72.0),(77.0,73.0),(99.0,76.0),
    (73.0,81.0),(88.0,87.0),(96.0,98.0),(96.0,82.0),
];

/*
#[test]
fn create_index() {
    let index = kdbush(points, 10);
    assert!(index.ids, ids, "ids are kd-sorted");
    assert!(index.coords, coords, "coords are kd-sorted");
}

#[test]
fn range_search() {
    let index = kdbush(points, 10);
    let result = index.range(20, 30, 50, 70);

    assert_eq!(result, &RANGE, "returns ids");

    for idx in &result {
        let p = points[idx];
        let is = p[0] < 20 || p[0] > 50 || p[1] < 30 || p[1] > 70;
        assert!(!is, "result point in range");
    }

    for idx in &IDS {
        let p = points[idx];
        let is = result.indexOf(idx) < 0 && p[0] >= 20 && p[0] <= 50 && p[1] >= 30 && p[1] <= 70;
        assert!(!is, "outside point not in range");
    }
}

#[test]
fn within_search() {
    let index = KDBush::new(points, 10);

    let qp = [50, 50];
    let r = 20;
    let r2 = 20 * 20;

    let result = index.within(qp[0], qp[1], r);

    assert_eq!(result, &WITHIN, "returns ids");

    for idx in &result {
        let p = points[idx];
        let is = sq_dist(p, qp) > r2;
        assert!(!is, "result point in range");
    }

    for idx in &IDS {
        let p = points[idx];
        let is = result.index_of(idx) < 0 && sq_dist(p, qp) <= r2;
        assert!(!is, "outside point not in range");
    }
}
*/
*/



mod data;
use crate::data::*;

use rooms::index::{KDBush, SpatialIndex};

#[test]
fn range() {
    let mut index: KDBush<f32> = KDBush::new(10);
    index.fill(POINTS.iter().cloned().enumerate()
        .map(|(i, p)| (i as u32, p)));

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
    let mut index: KDBush<f32> = KDBush::new(10);
    index.fill(POINTS.iter().cloned().enumerate()
        .map(|(i, p)| (i as u32, p)));

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
