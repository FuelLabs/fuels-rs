script;

struct Bim {
    bim: u64,
}

struct Bam {
    bam: str[5],
}

struct Boum {
    boum: bool,
}

fn main(my_tuple: (Bim, Bam, Boum), zim: Bam) -> ((Boum, Bim, Bam), u64) {
    (
        (
            Boum { boum: true },
            Bim { bim: 193817 },
            Bam { bam: "hello" },
        ),
        42242,
    )
}
