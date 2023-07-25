use std::collections::HashSet;

use fuel_abi_types::abi::full_program::FullTypeDeclaration;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    error::{error, Result},
    program_bindings::{
        custom_types::utils::{extract_components, extract_generic_parameters},
        generated_code::GeneratedCode,
        utils::Component,
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

    let components = extract_components(type_decl, false, &enum_type_path.parent())?;
    if components.is_empty() {
        return Err(error!("Enum must have at least one component!"));
    }
    let generics = extract_generic_parameters(type_decl)?;

    let code = enum_decl(enum_ident, &components, &generics, no_std);

    let enum_code = GeneratedCode::new(code, HashSet::from([enum_ident.into()]), no_std);

    Ok(enum_code.wrap_in_mod(enum_type_path.parent()))
}

fn enum_decl(
    enum_ident: &Ident,
    components: &[Component],
    generics: &[TokenStream],
    no_std: bool,
) -> TokenStream {
    let enum_variants = components.iter().map(
        |Component {
             field_name,
             field_type,
         }| {
            if field_type.is_unit() {
                quote! {#field_name}
            } else {
                quote! {#field_name(#field_type)}
            }
        },
    );
    let maybe_disable_std = no_std.then(|| quote! {#[NoStd]});

    quote! {
        #[allow(clippy::enum_variant_names)]
        #[derive(
            Clone,
            Debug,
            Eq,
            PartialEq,
            ::fuels::macros::Parameterize,
            ::fuels::macros::Tokenizable,
            ::fuels::macros::TryFrom
        )]
        #maybe_disable_std
        pub enum #enum_ident <#(#generics: ::fuels::core::traits::Tokenizable + ::fuels::core::traits::Parameterize),*> {
            #(#enum_variants),*
        }
    }
}
