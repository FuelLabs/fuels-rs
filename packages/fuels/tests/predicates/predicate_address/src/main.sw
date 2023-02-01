predicate;

use std::inputs::GTF_INPUT_MESSAGE_PREDICATE_DATA;


fn main2(input: Address) -> bool {
    let expected_addr = Address::from(0xef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a);

    input == expected_addr
}

// alternate main fn to verify std lib is using the correct GTF arg value
fn main() -> bool {
    let data = __gtf::<raw_ptr>(0, GTF_INPUT_MESSAGE_PREDICATE_DATA);
    let data_2: u64 = data.read::<u64>();
    //let expected_addr = Address::from(0xef86afa9696cf0dc6385e2c407a6e159a1103cefb7e2ae0636fb33d3cb2a9e4a);

    //input == expected_addr
    data_2 == 0
}
