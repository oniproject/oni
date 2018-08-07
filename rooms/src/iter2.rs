use num_traits::{Num, NumAssignOps};

#[derive(Copy, Clone)]
pub struct Iter2<N> {
    y: N,
    x: N,

    start_x: N,
    end_x: N,
    end_y: N,
}

impl<N: Copy> Iter2<N> {
    pub fn new(x: (N, N), y: (N, N)) -> Self {
        Self {
            start_x: x.0,
            x: x.0,
            y: y.0,
            end_x: x.1,
            end_y: y.1,
        }
    }
}

impl<N: Num + NumAssignOps + Ord + Copy> Iterator for Iter2<N> {
    type Item = (N, N);
    fn next(&mut self) -> Option<Self::Item> {
        if self.y > self.end_y {
            None
        } else {
            let key = (self.x, self.y);
            if self.x == self.end_x {
                self.x = self.start_x;
                self.y += N::one();
            } else {
                self.x += N::one();
            }
            Some(key)
        }
    }
}

#[test]
fn iter2() {
    let v: Vec<_> = Iter2::new((0, 2), (0, 1)).collect();

    assert_eq!(v, &[
        (0, 0),
        (1, 0),
        (2, 0),

        (0, 1),
        (1, 1),
        (2, 1),
    ]);
}
