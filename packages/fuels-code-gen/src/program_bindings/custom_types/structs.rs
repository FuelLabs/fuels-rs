use std::collections::HashSet;

use fuel_abi_types::{abi::full_program::FullTypeDeclaration, utils::ident};
use proc_macro2::{Ident, TokenStream};
use quote::quote;

use crate::{
    error::Result,
    program_bindings::{
        custom_types::utils::{extract_components, extract_generic_parameters},
        generated_code::GeneratedCode,
        resolved_type::GenericType,
        utils::Component,
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

    let components = extract_components(type_decl, true, &struct_type_path.parent())?;
    let generic_parameters = extract_generic_parameters(type_decl);

    let code = struct_decl(struct_ident, &components, &generic_parameters, no_std);

    let struct_code = GeneratedCode::new(code, HashSet::from([struct_ident.into()]), no_std);

    Ok(struct_code.wrap_in_mod(struct_type_path.parent()))
}

fn struct_decl(
    struct_ident: &Ident,
    components: &[Component],
    generic_parameters: &Vec<Ident>,
    no_std: bool,
) -> TokenStream {
    let (field_names, field_types): (Vec<_>, Vec<_>) = components
        .iter()
        .map(
            |Component {
                 field_name,
                 field_type,
             }| { (field_name, field_type) },
        )
        .unzip();

    let used_generics: HashSet<Ident> = components
        .iter()
        .flat_map(|component| component.field_type.generics())
        .filter_map(|generic_type| {
            if let GenericType::Named(name) = generic_type {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    let (phantom_fields, phantom_types): (Vec<_>, Vec<_>) = generic_parameters
        .iter()
        .filter(|generic| !used_generics.contains(generic))
        .enumerate()
        .map(|(index, generic)| {
            let field_name = ident(&format!("_unused_generic_{index}"));
            (field_name, quote! {::core::marker::PhantomData<#generic>})
        })
        .unzip();

    let derive_default = field_names
        .is_empty()
        .then(|| quote!(::core::default::Default,));

    let maybe_disable_std = no_std.then(|| quote! {#[NoStd]});

    let generics_with_bounds = quote! {
        <#(#generic_parameters: ::fuels::core::traits::Tokenizable + ::fuels::core::traits::Parameterize, )*>
    };

    quote! {
        #[derive(
            Clone,
            Debug,
            Eq,
            PartialEq,
            #derive_default
            ::fuels::macros::Parameterize,
            ::fuels::macros::Tokenizable,
            ::fuels::macros::TryFrom
        )]
        #maybe_disable_std
        pub struct #struct_ident #generics_with_bounds {
            #(
               pub #field_names: #field_types,
            )*
            #(
               #[Ignore]
               pub #phantom_fields : #phantom_types,
            )*
        }

        impl #generics_with_bounds #struct_ident<#(#generic_parameters),*> {
            pub fn new(#(#field_names: #field_types,)*) -> Self {
                Self {
                    #(#field_names,)*
                    #(#phantom_fields: ::core::default::Default::default(),)*
                }
            }
        }
    }
}
