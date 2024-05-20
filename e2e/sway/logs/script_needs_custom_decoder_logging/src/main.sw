script;

impl AbiEncode for [u8; 1000] {
    #[allow(dead_code)]
    fn abi_encode(self, buffer: Buffer) -> Buffer {
        let mut buffer = buffer;
        let mut i = 0;
        while i < 1000 {
            buffer = self[i].abi_encode(buffer);
            i += 1;
        };

        buffer
    }
}

fn main() {
    // TODO: This file can be made obsolete once
    // [retry](`https://github.com/FuelLabs/fuels-rs/issues/1020`) lands.
    let arr: [u8; 1000] = [0; 1000];
    log(arr);
}
