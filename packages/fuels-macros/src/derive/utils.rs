use fuels_code_gen::utils::TypePath;
use proc_macro2::{Ident, TokenStream};
use quote::{quote, ToTokens};
use syn::{Attribute, Error, Expr, ExprLit, Fields, Lit, Meta, Result, Type, Variant};

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
            "Expected name='value'",
        ));
    };

    let Expr::Lit(ExprLit {
        lit: Lit::Str(lit_str),
        ..
    }) = &name_value.value
    else {
        return Err(Error::new_spanned(
            &name_value.value,
            "Expected string literal",
        ));
    };

    TypePath::new(lit_str.value())
        .map_err(|_| Error::new_spanned(lit_str.value(), "Invalid path."))
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
    Normal { info: VariantInfo, discriminant: u8 },
    Ignored { info: VariantInfo },
}

pub(crate) fn extract_variants<'a>(
    contents: impl IntoIterator<Item = &'a Variant>,
    fuels_core_path: TokenStream,
) -> Result<ExtractedVariants> {
    let mut discriminant = 0;
    let variants = contents
        .into_iter()
        .map(|variant| -> Result<_> {
            let ignored = variant.attrs.iter().any(|attr| match &attr.meta {
                syn::Meta::Path(path) => path.get_ident().is_some_and(|ident| ident == "Ignore"),
                _ => false,
            });
            let name = variant.ident.clone();

            if ignored {
                let is_unit = matches!(variant.fields, Fields::Unit);
                Ok(ExtractedVariant::Ignored {
                    info: VariantInfo { name, is_unit },
                })
            } else {
                let is_unit = validate_and_extract_variant_type(variant)?.is_none();
                let current_discriminant = discriminant;
                discriminant += 1;

                Ok(ExtractedVariant::Normal {
                    info: VariantInfo { name, is_unit },
                    discriminant: current_discriminant,
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
        let match_branches = self.variants.iter().map(|variant| {
            let fuels_core_path = &self.fuels_core_path;

            match variant {
                ExtractedVariant::Normal { info: VariantInfo{ name, is_unit }, discriminant } => {
                if *is_unit {
                        quote! { Self::#name => (#discriminant, #fuels_core_path::traits::Tokenizable::into_token(())) }
                } else {
                        quote! { Self::#name(inner) => (#discriminant, #fuels_core_path::traits::Tokenizable::into_token(inner))}
                }
                },
                ExtractedVariant::Ignored { info: VariantInfo{ name, is_unit } } => {
                let name_stringified = name.to_string();
                if *is_unit {
                    quote! { Self::#name => ::core::panic!("Variant '{}' should never be constructed.", #name_stringified) }
                } else {
                    quote! { Self::#name(..) => ::core::panic!("Variant '{}' should never be constructed.", #name_stringified) }
                }
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
                    "Discriminant {} doesn't point to any of the enums variants.", discriminant
                )),
            }
        }
    }
}

fn validate_and_extract_variant_type(variant: &Variant) -> Result<Option<&Type>> {
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

pub(crate) fn std_lib_path(no_std: bool) -> TokenStream {
    if no_std {
        quote! {::alloc}
    } else {
        quote! {::std}
    }
}
