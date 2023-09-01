script;

struct Bimbam {
    val: u64,
}

struct SugarySnack {
    twix: u64,
    mars: u64,
}

fn main(bim: Bimbam, bam: SugarySnack) -> Bimbam {
    let val = bam.twix + bim.val + (bam.mars * 2);
    Bimbam { val: val }
}
