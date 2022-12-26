use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::quote;

use fuels_types::errors::Error;
use resolved_type::resolve_type;

use crate::code_gen::abi_types::{FullABIFunction, FullTypeApplication, FullTypeDeclaration};
use crate::code_gen::utils::{param_type_calls, Component};
use crate::code_gen::{resolved_type, resolved_type::ResolvedType};
use crate::utils::safe_ident;

#[derive(Debug)]
pub(crate) struct FunctionGenerator {
    name: String,
    args: Vec<Component>,
    output_type: TokenStream,
    body: TokenStream,
    doc: Option<String>,
}

impl FunctionGenerator {
    pub fn new(
        fun: &FullABIFunction,
        shared_types: &HashSet<FullTypeDeclaration>,
    ) -> Result<Self, Error> {
        let args = function_arguments(fun.inputs(), shared_types)?;

        let output_type = resolve_fn_output_type(fun, shared_types)?;

        Ok(Self {
            name: fun.name().to_string(),
            args,
            output_type: output_type.into(),
            body: Default::default(),
            doc: None,
        })
    }

    pub fn set_body(&mut self, body: TokenStream) -> &mut Self {
        self.body = body;
        self
    }

    pub fn set_doc(&mut self, text: String) -> &mut Self {
        self.doc = Some(text);
        self
    }

    pub fn fn_selector(&self) -> TokenStream {
        let param_type_calls = param_type_calls(&self.args);

        let name = &self.name;
        quote! {::fuels::core::code_gen::function_selector::resolve_fn_selector(#name, &[#(#param_type_calls),*])}
    }

    pub fn tokenized_args(&self) -> TokenStream {
        let arg_names = self.args.iter().map(|component| &component.field_name);
        quote! {[#(::fuels::core::Tokenizable::into_token(#arg_names)),*]}
    }

    pub fn set_output_type(&mut self, output_type: TokenStream) -> &mut Self {
        self.output_type = output_type;
        self
    }

    pub fn output_type(&self) -> &TokenStream {
        &self.output_type
    }
}

impl From<&FunctionGenerator> for TokenStream {
    fn from(fun: &FunctionGenerator) -> Self {
        let name = safe_ident(&fun.name);
        let doc = fun
            .doc
            .as_ref()
            .map(|text| {
                quote! { #[doc = #text] }
            })
            .unwrap_or_default();

        let arg_declarations = fun.args.iter().map(|component| {
            let name = &component.field_name;
            let field_type: TokenStream = (&component.field_type).into();
            quote! { #name: #field_type }
        });

        let output_type = fun.output_type();
        let body = &fun.body;

        quote! {
            #doc
            pub fn #name(&self #(,#arg_declarations)*) -> #output_type {
                #body
            }
        }
    }
}

impl From<FunctionGenerator> for TokenStream {
    fn from(fun: FunctionGenerator) -> Self {
        (&fun).into()
    }
}

fn resolve_fn_output_type(
    function: &FullABIFunction,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<ResolvedType, Error> {
    let output_type = resolve_type(function.output(), shared_types)?;
    if output_type.uses_vectors() {
        Err(Error::CompilationError(format!(
            "function '{}' contains a vector in its return type. This currently isn't supported.",
            function.name()
        )))
    } else {
        Ok(output_type)
    }
}

fn function_arguments(
    inputs: &[FullTypeApplication],
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<Vec<Component>, Error> {
    inputs
        .iter()
        .map(|input| Component::new(input, true, shared_types))
        .collect::<Result<Vec<_>, Error>>()
        .map_err(|e| Error::InvalidType(e.to_string()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use fuels_types::{ABIFunction, TypeApplication, TypeDeclaration};

    use super::*;

    #[test]
    fn test_expand_fn_arguments() -> Result<(), Error> {
        let the_argument = TypeApplication {
            name: "some_argument".to_string(),
            type_id: 0,
            ..Default::default()
        };

        // All arguments are here
        let the_function = ABIFunction {
            inputs: vec![the_argument],
            ..ABIFunction::default()
        };

        let types = [(
            0,
            TypeDeclaration {
                type_id: 0,
                type_field: String::from("u32"),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let result = function_arguments(
            FullABIFunction::from_counterpart(&the_function, &types)?.inputs(),
            &HashSet::default(),
        )?;
        let component = &result[0];

        assert_eq!(&component.field_name.to_string(), "some_argument");
        assert_eq!(&component.field_type.to_string(), "u32");

        Ok(())
    }

    #[test]
    fn test_expand_fn_arguments_primitive() -> Result<(), Error> {
        let the_function = ABIFunction {
            inputs: vec![TypeApplication {
                name: "bim_bam".to_string(),
                type_id: 1,
                ..Default::default()
            }],
            name: "pip_pop".to_string(),
            ..Default::default()
        };

        let types = [
            (
                0,
                TypeDeclaration {
                    type_id: 0,
                    type_field: String::from("()"),
                    ..Default::default()
                },
            ),
            (
                1,
                TypeDeclaration {
                    type_id: 1,
                    type_field: String::from("u64"),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let result = function_arguments(
            FullABIFunction::from_counterpart(&the_function, &types)?.inputs(),
            &HashSet::default(),
        )?;
        let component = &result[0];

        assert_eq!(&component.field_name.to_string(), "bim_bam");
        assert_eq!(&component.field_type.to_string(), "u64");

        Ok(())
    }

    #[test]
    fn test_expand_fn_arguments_composite() -> Result<(), Error> {
        let mut function = ABIFunction {
            inputs: vec![TypeApplication {
                name: "bim_bam".to_string(),
                type_id: 0,
                ..Default::default()
            }],
            name: "PipPopFunction".to_string(),
            ..Default::default()
        };

        let types = [
            (
                0,
                TypeDeclaration {
                    type_id: 0,
                    type_field: "struct CarMaker".to_string(),
                    components: Some(vec![TypeApplication {
                        name: "name".to_string(),
                        type_id: 1,
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
            ),
            (
                1,
                TypeDeclaration {
                    type_id: 1,
                    type_field: "str[5]".to_string(),
                    ..Default::default()
                },
            ),
            (
                2,
                TypeDeclaration {
                    type_id: 2,
                    type_field: "enum Cocktail".to_string(),
                    components: Some(vec![TypeApplication {
                        name: "variant".to_string(),
                        type_id: 3,
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
            ),
            (
                3,
                TypeDeclaration {
                    type_id: 3,
                    type_field: "u32".to_string(),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let result = function_arguments(
            FullABIFunction::from_counterpart(&function, &types)?.inputs(),
            &HashSet::default(),
        )?;

        assert_eq!(&result[0].field_name.to_string(), "bim_bam");
        assert_eq!(&result[0].field_type.to_string(), "self :: CarMaker");

        function.inputs[0].type_id = 2;
        let result = function_arguments(
            FullABIFunction::from_counterpart(&function, &types)?.inputs(),
            &HashSet::default(),
        )?;

        assert_eq!(&result[0].field_name.to_string(), "bim_bam");
        assert_eq!(&result[0].field_type.to_string(), "self :: Cocktail");

        Ok(())
    }
}
