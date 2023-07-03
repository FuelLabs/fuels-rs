script;

struct Foo {
    bar: u64,
    baz: bool,
}

fn main(foo: Foo) -> u64 {
    if foo.baz { foo.bar } else { 0 }
}
