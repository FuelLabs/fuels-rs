#[macro_use]
mod wrappers;

mod definition_site {
    definition_site_abi!();
}

mod forwarded {
    forwarded_abi!("nested/abi.json");
}

fn main() {}
