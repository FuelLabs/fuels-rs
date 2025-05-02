use core::fmt;
use std::collections::HashSet;

use fuel_abi_types::abi::full_program::FullTypeDeclaration;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    error::{Result, error},
    program_bindings::{
        custom_types::utils::extract_generic_parameters,
        generated_code::GeneratedCode,
        resolved_type::ResolvedType,
        utils::{Components, tokenize_generics},
    },
};

/// Returns a TokenStream containing the declaration, `Parameterize`,
/// `Tokenizable` and `TryFrom` implementations for the enum described by the
/// given TypeDeclaration.
pub(crate) fn expand_custom_enum(
    type_decl: &FullTypeDeclaration,
    no_std: bool,
) -> Result<GeneratedCode> {
    let enum_type_path = type_decl.custom_type_path()?;
    let enum_ident = enum_type_path.ident().unwrap();

    let components = Components::new(&type_decl.components, false, enum_type_path.parent())?;
    if components.is_empty() {
        return Err(error!("enum must have at least one component"));
    }
    let generics = extract_generic_parameters(type_decl);

    let code = enum_decl(enum_ident, &components, &generics, no_std);

    let enum_code = GeneratedCode::new(code, HashSet::from([enum_ident.into()]), no_std);

    Ok(enum_code.wrap_in_mod(enum_type_path.parent()))
}

fn enum_decl(
    enum_ident: &Ident,
    components: &Components,
    generics: &[Ident],
    no_std: bool,
) -> TokenStream {
    let maybe_disable_std = no_std.then(|| quote! {#[NoStd]});

    let enum_variants = components.as_enum_variants();
    let unused_generics_variant = components.generate_variant_for_unused_generics(generics);
    let (_, generics_w_bounds) = tokenize_generics(generics);

    let has_error_messages = components.has_error_message();

    let derive = if has_error_messages {
        quote! {#[derive(
            Clone,
            Eq,
            PartialEq,
            ::fuels::macros::Parameterize,
            ::fuels::macros::Tokenizable,
            ::fuels::macros::TryFrom,
        )]}
    } else {
        quote! {#[derive(
            Clone,
            Debug,
            Eq,
            PartialEq,
            ::fuels::macros::Parameterize,
            ::fuels::macros::Tokenizable,
            ::fuels::macros::TryFrom,
        )]}
    };

    let custom_dbg_impl = if has_error_messages {
        // custom debug impl
        let match_branches = components.iter().map(|(ident, ty, error_message)| {
            let error_msg = error_message.clone().expect("is there"); //TODO: fix clone
            if let ResolvedType::Unit = ty {
                quote! {#enum_ident::#ident =>  ::std::write!(f, "{}", #error_msg)}
            } else {
                quote! {#enum_ident::#ident(_) => ::std::write!(f, "{}", #error_msg)}
            }
        });

        let custom_dbg_impl = quote! {
            impl ::std::fmt::Debug for #enum_ident {
                fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                   match &self {
                    #(#match_branches,)*
                   }
                }
            }
        };

        custom_dbg_impl
    } else {
        quote! {}
    };

    quote! {
        #[allow(clippy::enum_variant_names)]
        #derive
        #maybe_disable_std
        pub enum #enum_ident #generics_w_bounds {
            #(#enum_variants,)*
            #unused_generics_variant
        }
        #custom_dbg_impl
    }
}
