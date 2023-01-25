use proc_macro2::{Ident, Span};

/// Expands a identifier string into an token.
pub(crate) fn ident(name: &str) -> Ident {
    Ident::new(name, Span::call_site())
}

pub(crate) fn safe_ident(name: &str) -> Ident {
    syn::parse_str::<Ident>(name).unwrap_or_else(|_| ident(&format!("{}_", name)))
}
