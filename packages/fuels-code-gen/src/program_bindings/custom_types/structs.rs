use std::collections::HashSet;

use fuel_abi_types::utils::extract_custom_type_name;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    error::{error, Result},
    program_bindings::{
        abi_types::FullTypeDeclaration,
        custom_types::utils::{extract_components, extract_generic_parameters},
        generated_code::GeneratedCode,
        utils::Component,
    },
    utils::{
        ident,
        type_path_lookup::{fuels_core_path, fuels_macros_path, fuels_types_path},
        TypePath,
    },
};

/// Returns a TokenStream containing the declaration, `Parameterize`,
/// `Tokenizable` and `TryFrom` implementations for the struct described by the
/// given TypeDeclaration.
pub(crate) fn expand_custom_struct(
    type_decl: &FullTypeDeclaration,
    shared_types: &HashSet<FullTypeDeclaration>,
    no_std: bool,
) -> Result<GeneratedCode> {
    let type_field = &type_decl.type_field;
    let struct_name = extract_custom_type_name(&type_decl.type_field)
        .ok_or_else(|| error!("Couldn't parse struct name from type field {type_field}"))?;
    let struct_ident = ident(&struct_name);

    let components = extract_components(type_decl, true, shared_types, no_std)?;
    let generic_parameters = extract_generic_parameters(type_decl)?;

    let code = struct_decl(&struct_ident, &components, &generic_parameters, no_std);

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
    no_std: bool,
) -> TokenStream {
    let fields = components
        .iter()
        .map(|component| component.as_struct_member());

    let fuels_types = fuels_types_path(no_std);
    let fuels_macros = fuels_macros_path(no_std);
    let fuels_core = fuels_core_path(no_std);

    let path_redirects = no_std.then(|| {
        let fuels_types = fuels_types.to_string();
        let fuels_core = fuels_core.to_string();
        quote! {
            #[FuelsTypesPath(#fuels_types)]
            #[FuelsCorePath(#fuels_core)]
        }
    });

    let std_disable_switch = no_std.then(|| quote! {#[NoStd]});

    quote! {
        #[derive(
            Clone,
            Debug,
            Eq,
            PartialEq,
            #fuels_macros::Parameterize,
            #fuels_macros::Tokenizable,
            #fuels_macros::TryFrom
        )]
        #path_redirects
        #std_disable_switch
        pub struct #struct_ident <#(#generic_parameters: #fuels_types::traits::Tokenizable + #fuels_types::traits::Parameterize, )*> {
            #(#fields),*
        }
    }
}
