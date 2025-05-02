contract;

#[error_type]
enum MyError {
  #[error(m="error A")]
  A: (),
  #[error(m="error B")]
  B: (u64),
}

abi PanicContract {
    fn some_panic();
    fn some_panic_error();
}

impl PanicContract for Contract {
    fn some_panic() {
        panic "some panic msg";
    }

    fn some_panic_error() {
        panic MyError::A;
    }
}
