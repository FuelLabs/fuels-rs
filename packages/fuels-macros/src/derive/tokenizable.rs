use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Data, DataEnum, DataStruct, DeriveInput, Error, Generics, Result};

use crate::{
    derive::{
        utils,
        utils::{find_attr, get_path_from_attr_or, std_lib_path},
    },
    parse_utils::{extract_struct_members, validate_and_extract_generic_types},
};

pub fn generate_tokenizable_impl(input: DeriveInput) -> Result<TokenStream> {
    let fuels_types_path =
        get_path_from_attr_or("FuelsTypesPath", &input.attrs, quote! {::fuels::types})?;
    let fuels_core_path =
        get_path_from_attr_or("FuelsCorePath", &input.attrs, quote! {::fuels::core})?;
    let no_std = find_attr("NoStd", &input.attrs).is_some();

    match input.data {
        Data::Struct(struct_contents) => tokenizable_for_struct(
            input.ident,
            input.generics,
            struct_contents,
            fuels_types_path,
            fuels_core_path,
            no_std,
        ),
        Data::Enum(enum_contents) => tokenizable_for_enum(
            input.ident,
            input.generics,
            enum_contents,
            fuels_types_path,
            fuels_core_path,
            no_std,
        ),
        _ => Err(Error::new_spanned(input, "Union type is not supported")),
    }
}

fn tokenizable_for_struct(
    name: Ident,
    generics: Generics,
    contents: DataStruct,
    fuels_types_path: TokenStream,
    fuels_core_path: TokenStream,
    no_std: bool,
) -> Result<TokenStream> {
    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();
    let struct_name_str = name.to_string();
    let members = extract_struct_members(contents, fuels_core_path.clone())?;
    let field_names = members.names().collect::<Vec<_>>();

    validate_and_extract_generic_types(&generics)?;
    let std_lib = std_lib_path(no_std);

    Ok(quote! {
        impl #impl_gen #fuels_core_path::traits::Tokenizable for #name #type_gen #where_clause {
            fn into_token(self) -> #fuels_types_path::Token {
                let tokens = #std_lib::vec![#(#fuels_core_path::traits::Tokenizable::into_token(self.#field_names)),*];
                #fuels_types_path::Token::Struct(tokens)
            }

            fn from_token(token: #fuels_types_path::Token)  -> #fuels_types_path::errors::Result<Self> {
                match token {
                    #fuels_types_path::Token::Struct(tokens) => {
                        let mut tokens_iter = tokens.into_iter();
                        let mut next_token = move || { tokens_iter
                            .next()
                            .ok_or_else(|| { #fuels_types_path::errors::Error::InstantiationError(#std_lib::format!("Ran out of tokens before '{}' has finished construction.", #struct_name_str)) })
                        };
                        ::core::result::Result::Ok(Self {
                            #(
                                #field_names: #fuels_core_path::traits::Tokenizable::from_token(next_token()?)?
                             ),*

                        })
                    },
                    other => ::core::result::Result::Err(#fuels_types_path::errors::Error::InstantiationError(#std_lib::format!("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}", #struct_name_str, other))),
                }
            }
        }
    })
}

fn tokenizable_for_enum(
    name: Ident,
    generics: Generics,
    contents: DataEnum,
    fuels_types_path: TokenStream,
    fuels_core_path: TokenStream,
    no_std: bool,
) -> Result<TokenStream> {
    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();
    let name_stringified = name.to_string();
    let variants = utils::extract_variants(&contents.variants, fuels_core_path.clone())?;
    let discriminant_and_token = variants.variant_into_discriminant_and_token();
    let constructed_variant = variants.variant_from_discriminant_and_token(no_std);

    validate_and_extract_generic_types(&generics)?;

    let std_lib = std_lib_path(no_std);

    Ok(quote! {
        impl #impl_gen #fuels_core_path::traits::Tokenizable for #name #type_gen #where_clause {
            fn into_token(self) -> #fuels_types_path::Token {
                let (discriminant, token) = #discriminant_and_token;

                let variants = match <Self as #fuels_core_path::traits::Parameterize>::param_type() {
                    #fuels_types_path::param_types::ParamType::Enum{variants, ..} => variants,
                    other => ::std::panic!("Calling {}::param_type() must return a ParamType::Enum but instead it returned: {:?}", #name_stringified, other)
                };

                #fuels_types_path::Token::Enum(#std_lib::boxed::Box::new((discriminant, token, variants)))
            }

            fn from_token(token: #fuels_types_path::Token) -> #fuels_types_path::errors::Result<Self>
            where
                Self: Sized,
            {
                match token {
                    #fuels_types_path::Token::Enum(selector) => {
                        let (discriminant, variant_token, _) = *selector;
                        #constructed_variant
                    }
                    _ => ::core::result::Result::Err(#std_lib::format!("Given token ({}) is not of the type Token::Enum.", token)),
                }.map_err(|e| #fuels_types_path::errors::Error::InvalidData(#std_lib::format!("Error while instantiating {} from token! {}", #name_stringified, e)) )
            }
        }
    })
}
