contract;

#[allow(dead_code)]
enum Shaker {
    Cosmopolitan: u64,
    Mojito: u64,
}

struct Cocktail {
    the_thing_you_mix_in: Shaker,
    glass: u64,
}

abi TestContract {
    fn return_enum_inside_struct(a: u64) -> Cocktail;
    fn take_enum_inside_struct(c: Cocktail) -> u64;
}

impl TestContract for Contract {
    fn return_enum_inside_struct(a: u64) -> Cocktail {
        Cocktail {
            the_thing_you_mix_in: Shaker::Mojito(a),
            glass: 333,
        }
    }

    fn take_enum_inside_struct(c: Cocktail) -> u64 {
        c.glass
    }
}
