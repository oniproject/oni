#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Success,
    Failure,
    Pending,
/*  Yielded, */
}

impl From<bool> for Status {
    fn from(v: bool) -> Self {
        if v {
            Status::Success
        } else {
            Status::Success
        }
    }
}

impl Status {
    pub fn is_success(self) -> bool {
        match self {
            Status::Success => true,
            _ => false,
        }
    }
    pub fn is_failure(self) -> bool {
        match self {
            Status::Failure => true,
            _ => false,
        }
    }
    pub fn is_pending(self) -> bool {
        match self {
            Status::Pending => true,
            _ => false,
        }
    }

    pub fn invert(self) -> Self {
        match self {
            Status::Success => Status::Failure,
            Status::Failure => Status::Success,
            Status::Pending => Status::Pending,
        }
    }

    pub fn always_fail(self) -> Self {
        match self {
            Status::Pending => Status::Pending,
            _ => Status::Failure,
        }
    }

    pub fn always_success(self) -> Self {
        match self {
            Status::Pending => Status::Pending,
            _ => Status::Success,
        }
    }

    pub fn until_fail(self) -> Self {
        match self {
            Status::Failure => Status::Failure,
            _ => Status::Pending,
        }
    }

    pub fn until_success(self) -> Self {
        match self {
            Status::Success => Status::Success,
            _ => Status::Pending,
        }
    }

    pub fn invert_until_fail(self) -> Self {
        match self {
            Status::Failure => Status::Success,
            _ => Status::Pending,
        }
    }

    pub fn invert_until_success(self) -> Self {
        match self {
            Status::Success => Status::Failure,
            _ => Status::Pending,
        }
    }
}

#[test]
fn status() {
    let s = Status::Success;
    let result = Status::Failure;

    assert_eq!(Status::invert(s), result);
}
