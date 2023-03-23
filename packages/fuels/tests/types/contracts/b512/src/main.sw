contract;

use std::b512::B512;

const HI_BITS = 0xbd0c9b8792876713afa8bff383eebf31c43437823ed761cc3600d0016de5110c;
const LO_BITS = 0x44ac566bd156b4fc71a4a4cb2655d3dd360c695edb17dc3b64d611e122fea23d;
const LO_BITS2 = 0x54ac566bd156b4fc71a4a4cb2655d3dd360c695edb17dc3b64d611e122fea23d;

abi MyContract {
    fn b512_as_output() -> B512;
    fn b512_as_input(b512: B512) -> bool;
}

impl MyContract for Contract {
    fn b512_as_output() -> B512 {
        B512::from((HI_BITS, LO_BITS))
    }

    fn b512_as_input(b512: B512) -> bool {
        let expected_b512 = B512::from((HI_BITS, LO_BITS2));

        b512 == expected_b512
    }
}
