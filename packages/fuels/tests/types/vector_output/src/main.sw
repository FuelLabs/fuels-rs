contract;

struct Bimbam {
    bim: u64,
    bam: u32,
}

abi VectorsOutputContract {
    fn u8_vec(len: u8) -> Vec<u8>;
    fn u16_vec(len: u16) -> Vec<u16>;
    fn u32_vec(len: u32) -> Vec<u32>;
    fn u64_vec(len: u64) -> Vec<u64>;
    fn struct_vec() -> Vec<Bimbam>;
}

impl VectorsOutputContract for Contract {
    fn u8_vec(len: u8) -> Vec<u8> {
        let mut vec: Vec<u8> = Vec::new();
        let mut i: u8 = 0;
        while i < len {
            vec.push(i);
            i += 1u8;
        }
        vec
    }

    fn u16_vec(len: u16) -> Vec<u16> {
        let mut vec: Vec<u16> = Vec::new();
        let mut i: u16 = 0;
        while i < len {
            vec.push(i);
            i += 1_u16;
        }
        vec
    }

    fn u32_vec(len: u32) -> Vec<u32> {
        let mut vec: Vec<u32> = Vec::new();
        let mut i: u32 = 0;
        while i < len {
            vec.push(i);
            i += 1_u32;
        }
        vec
    }

    fn u64_vec(len: u64) -> Vec<u64> {
        let mut vec: Vec<u64> = Vec::new();
        let mut i: u64 = 0;
        while i < len {
            vec.push(i);
            i += 1_u64;
        }
        vec
    }

    fn struct_vec() -> Vec<Bimbam> {
        let mut vec: Vec<Bimbam> = Vec::new();
        let a = Bimbam {
            bim: 1111,
            bam: 2222_u32,
        };
        vec.push(a);
        let b = Bimbam {
            bim: 3333,
            bam: 4444_u32,
        };
        vec.push(b);
        let c = Bimbam {
            bim: 5555,
            bam: 6666_u32,
        };
        vec.push(c);
        vec
    }
}
