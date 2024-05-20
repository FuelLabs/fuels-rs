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

fn main() -> [u8; 1000] {
    let arr: [u8; 1000] = [0; 1000];
    arr
}
