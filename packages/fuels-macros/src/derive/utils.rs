use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::{parenthesized, parse::ParseStream, Attribute, Error, Fields, LitStr, Type, Variant};

use crate::abigen::TypePath;

pub(crate) fn determine_fuels_types_path(attrs: &[Attribute]) -> syn::Result<TokenStream> {
    let attr_tokens = if let Some(attr) = find_attr("FuelsTypesPath", attrs) {
        attr.tokens.clone()
    } else {
        return Ok(quote! {::fuels::types});
    };

    let path_str = syn::parse::Parser::parse2(
        |parse_stream: ParseStream| {
            let content;
            parenthesized!(content in parse_stream);
            content.parse::<LitStr>()
        },
        attr_tokens,
    )?;

    TypePath::new(path_str.value())
        .map_err(|_| Error::new_spanned(path_str, "Invalid path."))
        .map(|type_path| type_path.to_token_stream())
}

fn find_attr<'a>(name: &str, attrs: &'a [Attribute]) -> Option<&'a Attribute> {
    attrs.iter().find(|attr| {
        attr.path
            .get_ident()
            .map(|ident| ident == name)
            .unwrap_or(false)
    })
}

pub(crate) struct ExtractedVariant {
    name: Ident,
    discriminant: u8,
    is_unit: bool,
}

pub(crate) fn extract_variants<'a>(
    contents: impl IntoIterator<Item = &'a Variant>,
    fuels_types_path: TokenStream,
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

    Ok(ExtractedVariants {
        variants,
        fuels_types_path,
    })
}

pub(crate) struct ExtractedVariants {
    fuels_types_path: TokenStream,
    variants: Vec<ExtractedVariant>,
}

impl ExtractedVariants {
    pub(crate) fn variant_into_discriminant_and_token(&self) -> TokenStream {
        let match_branches = self.variants.iter().map(|variant| {
            let discriminant = variant.discriminant;
            let name = &variant.name;
            let fuels_types_path = &self.fuels_types_path;
            if variant.is_unit {
                quote! { Self::#name => (#discriminant, #fuels_types_path::traits::Tokenizable::into_token(())) }
            } else {
                quote! { Self::#name(inner) => (#discriminant, #fuels_types_path::traits::Tokenizable::into_token(inner))}
            }
        });

        quote! {
            match self {
                #(#match_branches),*
            }
        }
    }
    pub(crate) fn variant_from_discriminant_and_token(&self) -> TokenStream {
        let match_discriminant = self.variants.iter().map(|variant| {
            let name = &variant.name;
            let discriminant = variant.discriminant;
            let fuels_tyeps_path = &self.fuels_types_path;

            let variant_value = if variant.is_unit {
                quote! {}
            } else {
                quote! { (#fuels_tyeps_path::traits::Tokenizable::from_token(variant_token)?) }
            };

            quote! { #discriminant => ::std::result::Result::Ok(Self::#name #variant_value)}
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
                    "Tuple-like enum variants must contain exactly one element.".to_string(),
                ))
            }
        }
        Fields::Unit => Ok(None),
    }
}
