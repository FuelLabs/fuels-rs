script;

struct GenericBimbam<U> {
    val: U,
}

struct GenericSnack<T, V> {
    twix: GenericBimbam<T>,
    mars: V,
}

fn main(
    bim: GenericBimbam<u8>,
    bam: GenericSnack<u16, u32>,
) -> (GenericSnack<u64, u32>, GenericBimbam<u8>) {
    let bot = GenericBimbam { val: bam.mars };
    (
        GenericSnack {
            twix: bot,
            mars: 2 * bim.val,
        },
        GenericBimbam { val: 255 },
    )
}
