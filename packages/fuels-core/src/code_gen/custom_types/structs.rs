use std::collections::HashSet;

use fuels_types::{errors::Error, utils::custom_type_name};
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    code_gen::{
        abi_types::FullTypeDeclaration,
        custom_types::utils::{extract_components, extract_generic_parameters, impl_try_from},
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

    let struct_decl = struct_decl(&struct_ident, &components, &generic_parameters);
    let try_from_impl = impl_try_from(&struct_ident, &generic_parameters);

    let code = quote! {
        #struct_decl

        #try_from_impl
    };
    Ok(GeneratedCode {
        code,
        usable_types: HashSet::from([
            TypePath::new(&struct_name).expect("Struct name is not empty!")
        ]),
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
            let field_type: TokenStream = field_type.into();
            quote! { pub #field_name: #field_type }
        },
    );

    quote! {
        #[derive(Clone, Debug, Eq, PartialEq, ::fuels::fuels_abigen::Parameterize, ::fuels::fuels_abigen::Tokenizable)]
        pub struct #struct_ident <#(#generic_parameters: ::fuels::types::traits::Tokenizable + ::fuels::types::traits::Parameterize, )*> {
            #(#fields),*
        }
    }
}
