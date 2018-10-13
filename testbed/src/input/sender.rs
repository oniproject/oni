use arrayvec::ArrayVec;
use oni::reliable::SequenceOps;
use crate::prot::InputSample;

pub struct InputSender {
    history: ArrayVec<[InputSample; 8]>,
}

impl InputSender {
    pub fn new() -> Self {
        Self {
            history: ArrayVec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.history.clear();
    }

    pub fn send<F>(&mut self, input: Option<InputSample>, f: F)
        where F: FnOnce(ArrayVec<[InputSample; 8]>)
    {
        // FIXME: inaccurate

        if let Some(input) = input {
            let drop_sequence = input.sequence.prev_n(5);
            self.history.retain(|input| input.sequence >= drop_sequence);
            while self.history.len() > 5 {
                self.history.remove(0);
            }
            self.history.push(input);
        } else if self.history.len() != 0 {
            self.history.remove(0);
        }
        if self.history.len() != 0 {
            f(self.history.clone());
        }
    }
}
