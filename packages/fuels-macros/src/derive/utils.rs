use fuels_code_gen::utils::TypePath;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::{Attribute, Error, Expr, ExprLit, Fields, Lit, Meta, Result, Variant};

use crate::parse_utils::has_ignore_attr;

pub(crate) fn get_path_from_attr_or(
    attr_name: &str,
    attrs: &[Attribute],
    default: TokenStream,
) -> Result<TokenStream> {
    let Some(attr) = find_attr(attr_name, attrs) else {
        return Ok(default);
    };

    let Meta::NameValue(name_value) = &attr.meta else {
        return Err(Error::new_spanned(
            attr.meta.path(),
            "expected name='value'",
        ));
    };

    let Expr::Lit(ExprLit {
        lit: Lit::Str(lit_str),
        ..
    }) = &name_value.value
    else {
        return Err(Error::new_spanned(
            &name_value.value,
            "expected string literal",
        ));
    };

    TypePath::new(lit_str.value())
        .map_err(|_| Error::new_spanned(lit_str.value(), "invalid path"))
        .map(|type_path| type_path.to_token_stream())
}

pub(crate) fn find_attr<'a>(name: &str, attrs: &'a [Attribute]) -> Option<&'a Attribute> {
    attrs.iter().find(|attr| {
        attr.path()
            .get_ident()
            .map(|ident| ident == name)
            .unwrap_or(false)
    })
}

pub(crate) struct VariantInfo {
    name: Ident,
    is_unit: bool,
}

pub(crate) enum ExtractedVariant {
    Normal {
        info: VariantInfo,
        discriminant: u64,
    },
    Ignored {
        info: VariantInfo,
    },
}

pub(crate) fn extract_variants(
    contents: impl IntoIterator<Item = Variant>,
    fuels_core_path: TokenStream,
) -> Result<ExtractedVariants> {
    let variants = contents
        .into_iter()
        .enumerate()
        .map(|(discriminant, variant)| -> Result<_> {
            let is_unit = matches!(variant.fields, Fields::Unit);
            if has_ignore_attr(&variant.attrs) {
                Ok(ExtractedVariant::Ignored {
                    info: VariantInfo {
                        name: variant.ident,
                        is_unit,
                    },
                })
            } else {
                validate_variant_type(&variant)?;

                let discriminant = discriminant.try_into().map_err(|_| {
                    Error::new_spanned(&variant.ident, "enums cannot have more than 256 variants")
                })?;

                Ok(ExtractedVariant::Normal {
                    info: VariantInfo {
                        name: variant.ident,
                        is_unit,
                    },
                    discriminant,
                })
            }
        })
        .collect::<Result<_>>()?;

    Ok(ExtractedVariants {
        variants,
        fuels_core_path,
    })
}

pub(crate) struct ExtractedVariants {
    fuels_core_path: TokenStream,
    variants: Vec<ExtractedVariant>,
}

impl ExtractedVariants {
    pub(crate) fn variant_into_discriminant_and_token(&self) -> TokenStream {
        let match_branches = self.variants.iter().map(|variant|
            match variant {
                ExtractedVariant::Normal { info: VariantInfo{ name, is_unit }, discriminant } => {
                    let fuels_core_path = &self.fuels_core_path;
                    if *is_unit {
                            quote! { Self::#name => (#discriminant, #fuels_core_path::traits::Tokenizable::into_token(())) }
                    } else {
                            quote! { Self::#name(inner) => (#discriminant, #fuels_core_path::traits::Tokenizable::into_token(inner))}
                    }
                },
                ExtractedVariant::Ignored { info: VariantInfo{ name, is_unit } } => {
                    let panic_expression = {
                        let name_stringified = name.to_string();
                        quote! {::core::panic!("variant `{}` should never be constructed", #name_stringified)}
                    };
                    if *is_unit {
                        quote! { Self::#name => #panic_expression }
                    } else {
                        quote! { Self::#name(..) => #panic_expression }
                    }
                }
            }
        );

        quote! {
            match self {
                #(#match_branches),*
            }
        }
    }
    pub(crate) fn variant_from_discriminant_and_token(&self, no_std: bool) -> TokenStream {
        let match_discriminant = self
            .variants
            .iter()
            .filter_map(|variant| match variant {
                ExtractedVariant::Normal { info, discriminant } => Some((info, discriminant)),
                _ => None,
            })
            .map(|(VariantInfo { name, is_unit }, discriminant)| {
                let fuels_core_path = &self.fuels_core_path;

                let variant_value = if *is_unit {
                    quote! {}
                } else {
                    quote! { (#fuels_core_path::traits::Tokenizable::from_token(variant_token)?) }
                };

                quote! { #discriminant => ::core::result::Result::Ok(Self::#name #variant_value)}
            });

        let std_lib = std_lib_path(no_std);
        quote! {
            match discriminant {
                #(#match_discriminant,)*
                _ => ::core::result::Result::Err(#std_lib::format!(
                    "discriminant {} doesn't point to any of the enums variants", discriminant
                )),
            }
        }
    }
}

fn validate_variant_type(variant: &Variant) -> Result<()> {
    match &variant.fields {
        Fields::Named(named_fields) => {
            return Err(Error::new_spanned(
                named_fields.clone(),
                "struct like enum variants are not supported".to_string(),
            ))
        }
        Fields::Unnamed(unnamed_fields) => {
            let fields = &unnamed_fields.unnamed;

            if fields.len() != 1 {
                return Err(Error::new_spanned(
                    unnamed_fields.clone(),
                    "tuple-like enum variants must contain exactly one element".to_string(),
                ));
            }
        }
        _ => {}
    }

    Ok(())
}

pub(crate) fn std_lib_path(no_std: bool) -> TokenStream {
    if no_std {
        quote! {::alloc}
    } else {
        quote! {::std}
    }
}
