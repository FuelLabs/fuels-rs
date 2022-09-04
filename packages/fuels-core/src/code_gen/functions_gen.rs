use crate::code_gen::abigen::Abigen;
use crate::code_gen::custom_types_gen::extract_custom_type_name_from_abi_property;
use crate::code_gen::docs_gen::expand_doc;
use crate::types::expand_type;
use crate::utils::{first_four_bytes_of_sha256_hash, ident, safe_ident};
use crate::{ParamType, Selector};
use fuels_types::errors::Error;
use fuels_types::function_selector::build_fn_selector;
use fuels_types::{
    ABIFunction, CustomType, TypeApplication, TypeDeclaration, ENUM_KEYWORD, STRUCT_KEYWORD,
};
use inflector::Inflector;
use itertools::{chain, Either, Itertools};
use proc_macro2::{Ident, Literal, TokenStream};
use quote::{quote, ToTokens};
use regex::Regex;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::iter;
use std::str::FromStr;
use syn::Expr::Type;

/// Functions used by the Abigen to expand functions defined in an ABI spec.

/// Transforms a function defined in [`Function`] into a [`TokenStream`]
/// that represents that same function signature as a Rust-native function
/// declaration.
/// The actual logic inside the function is the function `method_hash` under
/// [`Contract`], which is responsible for encoding the function selector
/// and the function parameters that will be used in the actual contract call.
///
/// [`Contract`]: crate::contract::Contract
pub fn expand_function(
    function: &ABIFunction,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    if function.name.is_empty() {
        return Err(Error::InvalidData("Function name can not be empty".into()));
    }

    let fn_param_types = function
        .inputs
        .iter()
        .map(|t| types.get(&t.type_id).unwrap().clone())
        .collect::<Vec<TypeDeclaration>>();

    let name = safe_ident(&function.name);
    let fn_signature = build_fn_selector(&function.name, &fn_param_types, types)?;

    let encoded = first_four_bytes_of_sha256_hash(&fn_signature);

    let tokenized_signature = expand_selector(encoded);

    let resolved_output_type = expand_fn_output(&function.output, types)?;

    let (input, arg) = expand_function_arguments(function, types)?;

    let doc = expand_doc(&format!(
        "Calls the contract's `{}` (0x{}) function",
        function.name,
        hex::encode(encoded)
    ));

    let t = types
        .get(&function.output.type_id)
        .expect("couldn't find type");

    let param_type = ParamType::from_type_declaration(t, types)?;

    let generics = resolved_output_type
        .generic_params
        .iter()
        .cloned()
        .map(|gen| TokenStream::from(gen))
        .collect::<Vec<_>>();
    let output_type_name = resolved_output_type.type_name.clone();
    let tok = if t.is_array() || generics.is_empty() {
        format!("Some(ParamType::{param_type})").parse()?
    } else {
        quote! { Some(#output_type_name::<#(#generics,)*>::param_type()) }
    };

    let output_param = tok;

    let output_type_tokenized: TokenStream = resolved_output_type.into();
    let result = quote! { ContractCallHandler<#output_type_tokenized> };

    Ok(quote! {
        #doc
        pub fn #name(&self #input) -> #result {
            Contract::method_hash(&self.wallet.get_provider().expect("Provider not set up"), self.contract_id.clone(), &self.wallet,
                #tokenized_signature, #output_param, #arg).expect("method not found (this should never happen)")
        }
    })
}

fn expand_selector(selector: Selector) -> TokenStream {
    let bytes = selector.iter().copied().map(Literal::u8_unsuffixed);
    quote! { [#( #bytes ),*] }
}
#[derive(Debug, Clone)]
struct ResolvedType {
    pub type_name: TokenStream,
    pub generic_params: Vec<ResolvedType>,
}

impl Display for ResolvedType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", TokenStream::from(self.clone()).to_string())
    }
}

impl From<ResolvedType> for TokenStream {
    fn from(resolved_type: ResolvedType) -> Self {
        let type_name = resolved_type.type_name;
        if resolved_type.generic_params.is_empty() {
            return quote! { #type_name };
        }

        let generic_params = resolved_type
            .generic_params
            .into_iter()
            .map(|generic_type| TokenStream::from(generic_type))
            .collect::<Vec<_>>();

        quote! { #type_name<#( #generic_params ),*> }
    }
}

fn resolve_type(
    type_application: &TypeApplication,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<ResolvedType, Error> {
    let base_type = types.get(&type_application.type_id).unwrap();

    if !base_type.is_custom_type(&types) {
        return Ok(ResolvedType {
            type_name: expand_type(&ParamType::from_type_declaration(base_type, types)?)?,
            generic_params: vec![],
        });
    }

    if base_type.is_array() {
        let array_type = base_type
            .components
            .iter()
            .flatten()
            .map(|array_type| resolve_type(&array_type, &types))
            .next()
            .expect("An array must have components!")
            .map(TokenStream::from)?;

        let len = extract_arr_length(base_type);

        return Ok(ResolvedType {
            type_name: quote! { [#array_type; #len] },
            generic_params: vec![],
        });
    }

    if base_type.is_tuple() {
        let inner_types = base_type
            .components
            .iter()
            .flatten()
            .map(|array_type| resolve_type(&array_type, &types))
            .map_ok(|resolved_type| TokenStream::from(resolved_type))
            .collect::<Result<Vec<_>, _>>()?;

        let resolved_type1 = ResolvedType {
            type_name: quote! {(#(#inner_types,)*)},
            generic_params: vec![],
        };
        return Ok(resolved_type1);
    }

    let base_type_name = extract_custom_type_name_from_abi_property(&base_type, None, &types)?;
    let inner_types = type_application
        .type_arguments
        .iter()
        .flatten()
        .map(|something| resolve_type(something, types))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ResolvedType {
        type_name: base_type_name.parse().unwrap(),
        generic_params: inner_types,
    })
}

fn extract_arr_length(base_type: &TypeDeclaration) -> usize {
    let regex = Regex::new(r"^\[.*;\s*(\d+)]").unwrap();
    let x = &regex.captures(&base_type.type_field).unwrap()[1];
    x.parse().unwrap()
}

fn expand_fn_output(
    output: &TypeApplication,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<ResolvedType, Error> {
    let output_type = types.get(&output.type_id).expect("couldn't find type");

    // If it's a primitive type, simply parse and expand.
    if !output_type.is_custom_type(types) {
        return resolve_type(&output, &types);
    }

    // If it's a {struct, enum} as the type of a function's output, use its tokenized name only.
    if output_type.is_custom_type(&types) {
        Ok(resolve_type(&output, &types)?.into())
    } else if output_type.has_custom_type_in_array(types) {
        let type_inside_array = types
            .get(
                &output_type
                    .components
                    .as_ref()
                    .expect("array should have components")[0]
                    .type_id,
            )
            .expect("couldn't find type");

        let parsed_custom_type_name: TokenStream = extract_custom_type_name_from_abi_property(
            type_inside_array,
            Some(
                type_inside_array
                    .get_custom_type()
                    .expect("Custom type in array should be set"),
            ),
            types,
        )?
        .parse()
        .expect("couldn't parse custom type name");

        let len = extract_arr_length(&output_type);

        Ok(ResolvedType {
            type_name: quote! { [#parsed_custom_type_name; #len] },
            generic_params: vec![],
        })
    } else {
        let type_name = expand_tuple_w_custom_types(output_type, types)?;
        Ok(ResolvedType {
            type_name,
            generic_params: vec![],
        })
    }
}

fn expand_tuple_w_custom_types(
    output: &TypeDeclaration,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    if !output.has_custom_type_in_tuple(types) {
        panic!("Output is of custom type, but not an enum, struct or enum/struct inside an array/tuple. This should never happen. Output received: {:?}", output);
    }

    let mut final_signature: String = "(".into();
    let mut type_strings: Vec<String> = vec![];

    for c in output
        .components
        .as_ref()
        .expect("tuples should have components")
        .iter()
    {
        let type_string = types.get(&c.type_id).unwrap().type_field.clone();

        // If custom type is inside a tuple `(struct | enum <name>, ...)`,
        // the type signature should be only `(<name>, ...)`.
        // To do that, we remove the `STRUCT_KEYWORD` and `ENUM_KEYWORD` from it.
        let keywords_removed = remove_words(&type_string, &[STRUCT_KEYWORD, ENUM_KEYWORD]);

        let tuple_type_signature = expand_b256_into_array_form(&keywords_removed)
            .parse()
            .expect("could not parse tuple type signature");

        type_strings.push(tuple_type_signature);
    }

    final_signature.push_str(&type_strings.join(", "));
    final_signature.push(')');

    Ok(final_signature.parse().unwrap())
}

fn expand_b256_into_array_form(type_field: &str) -> String {
    let re = Regex::new(r"\bb256\b").unwrap();
    re.replace_all(type_field, "Bits256").to_string()
}

fn remove_words(from: &str, words: &[&str]) -> String {
    words
        .iter()
        .fold(from.to_string(), |str_in_construction, word| {
            str_in_construction.replace(word, "")
        })
}

/// Expands the arguments in a function declaration and the same arguments as input
fn expand_function_arguments(
    fun: &ABIFunction,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<(TokenStream, TokenStream), Error> {
    let mut args = vec![];
    let mut call_args = vec![];

    for fn_type_application in &fun.inputs {
        // For each [`TypeDeclaration`] in a function input we expand:
        // 1. The name of the argument;
        // 2. The type of the argument;
        // Note that _any_ significant change in the way the JSON ABI is generated
        // could affect this function expansion.
        // TokenStream representing the name of the argument

        let name = expand_input_name(&fn_type_application.name)?;

        let param = types
            .get(&fn_type_application.type_id)
            .expect("couldn't find type");

        // TokenStream representing the type of the argument
        let kind = ParamType::from_type_declaration(param, types)?;

        // If it's a tuple, don't expand it, just use the type signature as it is (minus the string "struct " | "enum ").
        let tok = if let ParamType::Tuple(_tuple) = &kind {
            let toks = build_expanded_tuple_params(param, types)
                .expect("failed to build expanded tuple parameters");

            toks.parse::<TokenStream>().unwrap()
        } else {
            resolve_type(&fn_type_application, &types)?.into()
        };

        // Add the TokenStream to argument declarations
        args.push(quote! { #name: #tok });

        // This `name` TokenStream is also added to the call arguments
        call_args.push(name);
    }

    // The final TokenStream of the argument declaration in a function declaration
    let args = quote! { #( , #args )* };

    // The final TokenStream of the arguments being passed in a function call
    // It'll look like `&[my_arg.into_token(), another_arg.into_token()]`
    // as the [`Contract`] `method_hash` function expects a slice of Tokens
    // in order to encode the call.
    let call_args = quote! { &[ #(#call_args.into_token(), )* ] };

    Ok((args, call_args))
}

// Builds a string "(type_1,type_2,type_3,...,type_n,)"
fn build_expanded_tuple_params(
    tuple_param: &TypeDeclaration,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<String, Error> {
    let mut toks: String = "(".to_string();
    for type_application in tuple_param
        .components
        .as_ref()
        .expect("tuple parameter should have components")
    {
        let component = types
            .get(&type_application.type_id)
            .expect("couldn't find type");

        if !component.is_custom_type(types) {
            let p = ParamType::from_type_declaration(component, types)?;
            let tok = expand_type(&p)?;
            toks.push_str(&tok.to_string());
        } else {
            let tok: TokenStream = resolve_type(&type_application, &types)?.into();

            toks.push_str(&tok.to_string());
        }
        toks.push(',');
    }
    toks.push(')');
    Ok(toks)
}

/// Expands a positional identifier string that may be empty.
///
/// Note that this expands the parameter name with `safe_ident`, meaning that
/// identifiers that are reserved keywords get `_` appended to them.
pub fn expand_input_name(name: &str) -> Result<TokenStream, Error> {
    if name.is_empty() {
        return Err(Error::InvalidData(
            "Function arguments can not have empty names".into(),
        ));
    }
    let name = safe_ident(&name.to_snake_case());
    Ok(quote! { #name })
}

fn gen_parameterize_impl(
    input_type: &TypeApplication,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    let base_type = types.get(&input_type.type_id).unwrap();
    if base_type.is_struct_type() {
        gen_parameterize_impl_struct(input_type, types)
    } else {
        gen_parameterize_impl_enum(input_type, types)
    }
}

fn only_types_which_should_generate_impls(
    type_application: &TypeApplication,
    types: &HashMap<usize, TypeDeclaration>,
) -> bool {
    let type_decl = types.get(&type_application.type_id).unwrap();
    (type_decl.is_struct_type() || type_decl.is_enum_type())
        && !Abigen::is_sway_native_type(&type_decl.type_field)
}

pub fn extract_nongeneric_type_declarations(
    types: &HashMap<usize, TypeDeclaration>,
) -> Vec<TypeDeclaration> {
    types
                 .values()
                 .filter(|type_declaration| {
                     let is_generic = matches!(&type_declaration.type_parameters, Some(parameters) if !parameters.is_empty() );
                     !is_generic
                    })
                 .cloned()
                 .collect()
}

pub fn unravel_type_application(type_application: &TypeApplication) -> Vec<&TypeApplication> {
    type_application
        .type_arguments
        .iter()
        .flatten()
        .flat_map(|type_argument| unravel_type_application(type_argument))
        .chain(iter::once(type_application))
        .collect()
}

fn filter_all_unique_used_types(
    functions: &[ABIFunction],
    types: &HashMap<usize, TypeDeclaration>,
) -> Vec<TypeApplication> {
    functions
        .iter()
        .flat_map(|fun| chain!(&fun.inputs, iter::once(&fun.output)))
        .flat_map(|type_application| unravel_type_application(type_application))
        .cloned()
        .chain(
            extract_nongeneric_type_declarations(&types)
                .into_iter()
                .map(|type_declaration| TypeApplication {
                    name: Default::default(),
                    type_id: type_declaration.type_id,
                    type_arguments: None,
                }),
        )
        .unique_by(|el| (el.type_id, el.type_arguments.clone()))
        .collect::<Vec<_>>()
}

pub fn gen_trait_impls(
    functions: &[ABIFunction],
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    filter_all_unique_used_types(functions, types)
        .into_iter()
        .filter(|type_application| only_types_which_should_generate_impls(type_application, types))
        .flat_map(|type_application| {
            [
                gen_parameterize_impl(&type_application, &types),
                gen_tokenize_impl(&type_application, &types),
                gen_try_from_byte_slice_struct(&type_application, &types),
                gen_try_from_bytevec_ref_struct(&type_application, &types),
                gen_try_from_bytevec_struct(&type_application, &types),
            ]
        })
        .collect()
}

fn gen_tokenize_impl(
    type_application: &TypeApplication,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    let base_type = types.get(&type_application.type_id).unwrap();
    if base_type.is_struct_type() {
        gen_tokenize_impl_struct(&type_application, &types)
    } else {
        gen_tokenize_impl_enum(&type_application, &types)
    }
}

fn gen_parameterize_impl_struct(
    input_type: &TypeApplication,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    let resolved_type = resolve_type(&input_type, &types)?;

    let mut resolved_generics = &resolved_type.generic_params;
    let mut current_generic_index = 0;
    let mut prev_generic_name = String::new();
    let param_types = extract_components(&input_type, types)
        .into_iter()
        .map(|type_decl| -> Result<TokenStream, Error> {
            let param_type = ParamType::from_type_declaration(type_decl, types)?;
            let token = match param_type {
                ParamType::Struct(_) | ParamType::Enum(_) => {
                    let custom_type_ident = ident(&extract_custom_type_name_from_abi_property(
                        type_decl, None, types,
                    )?);

                    quote! { types.push(#custom_type_ident::param_type()) }
                }
                ParamType::Generic(name) => {
                    if prev_generic_name != "" && name != prev_generic_name {
                        current_generic_index += 1;
                    }
                    prev_generic_name = name;
                    let resolved_generic = resolved_generics[current_generic_index].clone();

                    let type_name = resolved_generic.type_name;
                    let tokenized_generic_parameters = resolved_generic
                        .generic_params
                        .into_iter()
                        .map(|param| TokenStream::from(param))
                        .collect::<Vec<_>>();
                    let stream = if tokenized_generic_parameters.is_empty() {
                        quote! { #type_name::param_type() }
                    } else {
                        quote! { #type_name::<#(#tokenized_generic_parameters,)*>::param_type() }
                    };
                    quote! {types.push(#stream)}
                }
                _ => {
                    let param_type_string_ident_tok: TokenStream =
                        param_type.to_string().parse()?;

                    quote! { types.push(ParamType::#param_type_string_ident_tok) }
                }
            };
            Ok(token)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let tokenized_resolved_type: TokenStream = resolved_type.into();

    Ok(quote! {
        impl Parameterize for #tokenized_resolved_type {
            fn param_type() -> ParamType {
                let mut types = Vec::new();
                #( #param_types; )*
                ParamType::Struct(types)
            }
        }
    })
}
fn gen_parameterize_impl_enum(
    input_type: &TypeApplication,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    let resolved_type = resolve_type(&input_type, &types)?;
    let base_type = types.get(&input_type.type_id).unwrap();

    let mut resolved_generics = &resolved_type.generic_params;
    let mut current_generic_index = 0;
    let mut prev_generic_name = String::new();

    let components = match &base_type.components {
        Some(components) if !components.is_empty() => Ok(components),
        _ => Err(Error::InvalidType(format!(
            "Enum '{}' must have at least one variant!",
            TokenStream::from(resolved_type.clone()).to_string()
        ))),
    }?;

    let param_types = components
        .iter()
        .map(|component| {
            let t = types.get(&component.type_id).expect("couldn't find type");
            let param_type = ParamType::from_type_declaration(t, types)?;

            let stream = match param_type {
                // Case where an enum takes another enum
                ParamType::Enum(_) | ParamType::Struct(_) => {
                    let inner_name = &extract_custom_type_name_from_abi_property(t, None, types)?;

                    let inner_ident = ident(inner_name);

                    quote! { types.push(#inner_ident::param_type()) }
                }
                ParamType::Generic(name) => {
                    if prev_generic_name != "" && name != prev_generic_name {
                        current_generic_index += 1;
                    }
                    prev_generic_name = name;
                    let resolved_generic = resolved_generics[current_generic_index].clone();

                    let type_name = resolved_generic.type_name;
                    let tokenized_generic_parameters = resolved_generic
                        .generic_params
                        .into_iter()
                        .map(|param| TokenStream::from(param))
                        .collect::<Vec<_>>();

                    let stream = if tokenized_generic_parameters.is_empty() {
                        quote! { #type_name::param_type() }
                    } else {
                        quote! { #type_name::<#(#tokenized_generic_parameters,)*>::param_type() }
                    };
                    quote! { types.push(#stream) }
                }
                _ => {
                    let param_type_string_ident_tok =
                        TokenStream::from_str(&param_type.to_string())?;
                    quote! { types.push(ParamType::#param_type_string_ident_tok) }
                }
            };
            Ok::<TokenStream, Error>(stream)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let type_name: TokenStream = resolved_type.into();
    let stringified_type_name = type_name.to_string();
    Ok(quote! {
        impl Parameterize for #type_name {
            fn param_type() -> ParamType {
                let mut types = Vec::new();
                #( #param_types; )*

                let variants = EnumVariants::new(types).expect(concat!("Enum ", #stringified_type_name, " has no variants! 'abigen!' should not have succeeded!"));

                ParamType::Enum(variants)
            }
        }
    })
}

fn gen_tokenize_impl_enum(
    input_type: &TypeApplication,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    let resolved_type = resolve_type(&input_type, &types)?;

    let mut resolved_generics = &resolved_type.generic_params;
    let mut current_generic_index = 0;
    let mut prev_generic_name = String::new();

    let base_type = types.get(&input_type.type_id).unwrap();
    let enum_name =
        &extract_custom_type_name_from_abi_property(base_type, Some(CustomType::Enum), types)?;

    let components = match &base_type.components {
        Some(components) if !components.is_empty() => Ok(components),
        _ => Err(Error::InvalidType(format!(
            "Enum '{}' must have at least one variant!",
            enum_name
        ))),
    }?;

    // Holds a TokenStream representing the process of creating an enum [`Token`].
    let mut enum_selector_builder = Vec::new();

    // Holds the TokenStream representing the process of creating a Self enum from each `Token`.
    // Used when creating a struct from tokens with `Tokenizable::from_token()`.
    let mut args = Vec::new();

    let enum_ident: TokenStream = resolved_type.type_name.clone();

    for (discriminant, component) in components.iter().enumerate() {
        let variant_name = ident(&component.name);
        let dis = discriminant as u8;
        let t = types.get(&component.type_id).expect("couldn't find type");
        let param_type = ParamType::from_type_declaration(t, types)?;

        match param_type {
            // Case where an enum takes another enum
            ParamType::Enum(_) | ParamType::Struct(_) => {
                let inner_name = &extract_custom_type_name_from_abi_property(t, None, types)?;

                let inner_ident = ident(inner_name);
                // Token creation
                enum_selector_builder.push(quote! {
                    #enum_ident::#variant_name(inner_enum) =>
                    (#dis, inner_enum.into_token())
                });

                args.push(quote! {
                    (#dis, token, _) => {
                        let variant_content = <#inner_ident>::from_token(token)?;
                        Ok(#enum_ident::#variant_name(variant_content))
                    }
                });
            }
            _ => {
                let ty = expand_type(&param_type)?;

                let param_type_string = match param_type {
                    ParamType::Array(..) => "Array".to_string(),
                    ParamType::String(..) => "String".to_string(),
                    _ => param_type.to_string(),
                };
                let param_type_string_ident = ident(&param_type_string);

                // Token creation
                match param_type {
                    ParamType::String(_len) => {
                        args.push(
                            quote! {(#dis, token, _) => Ok(#enum_ident::#variant_name(<#ty>::from_token(token)?)),},
                        );
                        enum_selector_builder.push(quote! {
                            #enum_ident::#variant_name(value) => (#dis, value.into_token())
                        });
                    }
                    ParamType::Array(_t, _s) => {
                        args.push(
                            quote! {(#dis, token, _) => Ok(#enum_ident::#variant_name(<#ty>::from_token(token)?)),},
                        );
                        enum_selector_builder.push(quote! {
                            #enum_ident::#variant_name(value) => (#dis, Token::#param_type_string_ident(vec![value.into_token()]))
                        });
                    }
                    ParamType::Unit => {
                        args.push(quote! {(#dis, token, _) => Ok(#enum_ident::#variant_name()),});
                        enum_selector_builder.push(quote! {
                            #enum_ident::#variant_name() => (#dis, Token::#param_type_string_ident)
                        });
                    }
                    ParamType::Generic(name) => {
                        if prev_generic_name != "" && name != prev_generic_name {
                            current_generic_index += 1;
                        }
                        prev_generic_name = name;
                        let resolved_generic = resolved_generics[current_generic_index].clone();

                        let tokenized_generic = TokenStream::from(resolved_generic);

                        args.push(
                            quote! {(#dis, token, _) => Ok(#enum_ident::<#tokenized_generic>::#variant_name(<#tokenized_generic>::from_token(token)?)),},
                        );

                        enum_selector_builder.push(quote! {
                                #enum_ident::<#tokenized_generic>::#variant_name(value) => (#dis, value.into_token())
                        });
                    }
                    // Primitive type
                    _ => {
                        args.push(
                            quote! {(#dis, token, _) => Ok(#enum_ident::#variant_name(<#ty>::from_token(token)?)),},
                        );
                        enum_selector_builder.push(quote! {
                            #enum_ident::#variant_name(value) => (#dis, value.into_token())
                        });
                    }
                }
            }
        }
    }

    let resolve_type_tokenstream: TokenStream = resolved_type.into();
    Ok(quote! {
        impl Tokenizable for #resolve_type_tokenstream {
            fn into_token(self) -> Token {
                let (dis, tok) = match self {
                    #( #enum_selector_builder, )*
                };

                let variants = match Self::param_type() {
                    ParamType::Enum(variants) => variants,
                    other => panic!("Calling ::param_type() on a custom enum must return a ParamType::Enum but instead it returned: {}", other)
                };

                let selector = (dis, tok, variants);
                Token::Enum(Box::new(selector))
            }

            fn from_token(token: Token)  -> Result<Self, SDKError> {
                if let Token::Enum(enum_selector) = token {
                        match *enum_selector {
                            #( #args )*
                            (_, _, _) => Err(SDKError::InstantiationError(format!("Could not construct '{}'. Failed to match with discriminant selector {:?}", #enum_name, enum_selector)))
                        }
                }
                else {
                    Err(SDKError::InstantiationError(format!("Could not construct '{}'. Expected a token of type Token::Enum, got {:?}", #enum_name, token)))
                }
            }
        }
    })
}

fn extract_components<'a>(
    input_type: &'a TypeApplication,
    types: &'a HashMap<usize, TypeDeclaration>,
) -> Vec<&'a TypeDeclaration> {
    types
        .get(&input_type.type_id)
        .unwrap()
        .components
        .as_ref()
        .expect("Fail to extract components from custom type")
        .into_iter()
        .map(|component| types.get(&component.type_id).expect("couldn't find type"))
        .collect()
}

fn gen_tokenize_impl_struct(
    input_type: &TypeApplication,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    let resolved_type = resolve_type(&input_type, &types)?;
    let mut resolved_generics = &resolved_type.generic_params;
    let mut current_generic_index = 0;
    let mut prev_generic_name = String::new();

    let tokenized_resolved_type: TokenStream = resolved_type.clone().into();
    let stringified_resolved_type: String = tokenized_resolved_type.to_string();

    let components = types
        .get(&input_type.type_id)
        .unwrap()
        .components
        .as_ref()
        .expect("Fail to extract components from custom type");

    let mut struct_fields_tokens = Vec::new();

    let mut args = Vec::new();

    for component in components {
        let field_name = ident(&component.name.to_snake_case());

        let t = types.get(&component.type_id).expect("couldn't find type");

        let param_type = ParamType::from_type_declaration(t, types)?;
        match param_type {
            ParamType::Struct(_) | ParamType::Enum(_) => {
                let inner_ident =
                    ident(&extract_custom_type_name_from_abi_property(t, None, types)?);

                args.push(quote! {#field_name: #inner_ident::from_token(next_token()?)?});
                struct_fields_tokens.push(quote! { tokens.push(self.#field_name.into_token()) });
            }
            _ => {
                let ty = expand_type(&param_type)?;

                let param_type_string = match param_type {
                    ParamType::Array(..) => "Array".to_string(),
                    ParamType::String(..) => "String".to_string(),
                    _ => param_type.to_string(),
                };

                let param_type_string_ident = ident(&param_type_string);

                // Check if param type is generic
                if let ParamType::Generic(name) = param_type {
                    if prev_generic_name != "" && name != prev_generic_name {
                        current_generic_index += 1;
                    }
                    prev_generic_name = name;
                    let resolved_generic = resolved_generics[current_generic_index].clone();
                    let type_name = resolved_generic.type_name;
                    let generic_params = resolved_generic
                        .generic_params
                        .into_iter()
                        .map(|param| TokenStream::from(param))
                        .collect::<Vec<_>>();
                    let stream = if generic_params.is_empty() {
                        quote! {#field_name: #type_name::from_token(next_token()?)?}
                    } else {
                        quote! {#field_name: #type_name::<#(#generic_params,)*>::from_token(next_token()?)?}
                    };
                    args.push(stream);
                    struct_fields_tokens
                        .push(quote! { tokens.push(self.#field_name.into_token()) });
                } else {
                    args.push(quote! {
                        #field_name: <#ty>::from_token(next_token()?)?
                    });

                    let stream = match param_type {
                        ParamType::String(len) => {
                            quote! {tokens.push(self.#field_name.into_token())}
                        }
                        ParamType::Array(..) => {
                            quote! {tokens.push(Token::#param_type_string_ident(vec![self.#field_name.into_token()]))}
                        }
                        _ => {
                            quote! {tokens.push(self.#field_name.into_token())}
                        }
                    };
                    struct_fields_tokens.push(stream);
                }
            }
        }
    }

    Ok(quote! {
        impl Tokenizable for #tokenized_resolved_type {
            fn into_token(self) -> Token {
                let mut tokens = Vec::new();
                #( #struct_fields_tokens; )*
                Token::Struct(tokens)
            }

            fn from_token(token: Token)  -> Result<Self, SDKError> {
                match token {
                    Token::Struct(tokens) => {
                        let mut tokens_iter = tokens.into_iter();
                        let mut next_token = move || { tokens_iter
                            .next()
                            .ok_or_else(|| { SDKError::InstantiationError(format!("Ran out of tokens before '{}' has finished construction!", #stringified_resolved_type)) })
                        };
                        Ok(Self { #( #args ),* })
                    },
                    other => Err(SDKError::InstantiationError(format!("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}", #stringified_resolved_type, other))),
                }
            }
        }
    })
}

fn gen_try_from_byte_slice_struct(
    type_application: &TypeApplication,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    let resolved_type_token_stream: TokenStream = resolve_type(&type_application, &types)?.into();
    let token_stream = quote! {
        impl TryFrom<&[u8]> for #resolved_type_token_stream {
            type Error = SDKError;

            fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
                try_from_bytes(bytes)
            }
        }
    };
    Ok(token_stream)
}

fn gen_try_from_bytevec_ref_struct(
    type_application: &TypeApplication,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    let resolved_type_token_stream: TokenStream = resolve_type(&type_application, &types)?.into();
    let token_stream = quote! {
        impl TryFrom<&Vec<u8>> for #resolved_type_token_stream {
            type Error = SDKError;

            fn try_from(bytes: &Vec<u8>) -> Result<Self, Self::Error> {
                try_from_bytes(bytes)
            }
        }
    };
    Ok(token_stream)
}

fn gen_try_from_bytevec_struct(
    type_application: &TypeApplication,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    let resolved_type_token_stream: TokenStream = resolve_type(&type_application, &types)?.into();
    let token_stream = quote! {
        impl TryFrom<Vec<u8>> for #resolved_type_token_stream {
            type Error = SDKError;

            fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
                try_from_bytes(&bytes)
            }
        }
    };
    Ok(token_stream)
}
fn expand_input_param(
    fun: &ABIFunction,
    type_application: &TypeApplication,
    kind: &ParamType,
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<TokenStream, Error> {
    match kind {
        ParamType::Array(ty, size) => {
            let ty = expand_input_param(fun, type_application, ty, types)?;
            Ok(quote! {
                [#ty; #size]
            })
        }
        ParamType::Enum(_) => {
            let t = types
                .get(&type_application.type_id)
                .expect("type not found");

            let ident = ident(&extract_custom_type_name_from_abi_property(
                t,
                Some(CustomType::Enum),
                types,
            )?);
            Ok(quote! { #ident })
        }
        ParamType::Struct(_) => {
            let t = types
                .get(&type_application.type_id)
                .expect("type not found");

            let ident = ident(&extract_custom_type_name_from_abi_property(
                t,
                Some(CustomType::Struct),
                types,
            )?);
            Ok(quote! { #ident })
        }
        // Primitive type
        _ => expand_type(kind),
    }
}

// Regarding string->TokenStream->string, refer to `custom_types_gen` tests for more details.
#[cfg(test)]
mod tests {
    use super::*;
    use fuels_types::ProgramABI;
    use std::str::FromStr;

    #[test]
    fn test_expand_function_simpleabi() -> Result<(), Error> {
        let s = r#"
        {
            "types": [
              {
                "typeId": 6,
                "type": "u64",
                "components": null,
                "typeParameters": null
              },
              {
                "typeId": 8,
                "type": "b256",
                "components": null,
                "typeParameters": null
              },
              {
                "typeId": 6,
                "type": "u64",
                "components": null,
                "typeParameters": null
              },
              {
                "typeId": 8,
                "type": "b256",
                "components": null,
                "typeParameters": null
              },
              {
                "typeId": 10,
                "type": "bool",
                "components": null,
                "typeParameters": null
              },
              {
                "typeId": 12,
                "type": "struct MyStruct1",
                "components": [
                  {
                    "name": "x",
                    "type": 6,
                    "typeArguments": null
                  },
                  {
                    "name": "y",
                    "type": 8,
                    "typeArguments": null
                  }
                ],
                "typeParameters": null
              },
              {
                "typeId": 6,
                "type": "u64",
                "components": null,
                "typeParameters": null
              },
              {
                "typeId": 8,
                "type": "b256",
                "components": null,
                "typeParameters": null
              },
              {
                "typeId": 2,
                "type": "struct MyStruct1",
                "components": [
                  {
                    "name": "x",
                    "type": 6,
                    "typeArguments": null
                  },
                  {
                    "name": "y",
                    "type": 8,
                    "typeArguments": null
                  }
                ],
                "typeParameters": null
              },
              {
                "typeId": 3,
                "type": "struct MyStruct2",
                "components": [
                  {
                    "name": "x",
                    "type": 10,
                    "typeArguments": null
                  },
                  {
                    "name": "y",
                    "type": 12,
                    "typeArguments": []
                  }
                ],
                "typeParameters": null
              },
              {
                "typeId": 26,
                "type": "struct MyStruct1",
                "components": [
                  {
                    "name": "x",
                    "type": 6,
                    "typeArguments": null
                  },
                  {
                    "name": "y",
                    "type": 8,
                    "typeArguments": null
                  }
                ],
                "typeParameters": null
              }
            ],
            "functions": [
              {
                "type": "function",
                "inputs": [
                  {
                    "name": "s1",
                    "type": 2,
                    "typeArguments": []
                  },
                  {
                    "name": "s2",
                    "type": 3,
                    "typeArguments": []
                  }
                ],
                "name": "some_abi_funct",
                "output": {
                  "name": "",
                  "type": 26,
                  "typeArguments": []
                }
              }
            ]
          }
"#;
        let parsed_abi: ProgramABI = serde_json::from_str(s)?;
        let all_types = parsed_abi
            .types
            .into_iter()
            .map(|t| (t.type_id, t))
            .collect::<HashMap<usize, TypeDeclaration>>();

        // Grabbing the one and only function in it.
        let result = expand_function(&parsed_abi.functions[0], &all_types);

        // let result = expand_function(&the_function, &Default::default(), &Default::default());
        let expected = TokenStream::from_str(
            r#"
            #[doc = "Calls the contract's `some_abi_funct` (0x00000000652399f3) function"]
            pub fn some_abi_funct(&self, s_1: MyStruct1, s_2: MyStruct2) -> ContractCallHandler<MyStruct1> {
                Contract::method_hash(
                    &self.wallet.get_provider().expect("Provider not set up"),
                    self.contract_id.clone(),
                    &self.wallet,
                    [0, 0, 0, 0, 101 , 35 , 153 , 243],
                    Some(ParamType::Struct(vec![ParamType::U64, ParamType::B256])),
                    &[s_1.into_token(), s_2.into_token(),]
                )
                .expect("method not found (this should never happen)")
            }

            "#,
        );
        let expected = expected?.to_string();

        assert_eq!(result?.to_string(), expected);
        Ok(())
    }

    // TODO: Move tests using the old abigen to the new one.
    // Currently, they will be skipped. Even though we're not fully testing these at
    // unit level, they're tested at integration level, in the main harness.rs file.

    // #[test]
    // fn test_expand_function_simple() -> Result<(), Error> {
    //     let mut the_function = Function {
    //         type_field: "unused".to_string(),
    //         inputs: vec![],
    //         name: "HelloWorld".to_string(),
    //         outputs: vec![],
    //     };
    //     the_function.inputs.push(Property {
    //         name: String::from("bimbam"),
    //         type_field: String::from("bool"),
    //         components: None,
    //     });
    //     let result = expand_function(&the_function, &Default::default(), &Default::default());
    //     let expected = TokenStream::from_str(
    //         r#"
    //         #[doc = "Calls the contract's `HelloWorld` (0x0000000097d4de45) function"]
    //         pub fn HelloWorld(&self, bimbam: bool) -> ContractCallHandler<()> {
    //             Contract::method_hash(
    //                 &self.wallet.get_provider().expect("Provider not set up"),
    //                 self.contract_id.clone(),
    //                 &self.wallet,
    //                 [0, 0, 0, 0, 151, 212, 222, 69],
    //                 None,
    //                 &[bimbam.into_token() ,]
    //             )
    //             .expect("method not found (this should never happen)")
    //         }
    //         "#,
    //     );
    //     let expected = expected?.to_string();

    //     assert_eq!(result?.to_string(), expected);
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_function_complex() -> Result<(), Error> {
    //     let mut the_function = Function {
    //         type_field: "function".to_string(),
    //         name: "hello_world".to_string(),
    //         inputs: vec![],
    //         outputs: vec![Property {
    //             name: String::from("stillnotused"),
    //             type_field: String::from("enum EntropyCirclesEnum"),
    //             components: Some(vec![
    //                 Property {
    //                     name: String::from("Postcard"),
    //                     type_field: String::from("bool"),
    //                     components: None,
    //                 },
    //                 Property {
    //                     name: String::from("Teacup"),
    //                     type_field: String::from("u64"),
    //                     components: None,
    //                 },
    //             ]),
    //         }],
    //     };
    //     the_function.inputs.push(Property {
    //         name: String::from("the_only_allowed_input"),
    //         type_field: String::from("struct BurgundyBeefStruct"),
    //         components: Some(vec![
    //             Property {
    //                 name: String::from("Beef"),
    //                 type_field: String::from("bool"),
    //                 components: None,
    //             },
    //             Property {
    //                 name: String::from("BurgundyWine"),
    //                 type_field: String::from("u64"),
    //                 components: None,
    //             },
    //         ]),
    //     });
    //     let mut custom_structs = HashMap::new();
    //     custom_structs.insert(
    //         "BurgundyBeefStruct".to_string(),
    //         Property {
    //             name: "unused".to_string(),
    //             type_field: "struct SomeWeirdFrenchCuisine".to_string(),
    //             components: None,
    //         },
    //     );
    //     custom_structs.insert(
    //         "CoolIndieGame".to_string(),
    //         Property {
    //             name: "unused".to_string(),
    //             type_field: "struct CoolIndieGame".to_string(),
    //             components: None,
    //         },
    //     );
    //     let mut custom_enums = HashMap::new();
    //     custom_enums.insert(
    //         "EntropyCirclesEnum".to_string(),
    //         Property {
    //             name: "unused".to_string(),
    //             type_field: "enum EntropyCirclesEnum".to_string(),
    //             components: None,
    //         },
    //     );
    //     let result = expand_function(&the_function, &custom_enums, &custom_structs);
    //     // Some more editing was required because it is not rustfmt-compatible (adding/removing parentheses or commas)
    //     let expected = TokenStream::from_str(
    //         r#"
    //         #[doc = "Calls the contract's `hello_world` (0x0000000076b25a24) function"]
    //         pub fn hello_world(
    //             &self,
    //             the_only_allowed_input: SomeWeirdFrenchCuisine
    //         ) -> ContractCallHandler<EntropyCirclesEnum> {
    //             Contract::method_hash(
    //                 &self.wallet.get_provider().expect("Provider not set up"),
    //                 self.contract_id.clone(),
    //                 &self.wallet,
    //                 [0, 0, 0, 0, 118, 178, 90, 36],
    //                 Some(ParamType::Enum(EnumVariants::new(vec![ParamType::Bool, ParamType::U64]).unwrap())),
    //                 &[the_only_allowed_input.into_token() ,]
    //             )
    //             .expect("method not found (this should never happen)")
    //         }
    //         "#,
    //     );
    //     let expected = expected?.to_string();

    //     assert_eq!(result?.to_string(), expected);
    //     Ok(())
    // }

    // --- expand_selector ---
    #[test]
    fn test_expand_selector() {
        let result = expand_selector(Selector::default());
        assert_eq!(result.to_string(), "[0 , 0 , 0 , 0 , 0 , 0 , 0 , 0]");

        let result = expand_selector([1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(result.to_string(), "[1 , 2 , 3 , 4 , 5 , 6 , 7 , 8]");
    }

    // --- expand_fn_outputs ---
    // #[test]
    // fn test_expand_fn_outputs() -> Result<(), Error> {
    //     let result = expand_fn_outputs(&[]);
    //     assert_eq!(result?.to_string(), "()");

    //     // Primitive type
    //     let result = expand_fn_outputs(&[Property {
    //         name: "unused".to_string(),
    //         type_field: "bool".to_string(),
    //         components: None,
    //     }]);
    //     assert_eq!(result?.to_string(), "bool");

    //     // Struct type
    //     let result = expand_fn_outputs(&[Property {
    //         name: "unused".to_string(),
    //         type_field: String::from("struct streaming_services"),
    //         components: Some(vec![
    //             Property {
    //                 name: String::from("unused"),
    //                 type_field: String::from("thistypedoesntexist"),
    //                 components: None,
    //             },
    //             Property {
    //                 name: String::from("unused"),
    //                 type_field: String::from("thistypedoesntexist"),
    //                 components: None,
    //             },
    //         ]),
    //     }]);
    //     assert_eq!(result?.to_string(), "streaming_services");

    //     // Enum type
    //     let result = expand_fn_outputs(&[Property {
    //         name: "unused".to_string(),
    //         type_field: String::from("enum StreamingServices"),
    //         components: Some(vec![
    //             Property {
    //                 name: String::from("unused"),
    //                 type_field: String::from("bool"),
    //                 components: None,
    //             },
    //             Property {
    //                 name: String::from("unused"),
    //                 type_field: String::from("u64"),
    //                 components: None,
    //             },
    //         ]),
    //     }]);
    //     assert_eq!(result?.to_string(), "StreamingServices");
    //     Ok(())
    // }

    // // --- expand_function_argument ---
    // #[test]
    // fn test_expand_function_arguments() -> Result<(), Error> {
    //     let hm: HashMap<String, Property> = HashMap::new();
    //     let the_argument = Property {
    //         name: "some_argument".to_string(),
    //         type_field: String::from("u32"),
    //         components: None,
    //     };

    //     // All arguments are here
    //     let mut the_function = Function {
    //         type_field: "".to_string(),
    //         inputs: vec![],
    //         name: "".to_string(),
    //         outputs: vec![],
    //     };
    //     the_function.inputs.push(the_argument);

    //     let result = expand_function_arguments(&the_function, &hm, &hm);
    //     let (args, call_args) = result?;
    //     let result = format!("({},{})", args, call_args);
    //     let expected = "(, some_argument : u32,& [some_argument . into_token () ,])";

    //     assert_eq!(result, expected);
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_function_arguments_primitive() -> Result<(), Error> {
    //     let hm: HashMap<String, Property> = HashMap::new();
    //     let mut the_function = Function {
    //         type_field: "function".to_string(),
    //         inputs: vec![],
    //         name: "pip_pop".to_string(),
    //         outputs: vec![],
    //     };

    //     the_function.inputs.push(Property {
    //         name: "bim_bam".to_string(),
    //         type_field: String::from("u64"),
    //         components: None,
    //     });
    //     let result = expand_function_arguments(&the_function, &hm, &hm);
    //     let (args, call_args) = result?;
    //     let result = format!("({},{})", args, call_args);

    //     assert_eq!(result, "(, bim_bam : u64,& [bim_bam . into_token () ,])");
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_function_arguments_composite() -> Result<(), Error> {
    //     let mut function = Function {
    //         type_field: "zig_zag".to_string(),
    //         inputs: vec![],
    //         name: "PipPopFunction".to_string(),
    //         outputs: vec![],
    //     };
    //     function.inputs.push(Property {
    //         name: "bim_bam".to_string(),
    //         type_field: String::from("struct CarMaker"),
    //         components: Some(vec![Property {
    //             name: "name".to_string(),
    //             type_field: "str[5]".to_string(),
    //             components: None,
    //         }]),
    //     });
    //     let mut custom_structs = HashMap::new();
    //     custom_structs.insert(
    //         "CarMaker".to_string(),
    //         Property {
    //             name: "unused".to_string(),
    //             type_field: "struct CarMaker".to_string(),
    //             components: None,
    //         },
    //     );
    //     let mut custom_enums = HashMap::new();
    //     custom_enums.insert(
    //         "Cocktail".to_string(),
    //         Property {
    //             name: "Cocktail".to_string(),
    //             type_field: "enum Cocktail".to_string(),
    //             components: Some(vec![Property {
    //                 name: "variant".to_string(),
    //                 type_field: "u32".to_string(),
    //                 components: None,
    //             }]),
    //         },
    //     );

    //     let result = expand_function_arguments(&function, &custom_enums, &custom_structs);
    //     let (args, call_args) = result?;
    //     let result = format!("({},{})", args, call_args);
    //     let expected = r#"(, bim_bam : CarMaker,& [bim_bam . into_token () ,])"#;
    //     assert_eq!(result, expected);

    //     function.inputs[0].type_field = "enum Cocktail".to_string();
    //     let result = expand_function_arguments(&function, &custom_enums, &custom_structs);
    //     let (args, call_args) = result?;
    //     let result = format!("({},{})", args, call_args);
    //     let expected = r#"(, bim_bam : Cocktail,& [bim_bam . into_token () ,])"#;
    //     assert_eq!(result, expected);
    //     Ok(())
    // }

    #[test]
    fn transform_name_to_snake_case() -> Result<(), Error> {
        let result = expand_input_name("CamelCaseHello");
        assert_eq!(result?.to_string(), "camel_case_hello");
        Ok(())
    }

    #[test]
    fn avoids_collisions_with_keywords() -> Result<(), Error> {
        let result = expand_input_name("if");
        assert_eq!(result?.to_string(), "if_");

        let result = expand_input_name("let");
        assert_eq!(result?.to_string(), "let_");
        Ok(())
    }

    // --- expand_input_param ---
    // #[test]
    // fn test_expand_input_param_primitive() -> Result<(), Error> {
    //     let def = Function::default();
    //     let result = expand_input_param(&def, "unused", &ParamType::Bool, &None);
    //     assert_eq!(result?.to_string(), "bool");

    //     let result = expand_input_param(&def, "unused", &ParamType::U64, &None);
    //     assert_eq!(result?.to_string(), "u64");

    //     let result = expand_input_param(&def, "unused", &ParamType::String(10), &None);
    //     assert_eq!(result?.to_string(), "String");
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_input_param_array() -> Result<(), Error> {
    //     let array_type = ParamType::Array(Box::new(ParamType::U64), 10);
    //     let result = expand_input_param(&Function::default(), "unused", &array_type, &None);
    //     assert_eq!(result?.to_string(), ":: std :: vec :: Vec < u64 >");
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_input_param_custom_type() -> Result<(), Error> {
    //     let def = Function::default();
    //     let struct_type = ParamType::Struct(vec![ParamType::Bool, ParamType::U64]);
    //     let struct_prop = Property {
    //         name: String::from("unused"),
    //         type_field: String::from("struct Babies"),
    //         components: None,
    //     };
    //     let struct_name = Some(&struct_prop);
    //     let result = expand_input_param(&def, "unused", &struct_type, &struct_name);
    //     assert_eq!(result?.to_string(), "Babies");

    //     let enum_type = ParamType::Enum(EnumVariants::new(vec![ParamType::U8, ParamType::U32])?);
    //     let enum_prop = Property {
    //         name: String::from("unused"),
    //         type_field: String::from("enum Babies"),
    //         components: None,
    //     };
    //     let enum_name = Some(&enum_prop);
    //     let result = expand_input_param(&def, "unused", &enum_type, &enum_name);
    //     assert_eq!(result?.to_string(), "Babies");
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_input_param_struct_wrong_name() {
    //     let def = Function::default();
    //     let struct_type = ParamType::Struct(vec![ParamType::Bool, ParamType::U64]);
    //     let struct_prop = Property {
    //         name: String::from("unused"),
    //         type_field: String::from("not_the_right_format"),
    //         components: None,
    //     };
    //     let struct_name = Some(&struct_prop);
    //     let result = expand_input_param(&def, "unused", &struct_type, &struct_name);
    //     assert!(matches!(result, Err(Error::InvalidData(_))));
    // }

    // #[test]
    // fn test_expand_input_param_struct_with_enum_name() {
    //     let def = Function::default();
    //     let struct_type = ParamType::Struct(vec![ParamType::Bool, ParamType::U64]);
    //     let struct_prop = Property {
    //         name: String::from("unused"),
    //         type_field: String::from("enum Butitsastruct"),
    //         components: None,
    //     };
    //     let struct_name = Some(&struct_prop);
    //     let result = expand_input_param(&def, "unused", &struct_type, &struct_name);
    //     assert!(matches!(result, Err(Error::InvalidType(_))));
    // }

    // #[test]
    // fn can_have_b256_mixed_in_tuple_w_custom_types() -> anyhow::Result<()> {
    //     let test_struct_component = Property {
    //         name: "__tuple_element".to_string(),
    //         type_field: "struct TestStruct".to_string(),
    //         components: Some(vec![Property {
    //             name: "value".to_string(),
    //             type_field: "u64".to_string(),
    //             components: None,
    //         }]),
    //     };
    //     let b256_component = Property {
    //         name: "__tuple_element".to_string(),
    //         type_field: "b256".to_string(),
    //         components: None,
    //     };

    //     let property = Property {
    //         name: "".to_string(),
    //         type_field: "(struct TestStruct, b256)".to_string(),
    //         components: Some(vec![test_struct_component, b256_component]),
    //     };

    //     let stream = expand_fn_outputs(slice::from_ref(&property))?;

    //     let actual = stream.to_string();
    //     let expected = "(TestStruct , [u8 ; 32])";

    //     assert_eq!(actual, expected);

    //     Ok(())
    // }

    #[test]
    fn will_not_replace_b256_in_middle_of_word() {
        let result = expand_b256_into_array_form("(b256, Someb256WeirdStructName, b256, b256)");

        assert_eq!(
            result,
            "([u8; 32], Someb256WeirdStructName, [u8; 32], [u8; 32])"
        );
    }
}
