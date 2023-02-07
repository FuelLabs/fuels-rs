use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

use crate::{
    error::{error, Result},
    program_bindings::{
        abi_types::{FullABIFunction, FullTypeApplication, FullTypeDeclaration},
        resolved_type::{resolve_type, ResolvedType},
        utils::{param_type_calls, Component},
    },
    utils::safe_ident,
};

#[derive(Debug)]
pub(crate) struct FunctionGenerator {
    name: String,
    args: Vec<Component>,
    output_type: TokenStream,
    body: TokenStream,
    doc: Option<String>,
}

impl FunctionGenerator {
    pub fn new(fun: &FullABIFunction, shared_types: &HashSet<FullTypeDeclaration>) -> Result<Self> {
        let args = function_arguments(fun.inputs(), shared_types)?;

        let output_type = resolve_fn_output_type(fun, shared_types)?;

        Ok(Self {
            name: fun.name().to_string(),
            args,
            output_type: output_type.to_token_stream(),
            body: Default::default(),
            doc: None,
        })
    }

    pub fn set_name(&mut self, name: String) -> &mut Self {
        self.name = name;
        self
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
        quote! {::fuels::core::function_selector::resolve_fn_selector(#name, &[#(#param_type_calls),*])}
    }

    pub fn tokenized_args(&self) -> TokenStream {
        let arg_names = self.args.iter().map(|component| &component.field_name);
        quote! {[#(::fuels::types::traits::Tokenizable::into_token(#arg_names)),*]}
    }

    pub fn set_output_type(&mut self, output_type: TokenStream) -> &mut Self {
        self.output_type = output_type;
        self
    }

    pub fn output_type(&self) -> &TokenStream {
        &self.output_type
    }
}

fn function_arguments(
    inputs: &[FullTypeApplication],
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<Vec<Component>> {
    inputs
        .iter()
        .map(|input| Component::new(input, true, shared_types))
        .collect::<Result<_>>()
}

fn resolve_fn_output_type(
    function: &FullABIFunction,
    shared_types: &HashSet<FullTypeDeclaration>,
) -> Result<ResolvedType> {
    let output_type = resolve_type(function.output(), shared_types)?;
    if output_type.uses_vectors() {
        Err(error!(
            "function '{}' contains a vector in its return type. This currently isn't supported.",
            function.name()
        ))
    } else {
        Ok(output_type)
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
            let field_type = &component.field_type;
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use fuel_abi_types::program_abi::{ABIFunction, TypeApplication, TypeDeclaration};

    use super::*;

    #[test]
    fn test_expand_fn_arguments() -> Result<()> {
        let the_argument = TypeApplication {
            name: "some_argument".to_string(),
            type_id: 0,
            ..Default::default()
        };

        // All arguments are here
        let the_function = ABIFunction {
            inputs: vec![the_argument],
            name: "some_fun".to_string(),
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
    fn test_expand_fn_arguments_primitive() -> Result<()> {
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
    fn test_expand_fn_arguments_composite() -> Result<()> {
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

    #[test]
    fn correct_output_type() -> Result<()> {
        let function = given_a_fun();
        let sut = FunctionGenerator::new(&function, &HashSet::default())?;

        let output_type = sut.output_type();

        assert_eq!(output_type.to_string(), "self :: CustomStruct < u64 >");

        Ok(())
    }

    #[test]
    fn correct_fn_selector_resolving_code() -> Result<()> {
        let function = given_a_fun();
        let sut = FunctionGenerator::new(&function, &HashSet::default())?;

        let fn_selector_code = sut.fn_selector();

        assert_eq!(
            fn_selector_code.to_string(),
            r#":: fuels :: core :: function_selector :: resolve_fn_selector ("test_function" , & [< self :: CustomStruct :: < u8 > as :: fuels :: types :: traits :: Parameterize > :: param_type ()])"#
        );

        Ok(())
    }

    #[test]
    fn correct_tokenized_args() -> Result<()> {
        let function = given_a_fun();
        let sut = FunctionGenerator::new(&function, &HashSet::default())?;

        let tokenized_args = sut.tokenized_args();

        assert_eq!(
            tokenized_args.to_string(),
            "[:: fuels :: types :: traits :: Tokenizable :: into_token (arg_0)]"
        );

        Ok(())
    }

    #[test]
    fn tokenizes_correctly() -> Result<()> {
        // given
        let function = given_a_fun();
        let mut sut = FunctionGenerator::new(&function, &HashSet::default())?;

        sut.set_doc("This is a doc".to_string())
            .set_body(quote! {this is ze body});

        // when
        let tokenized: TokenStream = sut.into();

        // then
        let expected = quote! {
            #[doc = "This is a doc"]
            pub fn test_function(&self, arg_0: self::CustomStruct<u8>) -> self::CustomStruct<u64> {
                this is ze body
            }
        };

        // then
        assert_eq!(tokenized.to_string(), expected.to_string());

        Ok(())
    }

    fn given_a_fun() -> FullABIFunction {
        let generic_type_t = FullTypeDeclaration {
            type_field: "generic T".to_string(),
            components: vec![],
            type_parameters: vec![],
        };
        let custom_struct_type = FullTypeDeclaration {
            type_field: "struct CustomStruct".to_string(),
            components: vec![FullTypeApplication {
                name: "field_a".to_string(),
                type_decl: generic_type_t.clone(),
                type_arguments: vec![],
            }],
            type_parameters: vec![generic_type_t],
        };

        let fn_output = FullTypeApplication {
            name: "".to_string(),
            type_decl: custom_struct_type.clone(),
            type_arguments: vec![FullTypeApplication {
                name: "".to_string(),
                type_decl: FullTypeDeclaration {
                    type_field: "u64".to_string(),
                    components: vec![],
                    type_parameters: vec![],
                },
                type_arguments: vec![],
            }],
        };
        let fn_inputs = vec![FullTypeApplication {
            name: "arg_0".to_string(),
            type_decl: custom_struct_type,
            type_arguments: vec![FullTypeApplication {
                name: "".to_string(),
                type_decl: FullTypeDeclaration {
                    type_field: "u8".to_string(),
                    components: vec![],
                    type_parameters: vec![],
                },
                type_arguments: vec![],
            }],
        }];

        FullABIFunction::new("test_function".to_string(), fn_inputs, fn_output, vec![])
            .expect("Hand crafted function known to be correct")
    }
}
