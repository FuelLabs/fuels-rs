script;

#[allow(dead_code)]
struct Bim {
    bim: u64,
}

#[allow(dead_code)]
struct Bam {
    bam: str[5],
}

#[allow(dead_code)]
struct Boum {
    boum: bool,
}

fn main(_my_tuple: (Bim, Bam, Boum), _zim: Bam) -> ((Boum, Bim, Bam), u64) {
    (
        (
            Boum { boum: true },
            Bim { bim: 193817 },
            Bam { bam: "hello" },
        ),
        42242,
    )
}
