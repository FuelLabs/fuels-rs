use crate::utils::first_four_bytes_of_sha256_hash;
use crate::ByteArray;
use fuels_types::param_types::ParamType;

/// Given a `ABIFunction` will return a ByteArray representing the function
/// selector as specified in the Fuel specs.

pub fn resolve_fn_selector(name: &str, inputs: &[ParamType]) -> ByteArray {
    let fn_args = resolve_args(inputs);

    let fn_signature = format!("{}({})", name, fn_args);
    eprintln!("the signature is: {fn_signature}");

    first_four_bytes_of_sha256_hash(&fn_signature)
}

fn resolve_args(arg: &[ParamType]) -> String {
    arg.iter().map(resolve_arg).collect::<Vec<_>>().join(",")
}

fn resolve_arg(arg: &ParamType) -> String {
    match &arg {
        ParamType::U8 => "u8".to_owned(),
        ParamType::U16 => "u16".to_owned(),
        ParamType::U32 => "u32".to_owned(),
        ParamType::U64 => "u64".to_owned(),
        ParamType::Bool => "bool".to_owned(),
        ParamType::Byte => "byte".to_owned(),
        ParamType::B256 => "b256".to_owned(),
        ParamType::Unit => "()".to_owned(),
        ParamType::String(len) => {
            format!("str[{len}]")
        }
        ParamType::Array(internal_type, len) => {
            let inner = resolve_arg(internal_type);
            format!("a[{inner};{len}]")
        }
        ParamType::Struct(fields, generics) => {
            let gen_params = resolve_args(generics);
            let field_params = resolve_args(fields);
            let gen_params = if !gen_params.is_empty() {
                format!("<{gen_params}>")
            } else {
                gen_params
            };
            format!("s{gen_params}({field_params})")
        }
        ParamType::Enum(fields, generics) => {
            let gen_params = resolve_args(generics);
            let field_params = resolve_args(fields.param_types());
            let gen_params = if !gen_params.is_empty() {
                format!("<{gen_params}>")
            } else {
                gen_params
            };
            format!("e{gen_params}({field_params})")
        }
        ParamType::Tuple(inner) => {
            let inner = resolve_args(inner);
            format!("({inner})")
        }
        ParamType::Generic(_name) => {
            panic!("ParamType::Generic cannot appear in a function selector!")
        }
    }
}

// FIXME TODO
// #[cfg(test)]
// mod tests {
//     use super::*;
//     use fuels_types::{ABIFunction, ProgramABI};
//
//     #[test]
//     fn handles_primitive_types() {
//         let check_selector_for_type = |primitive_type: &str| {
//             let fun = ABIFunction {
//                 inputs: vec![TypeApplication {
//                     name: "arg".to_string(),
//                     type_id: 0,
//                     type_arguments: None,
//                 }],
//                 name: "some_fun".to_string(),
//                 output: Default::default(),
//             };
//
//             let types = [TypeDeclaration {
//                 type_id: 0,
//                 type_field: primitive_type.to_string(),
//                 components: None,
//                 type_parameters: None,
//             }]
//             .map(|decl| (decl.type_id, decl))
//             .into();
//
//             let selector = resolve_fn_selector(&fun, &types);
//
//             assert_eq!(selector, format!("some_fun({})", primitive_type));
//         };
//
//         for primitive_type in [
//             "u8", "u16", "u32", "u64", "bool", "byte", "b256", "()", "str[15]",
//         ] {
//             check_selector_for_type(primitive_type);
//         }
//     }
//
//     #[test]
//     fn handles_arrays() {
//         let fun = ABIFunction {
//             inputs: vec![TypeApplication {
//                 name: "arg".to_string(),
//                 type_id: 0,
//                 type_arguments: None,
//             }],
//             name: "some_fun".to_string(),
//             output: Default::default(),
//         };
//
//         let types = [
//             TypeDeclaration {
//                 type_id: 0,
//                 type_field: "[_; 1]".to_string(),
//                 components: Some(vec![TypeApplication {
//                     name: "__array_element".to_string(),
//                     type_id: 1,
//                     type_arguments: None,
//                 }]),
//                 type_parameters: None,
//             },
//             TypeDeclaration {
//                 type_id: 1,
//                 type_field: "u8".to_string(),
//                 components: None,
//                 type_parameters: None,
//             },
//         ]
//         .map(|decl| (decl.type_id, decl))
//         .into();
//
//         let selector = resolve_fn_selector(&fun, &types);
//
//         assert_eq!(selector, format!("some_fun(a[u8;1])"));
//     }
//
//     #[test]
//     fn handles_tuples() {
//         let fun = ABIFunction {
//             inputs: vec![TypeApplication {
//                 name: "arg".to_string(),
//                 type_id: 0,
//                 type_arguments: None,
//             }],
//             name: "some_fun".to_string(),
//             output: Default::default(),
//         };
//
//         let types = [
//             TypeDeclaration {
//                 type_id: 0,
//                 type_field: "(_, _)".to_string(),
//                 components: Some(vec![
//                     TypeApplication {
//                         name: "__tuple_element".to_string(),
//                         type_id: 1,
//                         type_arguments: None,
//                     },
//                     TypeApplication {
//                         name: "__tuple_element".to_string(),
//                         type_id: 1,
//                         type_arguments: None,
//                     },
//                 ]),
//                 type_parameters: None,
//             },
//             TypeDeclaration {
//                 type_id: 1,
//                 type_field: "u8".to_string(),
//                 components: None,
//                 type_parameters: None,
//             },
//         ]
//         .map(|decl| (decl.type_id, decl))
//         .into();
//
//         let selector = resolve_fn_selector(&fun, &types);
//
//         assert_eq!(selector, format!("some_fun((u8,u8))"));
//     }
//
//     #[test]
//     fn handles_structs() {
//         let fun = ABIFunction {
//             inputs: vec![TypeApplication {
//                 name: "arg".to_string(),
//                 type_id: 2,
//                 type_arguments: Some(vec![TypeApplication {
//                     name: "".to_string(),
//                     type_id: 3,
//                     type_arguments: None,
//                 }]),
//             }],
//             name: "some_fun".to_string(),
//             output: Default::default(),
//         };
//
//         let types = [
//             TypeDeclaration {
//                 type_id: 1,
//                 type_field: "generic T".to_string(),
//                 components: None,
//                 type_parameters: None,
//             },
//             TypeDeclaration {
//                 type_id: 2,
//                 type_field: "struct SomeStruct".to_string(),
//                 components: Some(vec![
//                     TypeApplication {
//                         name: "a".to_string(),
//                         type_id: 4,
//                         type_arguments: None,
//                     },
//                     TypeApplication {
//                         name: "b".to_string(),
//                         type_id: 1,
//                         type_arguments: None,
//                     },
//                 ]),
//                 type_parameters: Some(vec![1]),
//             },
//             TypeDeclaration {
//                 type_id: 3,
//                 type_field: "u32".to_string(),
//                 components: None,
//                 type_parameters: None,
//             },
//             TypeDeclaration {
//                 type_id: 4,
//                 type_field: "u64".to_string(),
//                 components: None,
//                 type_parameters: None,
//             },
//         ]
//         .map(|decl| (decl.type_id, decl))
//         .into();
//
//         let selector = resolve_fn_selector(&fun, &types);
//
//         assert_eq!(selector, format!("some_fun(s<u32>(u64,u32))"));
//     }
//
//     #[test]
//     fn handles_enums() {
//         let fun = ABIFunction {
//             inputs: vec![TypeApplication {
//                 name: "arg".to_string(),
//                 type_id: 2,
//                 type_arguments: Some(vec![TypeApplication {
//                     name: "".to_string(),
//                     type_id: 3,
//                     type_arguments: None,
//                 }]),
//             }],
//             name: "some_fun".to_string(),
//             output: Default::default(),
//         };
//
//         let types = [
//             TypeDeclaration {
//                 type_id: 1,
//                 type_field: "generic T".to_string(),
//                 components: None,
//                 type_parameters: None,
//             },
//             TypeDeclaration {
//                 type_id: 2,
//                 type_field: "enum SomeEnum".to_string(),
//                 components: Some(vec![
//                     TypeApplication {
//                         name: "a".to_string(),
//                         type_id: 4,
//                         type_arguments: None,
//                     },
//                     TypeApplication {
//                         name: "b".to_string(),
//                         type_id: 1,
//                         type_arguments: None,
//                     },
//                 ]),
//                 type_parameters: Some(vec![1]),
//             },
//             TypeDeclaration {
//                 type_id: 3,
//                 type_field: "u32".to_string(),
//                 components: None,
//                 type_parameters: None,
//             },
//             TypeDeclaration {
//                 type_id: 4,
//                 type_field: "u64".to_string(),
//                 components: None,
//                 type_parameters: None,
//             },
//         ]
//         .map(|decl| (decl.type_id, decl))
//         .into();
//
//         let selector = resolve_fn_selector(&fun, &types);
//
//         assert_eq!(selector, format!("some_fun(e<u32>(u64,u32))"));
//     }
//
//     #[test]
//     fn ultimate_test() {
//         let abi = ProgramABI {
//             types: vec![
//                 TypeDeclaration {
//                     type_id: 0,
//                     type_field: "()".to_string(),
//                     components: Some(vec![]),
//                     type_parameters: None,
//                 },
//                 TypeDeclaration {
//                     type_id: 1,
//                     type_field: "(_, _)".to_string(),
//                     components: Some(vec![
//                         TypeApplication {
//                             name: "__tuple_element".to_string(),
//                             type_id: 11,
//                             type_arguments: None,
//                         },
//                         TypeApplication {
//                             name: "__tuple_element".to_string(),
//                             type_id: 11,
//                             type_arguments: None,
//                         },
//                     ]),
//                     type_parameters: None,
//                 },
//                 TypeDeclaration {
//                     type_id: 2,
//                     type_field: "(_, _)".to_string(),
//                     components: Some(vec![
//                         TypeApplication {
//                             name: "__tuple_element".to_string(),
//                             type_id: 5,
//                             type_arguments: None,
//                         },
//                         TypeApplication {
//                             name: "__tuple_element".to_string(),
//                             type_id: 13,
//                             type_arguments: None,
//                         },
//                     ]),
//                     type_parameters: None,
//                 },
//                 TypeDeclaration {
//                     type_id: 3,
//                     type_field: "(_, _)".to_string(),
//                     components: Some(vec![
//                         TypeApplication {
//                             name: "__tuple_element".to_string(),
//                             type_id: 4,
//                             type_arguments: None,
//                         },
//                         TypeApplication {
//                             name: "__tuple_element".to_string(),
//                             type_id: 21,
//                             type_arguments: None,
//                         },
//                     ]),
//                     type_parameters: None,
//                 },
//                 TypeDeclaration {
//                     type_id: 4,
//                     type_field: "[_; 1]".to_string(),
//                     components: Some(vec![TypeApplication {
//                         name: "__array_element".to_string(),
//                         type_id: 8,
//                         type_arguments: Some(vec![TypeApplication {
//                             name: "".to_string(),
//                             type_id: 20,
//                             type_arguments: Some(vec![TypeApplication {
//                                 name: "".to_string(),
//                                 type_id: 19,
//                                 type_arguments: Some(vec![TypeApplication {
//                                     name: "".to_string(),
//                                     type_id: 17,
//                                     type_arguments: Some(vec![TypeApplication {
//                                         name: "".to_string(),
//                                         type_id: 13,
//                                         type_arguments: None,
//                                     }]),
//                                 }]),
//                             }]),
//                         }]),
//                     }]),
//                     type_parameters: None,
//                 },
//                 TypeDeclaration {
//                     type_id: 5,
//                     type_field: "[_; 2]".to_string(),
//                     components: Some(vec![TypeApplication {
//                         name: "__array_element".to_string(),
//                         type_id: 14,
//                         type_arguments: None,
//                     }]),
//                     type_parameters: None,
//                 },
//                 TypeDeclaration {
//                     type_id: 6,
//                     type_field: "[_; 2]".to_string(),
//                     components: Some(vec![TypeApplication {
//                         name: "__array_element".to_string(),
//                         type_id: 10,
//                         type_arguments: None,
//                     }]),
//                     type_parameters: None,
//                 },
//                 TypeDeclaration {
//                     type_id: 7,
//                     type_field: "b256".to_string(),
//                     components: None,
//                     type_parameters: None,
//                 },
//                 TypeDeclaration {
//                     type_id: 8,
//                     type_field: "enum EnumWGeneric".to_string(),
//                     components: Some(vec![
//                         TypeApplication {
//                             name: "a".to_string(),
//                             type_id: 22,
//                             type_arguments: None,
//                         },
//                         TypeApplication {
//                             name: "b".to_string(),
//                             type_id: 12,
//                             type_arguments: None,
//                         },
//                     ]),
//                     type_parameters: Some(vec![12]),
//                 },
//                 TypeDeclaration {
//                     type_id: 9,
//                     type_field: "generic K".to_string(),
//                     components: None,
//                     type_parameters: None,
//                 },
//                 TypeDeclaration {
//                     type_id: 10,
//                     type_field: "generic L".to_string(),
//                     components: None,
//                     type_parameters: None,
//                 },
//                 TypeDeclaration {
//                     type_id: 11,
//                     type_field: "generic M".to_string(),
//                     components: None,
//                     type_parameters: None,
//                 },
//                 TypeDeclaration {
//                     type_id: 12,
//                     type_field: "generic N".to_string(),
//                     components: None,
//                     type_parameters: None,
//                 },
//                 TypeDeclaration {
//                     type_id: 13,
//                     type_field: "generic T".to_string(),
//                     components: None,
//                     type_parameters: None,
//                 },
//                 TypeDeclaration {
//                     type_id: 14,
//                     type_field: "generic U".to_string(),
//                     components: None,
//                     type_parameters: None,
//                 },
//                 TypeDeclaration {
//                     type_id: 15,
//                     type_field: "str[2]".to_string(),
//                     components: None,
//                     type_parameters: None,
//                 },
//                 TypeDeclaration {
//                     type_id: 16,
//                     type_field: "struct MegaExample".to_string(),
//                     components: Some(vec![
//                         TypeApplication {
//                             name: "a".to_string(),
//                             type_id: 2,
//                             type_arguments: None,
//                         },
//                         TypeApplication {
//                             name: "b".to_string(),
//                             type_id: 3,
//                             type_arguments: None,
//                         },
//                     ]),
//                     type_parameters: Some(vec![13, 14]),
//                 },
//                 TypeDeclaration {
//                     type_id: 17,
//                     type_field: "struct PassTheGenericOn".to_string(),
//                     components: Some(vec![TypeApplication {
//                         name: "one".to_string(),
//                         type_id: 18,
//                         type_arguments: Some(vec![TypeApplication {
//                             name: "".to_string(),
//                             type_id: 9,
//                             type_arguments: None,
//                         }]),
//                     }]),
//                     type_parameters: Some(vec![9]),
//                 },
//                 TypeDeclaration {
//                     type_id: 18,
//                     type_field: "struct SimpleGeneric".to_string(),
//                     components: Some(vec![TypeApplication {
//                         name: "single_generic_param".to_string(),
//                         type_id: 13,
//                         type_arguments: None,
//                     }]),
//                     type_parameters: Some(vec![13]),
//                 },
//                 TypeDeclaration {
//                     type_id: 19,
//                     type_field: "struct StructWArrayGeneric".to_string(),
//                     components: Some(vec![TypeApplication {
//                         name: "a".to_string(),
//                         type_id: 6,
//                         type_arguments: None,
//                     }]),
//                     type_parameters: Some(vec![10]),
//                 },
//                 TypeDeclaration {
//                     type_id: 20,
//                     type_field: "struct StructWTupleGeneric".to_string(),
//                     components: Some(vec![TypeApplication {
//                         name: "a".to_string(),
//                         type_id: 1,
//                         type_arguments: None,
//                     }]),
//                     type_parameters: Some(vec![11]),
//                 },
//                 TypeDeclaration {
//                     type_id: 21,
//                     type_field: "u32".to_string(),
//                     components: None,
//                     type_parameters: None,
//                 },
//                 TypeDeclaration {
//                     type_id: 22,
//                     type_field: "u64".to_string(),
//                     components: None,
//                     type_parameters: None,
//                 },
//             ],
//             functions: vec![ABIFunction {
//                 inputs: vec![TypeApplication {
//                     name: "arg1".to_string(),
//                     type_id: 16,
//                     type_arguments: Some(vec![
//                         TypeApplication {
//                             name: "".to_string(),
//                             type_id: 15,
//                             type_arguments: None,
//                         },
//                         TypeApplication {
//                             name: "".to_string(),
//                             type_id: 7,
//                             type_arguments: None,
//                         },
//                     ]),
//                 }],
//                 name: "complex_test".to_string(),
//                 output: TypeApplication {
//                     name: "".to_string(),
//                     type_id: 0,
//                     type_arguments: None,
//                 },
//             }],
//         };
//
//         let the_fun = abi.functions.first().unwrap();
//         let types = abi
//             .types
//             .into_iter()
//             .map(|decl| (decl.type_id, decl))
//             .collect();
//
//         let selector = resolve_fn_selector(the_fun, &types);
//
//         assert_eq!(selector, "complex_test(s<str[2],b256>((a[b256;2],str[2]),(a[e<s<s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2])>((s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2]),s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2])))>(u64,s<s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2])>((s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2]),s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2]))));1],u32)))");
//     }
// }
