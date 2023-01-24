use parse_utils::extract_struct_members;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Data, DataEnum, DataStruct, DeriveInput, Error, Fields, Generics, Type, Variant};

use crate::parse_utils;

pub fn generate_tokenizable_impl(input: DeriveInput) -> syn::Result<TokenStream> {
    match input.data {
        Data::Struct(struct_contents) => {
            tokenizable_for_struct(input.ident, input.generics, struct_contents)
        }
        Data::Enum(enum_contents) => {
            tokenizable_for_enum(input.ident, input.generics, enum_contents)
        }
        _ => Err(Error::new_spanned(input, "Union type is not supported")),
    }
}

fn tokenizable_for_struct(
    name: Ident,
    generics: Generics,
    contents: DataStruct,
) -> Result<TokenStream, Error> {
    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();

    let struct_name_str = name.to_string();

    // TODO: The quote below references `field_names` twice.
    // Check if it somehow collects it internally,
    // otherwise the iterator would be exhausted for the first repetition in quote leaving no elements behind for the second. Collecting it here is a workaround.
    let field_names = extract_struct_members(contents)?
        .names()
        .collect::<Vec<_>>();

    Ok(quote! {
        impl #impl_gen ::fuels::types::traits::Tokenizable for #name #type_gen #where_clause {
            fn into_token(self) -> ::fuels::types::Token {
                let tokens = [#(::fuels::types::traits::Tokenizable::into_token(self.#field_names)),*].to_vec();
                ::fuels::types::Token::Struct(tokens)
            }

            fn from_token(token: ::fuels::types::Token)  -> ::std::result::Result<Self, ::fuels::types::errors::Error> {
                match token {
                    ::fuels::types::Token::Struct(tokens) => {
                        let mut tokens_iter = tokens.into_iter();
                        let mut next_token = move || { tokens_iter
                            .next()
                            .ok_or_else(|| { ::fuels::types::errors::Error::InstantiationError(format!("Ran out of tokens before '{}' has finished construction!", #struct_name_str)) })
                        };
                        ::std::result::Result::Ok(Self {
                            #(
                                #field_names: ::fuels::types::traits::Tokenizable::from_token(next_token()?)?
                             ),*

                        })
                    },
                    other => ::std::result::Result::Err(::fuels::types::errors::Error::InstantiationError(format!("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}", #struct_name_str, other))),
                }
            }
        }
    })
}

struct ExtractedVariant {
    name: Ident,
    discriminant: u8,
    is_unit: bool,
}

struct ExtractedVariants {
    variants: Vec<ExtractedVariant>,
}

impl ExtractedVariants {
    fn variant_into_discriminant_and_token(&self) -> TokenStream {
        let match_branches = self.variants.iter().map(|variant| {
            let discriminant = variant.discriminant;
            let name = &variant.name;
            if variant.is_unit {
                quote! { Self::#name => (#discriminant, ::fuels::types::traits::Tokenizable::into_token(())) }
            } else {
                quote! { Self::#name(inner) => (#discriminant, ::fuels::types::traits::Tokenizable::into_token(inner))}
            }
        });

        quote! {
            match self {
                #(#match_branches),*
            }
        }
    }
    fn variant_from_discriminant_and_token(&self) -> TokenStream {
        let match_discriminant = self.variants.iter().map(|variant| {
            let name = &variant.name;
            let discriminant = variant.discriminant;

            let arg = if variant.is_unit {
                quote! {}
            } else {
                quote! { (::fuels::types::traits::Tokenizable::from_token(variant_token)?) }
            };

            quote! { #discriminant => ::std::result::Result::Ok(Self::#name #arg)}
        });

        quote! {
            match discriminant {
                #(#match_discriminant,)*
                _ => ::std::result::Result::Err(format!(
                    "Discriminant {} doesn't point to any of the enums variants.", discriminant
                )),
            }
        }
    }
}

fn extract_variants<'a>(
    contents: impl IntoIterator<Item = &'a Variant>,
) -> Result<ExtractedVariants, Error> {
    let variants = contents
        .into_iter()
        .enumerate()
        .map(|(discriminant, variant)| -> syn::Result<_> {
            let name = variant.ident.clone();
            let ty = get_variant_type(variant)?;
            Ok(ExtractedVariant {
                name,
                discriminant: discriminant as u8,
                is_unit: ty.is_none(),
            })
        })
        .collect::<Result<_, _>>()?;

    Ok(ExtractedVariants { variants })
}

fn tokenizable_for_enum(
    name: Ident,
    generics: Generics,
    contents: DataEnum,
) -> Result<TokenStream, Error> {
    let (impl_gen, type_gen, where_clause) = generics.split_for_impl();

    let name_stringified = name.to_string();

    let variants = extract_variants(&contents.variants)?;
    let discriminant_and_token = variants.variant_into_discriminant_and_token();
    let constructed_variant = variants.variant_from_discriminant_and_token();

    Ok(quote! {
        impl #impl_gen ::fuels::types::traits::Tokenizable for #name #type_gen #where_clause {
            fn into_token(self) -> ::fuels::types::Token {
                let (discriminant, token) = #discriminant_and_token;

                let variants = match <Self as ::fuels::types::traits::Parameterize>::param_type() {
                    ::fuels::types::param_types::ParamType::Enum{variants, ..} => variants,
                    other => panic!("Calling {}::param_type() must return a ParamType::Enum but instead it returned: {:?}", #name_stringified, other)
                };

                ::fuels::types::Token::Enum(::std::boxed::Box::new((discriminant, token, variants)))
            }

            fn from_token(token: ::fuels::types::Token) -> ::std::result::Result<Self, ::fuels::types::errors::Error>
            where
                Self: Sized,
            {
                match token {
                    ::fuels::types::Token::Enum(selector) => {
                        let (discriminant, variant_token, _) = *selector;
                        #constructed_variant
                    }
                    _ => ::std::result::Result::Err(format!("Given token ({}) is not of the type Token::Enum!", token)),
                }.map_err(|e| ::fuels::types::errors::Error::InvalidData(format!("Error while instantiating {} from token! {}", #name_stringified, e)) )
            }
        }
    })
}

fn get_variant_type(variant: &Variant) -> syn::Result<Option<&Type>> {
    match &variant.fields {
        Fields::Named(named_fields) => Err(Error::new_spanned(
            named_fields.clone(),
            "Struct like enum variants are not supported".to_string(),
        )),
        Fields::Unnamed(unnamed_fields) => {
            let fields = &unnamed_fields.unnamed;

            if fields.len() == 1 {
                Ok(fields.iter().next().map(|field| &field.ty))
            } else {
                Err(Error::new_spanned(
                    unnamed_fields.clone(),
                    "Tuple-like enum variants must contain exactly one element!".to_string(),
                ))
            }
        }
        Fields::Unit => Ok(None),
    }
}
