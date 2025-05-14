use std::collections::HashSet;

use fuel_abi_types::abi::full_program::FullTypeDeclaration;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    error::Result,
    program_bindings::{
        custom_types::utils::extract_generic_parameters,
        generated_code::GeneratedCode,
        resolved_type::ResolvedType,
        utils::{Component, Components, tokenize_generics},
    },
};

/// Returns a TokenStream containing the declaration, `Parameterize`,
/// `Tokenizable` and `TryFrom` implementations for the struct described by the
/// given TypeDeclaration.
pub(crate) fn expand_custom_struct(
    type_decl: &FullTypeDeclaration,
    no_std: bool,
) -> Result<GeneratedCode> {
    let struct_type_path = type_decl.custom_type_path()?;
    let struct_ident = struct_type_path.ident().unwrap();

    let components = Components::new(&type_decl.components, true, struct_type_path.parent())?;
    let generic_parameters = extract_generic_parameters(type_decl);

    let code = struct_decl(struct_ident, &components, &generic_parameters, no_std);

    let struct_code = GeneratedCode::new(code, HashSet::from([struct_ident.into()]), no_std);

    Ok(struct_code.wrap_in_mod(struct_type_path.parent()))
}

fn unzip_field_names_and_types(components: &Components) -> (Vec<&Ident>, Vec<&ResolvedType>) {
    components
        .iter()
        .map(
            |Component {
                 ident,
                 resolved_type,
                 ..
             }| (ident, resolved_type),
        )
        .unzip()
}

fn struct_decl(
    struct_ident: &Ident,
    components: &Components,
    generics: &[Ident],
    no_std: bool,
) -> TokenStream {
    let derive_default = components
        .is_empty()
        .then(|| quote!(::core::default::Default,));

    let maybe_disable_std = no_std.then(|| quote! {#[NoStd]});

    let (generics_wo_bounds, generics_w_bounds) = tokenize_generics(generics);
    let (field_names, field_types): (Vec<_>, Vec<_>) = unzip_field_names_and_types(components);
    let (phantom_fields, phantom_types) =
        components.generate_parameters_for_unused_generics(generics);

    quote! {
        #[derive(
            Clone,
            Debug,
            Eq,
            PartialEq,
            #derive_default
            ::fuels::macros::Parameterize,
            ::fuels::macros::Tokenizable,
            ::fuels::macros::TryFrom,
        )]
        #maybe_disable_std
        pub struct #struct_ident #generics_w_bounds {
            #( pub #field_names: #field_types, )*
            #(#[Ignore] pub #phantom_fields: #phantom_types, )*
        }

        impl #generics_w_bounds #struct_ident #generics_wo_bounds {
            pub fn new(#(#field_names: #field_types,)*) -> Self {
                Self {
                    #(#field_names,)*
                    #(#phantom_fields: ::core::default::Default::default(),)*
                }
            }
        }
    }
}
