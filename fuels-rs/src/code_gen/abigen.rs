use std::collections::HashMap;

use crate::code_gen::bindings::ContractBindings;
use crate::code_gen::custom_types_gen::{expand_internal_enum, expand_internal_struct};
use crate::code_gen::functions_gen::expand_function;
use crate::errors::Error;
use crate::json_abi::ABIParser;
use crate::source::Source;
use crate::utils::ident;
use core_types::{JsonABI, Property};

use proc_macro2::{Ident, TokenStream};
use quote::quote;

pub struct Abigen {
    /// The parsed ABI.
    abi: JsonABI,

    /// The parser used to transform the JSON format into `JsonABI`
    abi_parser: ABIParser,

    /// The contract name as an identifier.
    contract_name: Ident,

    custom_structs: HashMap<String, Property>,

    custom_enums: HashMap<String, Property>,

    /// Format the code using a locally installed copy of `rustfmt`.
    rustfmt: bool,

    /// Generate no-std safe code
    no_std: bool,
}

enum CustomType {
    Enum,
    Struct,
}

impl Abigen {
    /// Creates a new contract with the given ABI JSON source.
    pub fn new<S: AsRef<str>>(contract_name: &str, abi_source: S) -> Result<Self, Error> {
        let source = Source::parse(abi_source).unwrap();
        let parsed_abi: JsonABI = serde_json::from_str(&source.get().unwrap())?;

        Ok(Self {
            custom_structs: Abigen::get_custom_types(&parsed_abi, &CustomType::Struct),
            custom_enums: Abigen::get_custom_types(&parsed_abi, &CustomType::Enum),
            abi: parsed_abi,
            contract_name: ident(contract_name),
            abi_parser: ABIParser::new(),
            rustfmt: true,
            no_std: false,
        })
    }

    pub fn no_std(mut self) -> Self {
        self.no_std = true;
        self
    }

    /// Generates the contract bindings.
    pub fn generate(self) -> Result<ContractBindings, Error> {
        let rustfmt = self.rustfmt;
        let tokens = self.expand()?;

        Ok(ContractBindings { tokens, rustfmt })
    }

    /// Entry point of the Abigen's expansion logic.
    /// The high-level goal of this function is to expand[0] a contract
    /// defined as a JSON into type-safe bindings of that contract that can be
    /// used after it is brought into scope after a successful generation.
    ///
    /// [0]: To expand, in procedural macro terms, means to automatically generate
    /// Rust code after a transformation of `TokenStream` to another
    /// set of `TokenStream`. This generated Rust code is the brought into scope
    /// after it is called through a procedural macro (`abigen!()` in our case).
    pub fn expand(&self) -> Result<TokenStream, Error> {
        let name = &self.contract_name;
        let name_mod = ident(&format!(
            "{}_mod",
            self.contract_name.to_string().to_lowercase()
        ));

        let contract_functions = self.functions()?;
        let abi_structs = self.abi_structs()?;
        let abi_enums = self.abi_enums()?;

        let (includes, code) = if self.no_std {
            (
                quote! {
                    use alloc::vec::Vec;
                },
                quote! {},
            )
        } else {
            (
                quote! {
                    use fuels_rs::contract::{Contract, ContractCall, CompiledContract};
                    use fuel_client::client::FuelClient;
                },
                quote! {
                    pub struct #name {
                        compiled: CompiledContract,
                        fuel_client: FuelClient
                    }

                    impl #name {
                        pub fn new(compiled: CompiledContract, fuel_client: FuelClient) -> Self {
                            Self{ compiled, fuel_client }
                        }

                        #contract_functions
                    }
                },
            )
        };

        Ok(quote! {
            pub use #name_mod::*;

            #[allow(clippy::too_many_arguments)]
            mod #name_mod {
                #![allow(clippy::enum_variant_names)]
                #![allow(dead_code)]
                #![allow(unused_imports)]

                #includes
                use fuels_core::{EnumSelector, ParamType, Tokenizable, Token};

                #code

                #abi_structs
                #abi_enums
            }
        })
    }

    pub fn functions(&self) -> Result<TokenStream, Error> {
        let mut tokenized_functions = Vec::new();

        for function in &self.abi {
            let tokenized_fn = expand_function(
                function,
                &self.abi_parser,
                &self.custom_enums,
                &self.custom_structs,
            )?;
            tokenized_functions.push(tokenized_fn);
        }

        Ok(quote! { #( #tokenized_functions )* })
    }

    fn abi_structs(&self) -> Result<TokenStream, Error> {
        let mut structs = TokenStream::new();

        for prop in self.custom_structs.values() {
            structs.extend(expand_internal_struct(prop)?);
        }

        Ok(structs)
    }

    fn abi_enums(&self) -> Result<TokenStream, Error> {
        let mut enums = TokenStream::new();

        for (name, prop) in &self.custom_enums {
            enums.extend(expand_internal_enum(name, prop)?);
        }

        Ok(enums)
    }

    /// Reads the parsed ABI and returns the custom structs found in it.
    fn get_custom_types(abi: &JsonABI, ty: &CustomType) -> HashMap<String, Property> {
        let mut structs = HashMap::new();
        let mut inner_structs: Vec<Property> = Vec::new();

        let type_string = match ty {
            CustomType::Enum => "enum",
            CustomType::Struct => "struct",
        };

        for function in abi {
            for prop in &function.inputs {
                if prop.type_field.contains(type_string) {
                    // Top level struct
                    if !structs.contains_key(&prop.name) {
                        structs.insert(prop.name.clone(), prop.clone());
                    }

                    // Find inner structs in case of nested custom types
                    for inner_component in prop.components.as_ref().unwrap() {
                        inner_structs.extend(Abigen::get_inner_custom_properties(
                            inner_component,
                            type_string,
                        ));
                    }
                }
            }
        }

        for inner_struct in inner_structs {
            if !structs.contains_key(&inner_struct.name) {
                structs.insert(inner_struct.name.clone(), inner_struct);
            }
        }

        structs
    }

    // Recursively gets inner properties defined in nested structs or nested enums
    fn get_inner_custom_properties(prop: &Property, ty: &str) -> Vec<Property> {
        let mut props = Vec::new();

        if prop.type_field.contains(ty) {
            props.push(prop.clone());

            for inner_prop in prop.components.as_ref().unwrap() {
                let inner = Abigen::get_inner_custom_properties(inner_prop, ty);
                props.extend(inner);
            }
        }

        props
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_bindings() {
        let contract = r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"arg",
                        "type":"u32"
                    }
                ],
                "name":"takes_u32_returns_bool",
                "outputs":[
                    {
                        "name":"",
                        "type":"bool"
                    }
                ]
            }
        ]
        "#;

        let bindings = Abigen::new("test", contract).unwrap().generate().unwrap();
        bindings.write(std::io::stdout()).unwrap();
    }

    #[test]
    fn generates_bindings_two_args() {
        let contract = r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"arg",
                        "type":"u32"
                    },
                    {
                        "name":"second_arg",
                        "type":"u16"
                    }
                ],
                "name":"takes_ints_returns_bool",
                "outputs":[
                    {
                        "name":"",
                        "type":"bool"
                    }
                ]
            }
        ]
        "#;

        let bindings = Abigen::new("test", contract).unwrap().generate().unwrap();
        bindings.write(std::io::stdout()).unwrap();
    }

    #[test]
    fn custom_struct() {
        let contract = r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"value",
                        "type":"struct MyStruct",
                        "components": [
                            {
                                "name": "foo",
                                "type": "u8"
                            },
                            {
                                "name": "bar",
                                "type": "bool"
                            }
                        ]
                    }
                ],
                "name":"takes_struct",
                "outputs":[]
            }
        ]
        "#;

        let contract = Abigen::new("custom", contract).unwrap();

        assert_eq!(1, contract.custom_structs.len());

        assert!(contract.custom_structs.contains_key("value"));

        let bindings = contract.generate().unwrap();
        bindings.write(std::io::stdout()).unwrap();
    }

    #[test]
    fn multiple_custom_types() {
        let contract = r#"
        [
            {
                "type":"contract",
                "inputs":[
                {
                    "name":"input",
                    "type":"struct MyNestedStruct",
                    "components":[
                    {
                        "name":"x",
                        "type":"u16"
                    },
                    {
                        "name":"foo",
                        "type":"struct InnerStruct",
                        "components":[
                        {
                            "name":"a",
                            "type":"bool"
                        },
                        {
                            "name":"b",
                            "type":"u8[2]"
                        }
                        ]
                    }
                    ]
                },
                {
                    "name":"y",
                    "type":"struct MySecondNestedStruct",
                    "components":[
                    {
                        "name":"x",
                        "type":"u16"
                    },
                    {
                        "name":"bar",
                        "type":"struct SecondInnerStruct",
                        "components":[
                        {
                            "name":"inner_bar",
                            "type":"struct ThirdInnerStruct",
                            "components":[
                            {
                                "name":"foo",
                                "type":"u8"
                            }
                            ]
                        }
                        ]
                    }
                    ]
                }
                ],
                "name":"takes_nested_struct",
                "outputs":[
                
                ]
            }
        ]
        "#;

        let contract = Abigen::new("custom", contract).unwrap();

        assert_eq!(5, contract.custom_structs.len());

        let expected_custom_struct_names = vec!["input", "foo", "y", "bar", "inner_bar"];

        for name in expected_custom_struct_names {
            assert!(contract.custom_structs.contains_key(name));
        }

        let bindings = contract.generate().unwrap();
        bindings.write(std::io::stdout()).unwrap();
    }

    #[test]
    fn single_nested_struct() {
        let contract = r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"top_value",
                        "type":"struct MyNestedStruct",
                        "components": [
                            {
                                "name": "x",
                                "type": "u16"
                            },
                            {
                                "name": "foo",
                                "type": "struct InnerStruct",
                                "components": [
                                    {
                                        "name":"a",
                                        "type": "bool"
                                    }
                                ]
                            }
                        ]
                    }
                ],
                "name":"takes_nested_struct",
                "outputs":[]
            }
        ]
        "#;

        let contract = Abigen::new("custom", contract).unwrap();

        assert_eq!(2, contract.custom_structs.len());

        assert!(contract.custom_structs.contains_key("top_value"));
        assert!(contract.custom_structs.contains_key("foo"));

        let bindings = contract.generate().unwrap();
        bindings.write(std::io::stdout()).unwrap();
    }

    #[test]
    fn custom_enum() {
        let contract = r#"
        [
            {
                "type":"contract",
                "inputs":[
                    {
                        "name":"my_enum",
                        "type":"enum MyEnum",
                        "components": [
                            {
                                "name": "x",
                                "type": "u32"
                            },
                            {
                                "name": "y",
                                "type": "bool"
                            }
                        ]
                    }
                ],
                "name":"takes_enum",
                "outputs":[]
            }
        ]
        "#;

        let contract = Abigen::new("custom", contract).unwrap();

        assert_eq!(1, contract.custom_enums.len());
        assert_eq!(0, contract.custom_structs.len());

        assert!(contract.custom_enums.contains_key("my_enum"));

        let bindings = contract.generate().unwrap();
        bindings.write(std::io::stdout()).unwrap();
    }
}
