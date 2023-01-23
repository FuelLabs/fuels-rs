use std::collections::HashSet;

use proc_macro2::{Ident, TokenStream};
use quote::quote;

use fuels_types::{errors::Error, utils::custom_type_name};

use crate::{
    code_gen::{
        abi_types::FullTypeDeclaration,
        custom_types::utils::{extract_components, extract_generic_parameters},
        generated_code::GeneratedCode,
        type_path::TypePath,
        utils::Component,
    },
    utils::ident,
};

/// Returns a TokenStream containing the declaration, `Parameterize`,
/// `Tokenizable` and `TryFrom` implementations for the struct described by the
/// given TypeDeclaration.
pub(crate) fn expand_custom_struct(
    type_decl: &FullTypeDeclaration,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<GeneratedCode, Error> {
    let struct_name = custom_type_name(&type_decl.type_field)?;
    let struct_ident = ident(&struct_name);

    let components = extract_components(type_decl, true, shared_types)?;
    let generic_parameters = extract_generic_parameters(type_decl)?;

    let code = struct_decl(&struct_ident, &components, &generic_parameters);

    let struct_type_path = TypePath::new(&struct_name).expect("Struct name is not empty!");

    Ok(GeneratedCode {
        code,
        usable_types: HashSet::from([struct_type_path]),
    })
}

fn struct_decl(
    struct_ident: &Ident,
    components: &[Component],
    generic_parameters: &Vec<TokenStream>,
) -> TokenStream {
    let fields = components.iter().map(
        |Component {
             field_name,
             field_type,
         }| {
            quote! { pub #field_name: #field_type }
        },
    );

    quote! {
        #[derive(
            Clone,
            Debug,
            Eq,
            PartialEq,
            ::fuels::fuels_abigen::Parameterize,
            ::fuels::fuels_abigen::Tokenizable,
            ::fuels::fuels_abigen::TryFrom
        )]
        pub struct #struct_ident <#(#generic_parameters: ::fuels::types::traits::Tokenizable + ::fuels::types::traits::Parameterize, )*> {
            #(#fields),*
        }
    }
}
