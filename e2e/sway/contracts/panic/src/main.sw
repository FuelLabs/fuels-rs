contract;

abi PanicContract {
    fn some_panic();
}

impl PanicContract for Contract {
    fn some_panic() {
        panic "some panic msg"
    }
}
