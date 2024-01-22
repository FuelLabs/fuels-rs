contract;

use lib_contract::LibContract;

impl AbiEncode for str[21] {
    fn abi_encode(self, ref mut buffer: Buffer) {
        let s = from_str_array(self);

        let len = s.len();
        let ptr = s.as_ptr();

        let mut i = 0;
        while i < len {
            let byte = ptr.add::<u8>(i).read::<u8>();
            buffer.push(byte);
            i += 1;
        }
    }
}

impl LibContract for Contract {
    fn increment(value: u64) -> u64 {
        value + 1
    }

    fn require() -> () {
        require(false, __to_str_array("require from contract"));
    }
}
