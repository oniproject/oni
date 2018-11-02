use arrayvec::ArrayVec;
//use generic_array::{GenericArray, typenum::U8};
//use oni_reliable::SequenceOps;

const MESSAGE_COUNT: usize = 4;

pub struct Sender<S> {
    queue: ArrayVec<[(usize, S); MESSAGE_COUNT]>,
}

impl<S: Clone> Sender<S> {
    pub fn new() -> Self {
        Self {
            queue: ArrayVec::new(),
        }
    }

    pub fn send<T>(&mut self, sample: T) -> impl Iterator<Item=S> + '_
        where T: Into<Option<S>>
    {
        self.send_impl(sample.into())
    }

    fn send_impl(&mut self, sample: Option<S>) -> impl Iterator<Item=S> + '_ {
        for s in &mut self.queue {
            s.0 += 1;
        }
        let cap = self.queue.capacity();
        self.queue.retain(|s| s.0 < cap);
        if let Some(sample) = sample {
            self.queue.push((0, sample));
        }
        self.queue.iter().map(|s| s.1.clone())
    }
}

#[test]
fn sender() {
    let tests: &[(_, &[_])] = &[
        (None, &[]),
        (None, &[]),

        (Some(0), &[         0]),
        (None,    &[      0   ]),
        (None,    &[   0      ]),
        (None,    &[0         ]),
        (None,    &[          ]),

        (Some(1), &[         1]),
        (Some(2), &[      1, 2]),
        (None,    &[   1, 2   ]),
        (None,    &[1, 2      ]),
        (None,    &[2         ]),
        (None,    &[          ]),

        (Some(1), &[         1]),
        (Some(2), &[      1, 2]),
        (Some(3), &[   1, 2, 3]),
        (None,    &[1, 2, 3   ]),
        (None,    &[2, 3      ]),
        (None,    &[3         ]),
        (None,    &[          ]),

        (Some(1), &[         1]),
        (None,    &[      1   ]),
        (Some(3), &[   1,    3]),
        (None,    &[1,    3   ]),
        (None,    &[   3      ]),
        (None,    &[3         ]),
        (None,    &[          ]),

        (Some(1), &[         1]),
        (None,    &[      1   ]),
        (None,    &[   1      ]),
        (Some(4), &[1,       4]),
        (None,    &[      4   ]),
        (None,    &[   4      ]),
        (None,    &[4         ]),
        (None,    &[          ]),

        (Some(1), &[         1]),
        (Some(2), &[      1, 2]),
        (Some(3), &[   1, 2, 3]),
        (Some(4), &[1, 2, 3, 4]),
        (Some(5), &[2, 3, 4, 5]),
        (Some(6), &[3, 4, 5, 6]),
        (None,    &[4, 5, 6   ]),
        (None,    &[5, 6      ]),
        (None,    &[6         ]),
        (None,    &[          ]),
    ];

    let mut sender: Sender<usize> = Sender::new();
    for (i, test) in tests.iter().enumerate() {
        println!("{}:\t{:?}\t\tq:\t{:?}", i, test, &sender.queue);
        let b: Vec<_> = sender.send(test.0).collect();
        assert_eq!(&b, &test.1, "i: {}", i);
    }
}
