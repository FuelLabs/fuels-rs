contract;

use foo::FooContract;

impl FooContract for Contract {
    fn foo(value: bool) -> bool {
       !value
    }
}
