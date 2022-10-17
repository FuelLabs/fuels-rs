script;

struct Bimbam {
    val: u64,
}

struct SugarySnack {
    twix: u64,
    mars: u64,
}

fn main(bim: Bimbam, bam: SugarySnack) -> u64 {
    bam.twix + bim.val + (bam.mars * 2)
}
