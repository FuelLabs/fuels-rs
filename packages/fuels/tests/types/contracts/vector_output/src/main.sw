contract;

struct Bimbam {
    bim: u64,
    bam: u32,
}

enum Pasta {
    Rigatoni: u64,
    Spaghetti: bool,
    Tortelini: Bimbam,
}

struct ZimZam {
    vec_component: Vec<u64>,
}

abi VectorsOutputContract {
    fn vec_inside_type() -> ZimZam;
    fn array_in_vec() -> Vec<[u64; 4]>;
    fn bool_in_vec() -> Vec<bool>;
    fn enum_in_vec() -> Vec<Pasta>;
    fn str_in_vec() -> Vec<str[4]>;
    fn struct_in_vec() -> Vec<Bimbam>;
    fn tuple_in_vec() -> Vec<(u64, u32)>;
    fn u16_in_vec(len: u16) -> Vec<u16>;
    fn u32_in_vec(len: u32) -> Vec<u32>;
    fn u64_in_vec(len: u64) -> Vec<u64>;
    fn u8_in_vec(len: u8) -> Vec<u8>;
}

impl VectorsOutputContract for Contract {
    fn vec_inside_type() -> ZimZam {
        let mut b = Vec::new();
        b.push(255);
        b.push(255);
        b.push(255);
        b.push(255);
        ZimZam {
            vec_component: b,
        }
    }

    fn array_in_vec() -> Vec<[u64; 4]> {
        let mut vec: Vec<[u64; 4]> = Vec::new();
        vec.push([1, 1, 1, 1]);
        vec.push([2, 2, 2, 2]);
        vec.push([3, 3, 3, 3]);
        vec.push([4, 4, 4, 4]);
        vec
    }

    fn bool_in_vec() -> Vec<bool> {
        let mut vec: Vec<bool> = Vec::new();
        vec.push(true);
        vec.push(false);
        vec.push(true);
        vec.push(false);
        vec
    }

    fn enum_in_vec() -> Vec<Pasta> {
        let mut vec: Vec<Pasta> = Vec::new();
        vec.push(Pasta::Tortelini(Bimbam {
            bim: 1111,
            bam: 2222_u32,
        }));
        vec.push(Pasta::Rigatoni(1987));
        vec.push(Pasta::Spaghetti(true));
        vec
    }

    fn str_in_vec() -> Vec<str[4]> {
        let mut vec: Vec<str[4]> = Vec::new();
        vec.push(__to_str_array("hell"));
        vec.push(__to_str_array("ello"));
        vec.push(__to_str_array("lloh"));
        vec
    }

    fn struct_in_vec() -> Vec<Bimbam> {
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

    fn tuple_in_vec() -> Vec<(u64, u32)> {
        let mut vec: Vec<(u64, u32)> = Vec::new();
        vec.push((1111, 2222_u32));
        vec.push((3333, 4444_u32));
        vec.push((5555, 6666_u32));
        vec
    }

    fn u8_in_vec(len: u8) -> Vec<u8> {
        let mut vec: Vec<u8> = Vec::new();
        let mut i: u8 = 0;
        while i < len {
            vec.push(i);
            i += 1u8;
        }
        vec
    }

    fn u16_in_vec(len: u16) -> Vec<u16> {
        let mut vec: Vec<u16> = Vec::new();
        let mut i: u16 = 0;
        while i < len {
            vec.push(i);
            i += 1_u16;
        }
        vec
    }

    fn u32_in_vec(len: u32) -> Vec<u32> {
        let mut vec: Vec<u32> = Vec::new();
        let mut i: u32 = 0;
        while i < len {
            vec.push(i);
            i += 1_u32;
        }
        vec
    }

    fn u64_in_vec(len: u64) -> Vec<u64> {
        let mut vec: Vec<u64> = Vec::new();
        let mut i: u64 = 0;
        while i < len {
            vec.push(i);
            i += 1_u64;
        }
        vec
    }
}
