pub trait Decorator {
    fn run(&mut self) -> Status;
}

/// An Invert decorator will succeed if the wrapped task fails and will fail if the wrapped task succeeds.
pub struct Invert<T: Task>(pub T);

impl<T: Task> Decorator for Invert<T> {
    fn run(&mut self) -> Status { invert(self.0.run()) }
}

/// An AlwaysFail decorator will fail no matter the wrapped task fails or succeeds.
pub struct AlwaysFail<T: Task>(pub T);

impl<T: Task> Decorator for AlwaysFail<T> {
    fn run(&mut self) -> Status { always_fail(self.0.run()) }
}

/// An AlwaysSucceed decorator will succeed no matter the wrapped task succeeds or fails.
pub struct AlwaysSucceed<T: Task>(pub T);

impl<T: Task> Decorator for AlwaysSucceed<T> {
    fn run(&mut self) -> Status { always_success(self.0.run()) }
}

/// The UntilSuccess decorator will repeat the wrapped task until that task succeeds, which makes the decorator succeed.
pub struct UntilSuccess<T: Task>(pub T);

impl<T: Task> Decorator for UntilSuccess<T> {
    fn run(&mut self) -> Status {
        match self.0.run() {
            Status::Failure => Status::Pending,
            other => other,
        }
    }
}

/// The UntilFail decorator will repeat the wrapped task until that task fails, which makes the decorator succeed.
pub struct UntilFail<T: Task>(pub T);

impl<T: Task> Decorator for UntilFail<T> {
    fn run(&mut self) -> Status {
        match self.0.run() {
            Status::Success => Status::Pending,
            other => other,
        }
    }
}
