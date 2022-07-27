use crate::ByteArray;
use proc_macro2::{Ident, Span};
use sha2::{Digest, Sha256};
use syn::Ident as SynIdent;

/// Hashes an encoded function selector using SHA256 and returns the first 4 bytes.
/// The function selector has to have been already encoded following the ABI specs defined
/// [here](https://github.com/FuelLabs/fuel-specs/blob/1be31f70c757d8390f74b9e1b3beb096620553eb/specs/protocol/abi.md)
pub fn first_four_bytes_of_sha256_hash(string: &str) -> ByteArray {
    let string_as_bytes = string.as_bytes();
    let mut hasher = Sha256::new();
    hasher.update(string_as_bytes);
    let result = hasher.finalize();
    let mut output = ByteArray::default();
    (&mut output[4..]).copy_from_slice(&result[..4]);
    output
}

/// Expands a identifier string into an token.
pub fn ident(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}

// Expands an identifier string into a token and appending `_` if the
/// identifier is for a reserved keyword.
///
/// Parsing keywords like `self` can fail, in this case we add an underscore.
pub fn safe_ident(name: &str) -> Ident {
    syn::parse_str::<SynIdent>(name).unwrap_or_else(|_| ident(&format!("{}_", name)))
}
