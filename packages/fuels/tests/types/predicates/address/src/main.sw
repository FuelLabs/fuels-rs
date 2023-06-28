predicate;

fn main(input: Address) -> bool {
    let expected_addr = Address::from(0xef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a);

    input == expected_addr
}
