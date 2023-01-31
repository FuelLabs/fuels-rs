use std::collections::HashSet;

use fuel_abi_types::utils::extract_custom_type_name;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    abigen::code_gen::{
        abi_types::FullTypeDeclaration,
        custom_types::utils::{extract_components, extract_generic_parameters},
        generated_code::GeneratedCode,
        type_path::TypePath,
        utils::Component,
    },
    error::{error, Result},
    utils::ident,
};

/// Returns a TokenStream containing the declaration, `Parameterize`,
/// `Tokenizable` and `TryFrom` implementations for the enum described by the
/// given TypeDeclaration.
pub(crate) fn expand_custom_enum(
    type_decl: &FullTypeDeclaration,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<GeneratedCode> {
    let type_field = &type_decl.type_field;
    let enum_name = extract_custom_type_name(type_field).ok_or_else(|| {
        error!(
            "Could not extract enum name from type_field: {}",
            type_field
        )
    })?;
    let enum_ident = ident(&enum_name);

    let components = extract_components(type_decl, false, shared_types)?;
    if components.is_empty() {
        return Err(error!("Enum must have at least one component!"));
    }
    let generics = extract_generic_parameters(type_decl)?;

    let code = enum_decl(&enum_ident, &components, &generics);

    let enum_type_path = TypePath::new(&enum_name).expect("Enum name is not empty!");

    Ok(GeneratedCode {
        code,
        usable_types: HashSet::from([enum_type_path]),
    })
}

fn enum_decl(
    enum_ident: &Ident,
    components: &[Component],
    generics: &[TokenStream],
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
        pub enum #enum_ident <#(#generics: ::fuels::types::traits::Tokenizable + ::fuels::types::traits::Parameterize),*> {
            #(#enum_variants),*
        }
    }
}
