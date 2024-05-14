predicate;

use std::bytes::Bytes;
use std::hash::{Hash, sha256};

fn main(bytes: Bytes, hash: b256) -> bool {
    sha256(bytes) == hash
}
