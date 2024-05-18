script;

struct GenericBimbam<U> {
    val: U,
}

#[allow(dead_code)]
struct GenericSnack<T, V> {
    twix: GenericBimbam<T>,
    mars: V,
}

fn main(
    bim: GenericBimbam<u8>,
    bam: GenericSnack<u16, u32>,
) -> (GenericSnack<u64, u32>, GenericBimbam<u8>) {
    (
        GenericSnack {
            twix: GenericBimbam {
                val: bam.mars.as_u64(),
            },
            mars: 2u32 * bim.val.as_u32(),
        },
        GenericBimbam { val: 255u8 },
    )
}
