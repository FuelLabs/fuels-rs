use proc_macro2::TokenStream;
use quote::quote;

pub fn std_lib_path(no_std: bool) -> TokenStream {
    if no_std {
        quote! {::alloc}
    } else {
        quote! {::std}
    }
}

pub fn fuels_macros_path(no_std: bool) -> TokenStream {
    if no_std {
        quote! {::fuels_macros}
    } else {
        quote! {::fuels::macros}
    }
}

pub fn fuels_core_path(no_std: bool) -> TokenStream {
    if no_std {
        quote! {::fuels_core}
    } else {
        quote! {::fuels::core}
    }
}

pub fn fuels_types_path(no_std: bool) -> TokenStream {
    if no_std {
        quote! {::fuels_types}
    } else {
        quote! {::fuels::types}
    }
}

pub fn fuels_signers_path(no_std: bool) -> TokenStream {
    if no_std {
        quote! {::fuels_signers}
    } else {
        quote! {::fuels::signers}
    }
}
pub fn fuels_programs_path(no_std: bool) -> TokenStream {
    if no_std {
        quote! {::fuels_programs}
    } else {
        quote! {::fuels::programs}
    }
}

pub fn fuels_tx_path(no_std: bool) -> TokenStream {
    if no_std {
        quote! {::fuel_tx}
    } else {
        quote! {::fuels::tx}
    }
}
