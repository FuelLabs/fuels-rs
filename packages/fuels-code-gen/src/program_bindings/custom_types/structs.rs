use std::collections::HashSet;

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    error::Result,
    program_bindings::{
        custom_types::utils::{extract_components, extract_generic_parameters},
        generated_code::GeneratedCode,
        utils::Component,
    },
};

use fuel_abi_types::abi::full_program::FullTypeDeclaration;

/// Returns a TokenStream containing the declaration, `Parameterize`,
/// `Tokenizable` and `TryFrom` implementations for the struct described by the
/// given TypeDeclaration.
pub(crate) fn expand_custom_struct(
    type_decl: &FullTypeDeclaration,
    no_std: bool,
) -> Result<GeneratedCode> {
    let struct_type_path = type_decl.custom_type_path()?;
    let struct_ident = struct_type_path.ident().unwrap();

    let components = extract_components(type_decl, true, &struct_type_path.parent())?;
    let generic_parameters = extract_generic_parameters(type_decl)?;

    let code = struct_decl(struct_ident, &components, &generic_parameters, no_std);

    let struct_code = GeneratedCode::new(code, HashSet::from([struct_ident.into()]), no_std);

    Ok(struct_code.wrap_in_mod(struct_type_path.parent()))
}

fn struct_decl(
    struct_ident: &Ident,
    components: &[Component],
    generic_parameters: &Vec<TokenStream>,
    no_std: bool,
) -> TokenStream {
    let fields = components.iter().map(
        |Component {
             field_name,
             field_type,
         }| {
            quote! { pub #field_name: #field_type }
        },
    );
    let maybe_disable_std = no_std.then(|| quote! {#[NoStd]});

    quote! {
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
        pub struct #struct_ident <#(#generic_parameters: ::fuels::core::traits::Tokenizable + ::fuels::core::traits::Parameterize, )*> {
            #(#fields),*
        }
    }
}
