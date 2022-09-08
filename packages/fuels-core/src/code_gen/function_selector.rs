use crate::code_gen::resolved_type::{resolve_type, ResolvedType};
use fuels_types::param_types::ParamType;
use fuels_types::{ABIFunction, TypeApplication, TypeDeclaration};
use itertools::Itertools;
use std::collections::HashMap;
use std::iter::zip;
use std::ptr::replace;

#[derive(Debug, Clone)]
struct Something {
    param_type: ParamType,
    original_type: TypeDeclaration,
    generic_params: Vec<Something>,
    components: Vec<Something>,
}

fn replace_generics(
    args: Vec<Something>,
    generics_lookup: &HashMap<usize, Something>,
) -> Vec<Something> {
    args.into_iter()
        .map(|arg| {
            if let ParamType::Generic(name) = arg.param_type {
                (*generics_lookup.get(&arg.original_type.type_id).unwrap()).clone()
            } else {
                let components = replace_generics(arg.components, generics_lookup);
                let generic_params = replace_generics(arg.generic_params, generics_lookup);
                Something {
                    components,
                    generic_params,
                    ..arg
                }
            }
        })
        .collect()
}

fn resolve_type_application(
    type_application: &TypeApplication,
    types: &HashMap<usize, TypeDeclaration>,
) -> Something {
    let type_decl = types.get(&type_application.type_id).unwrap();
    let param_type = ParamType::from_type_declaration(&type_decl, &types).unwrap();

    let generic_params = type_application
        .type_arguments
        .iter()
        .flatten()
        .map(|generic| resolve_type_application(generic, types))
        .collect_vec();

    let components = type_decl
        .components
        .iter()
        .flatten()
        .map(|component| resolve_type_application(component, types))
        .collect_vec();

    let type_ids_of_used_generics = type_decl.type_parameters.clone().unwrap_or_default();
    let generics_lookup =
        zip(type_ids_of_used_generics, generic_params.clone()).collect::<HashMap<_, _>>();
    let components = replace_generics(components, &generics_lookup);

    let original_type = type_decl.clone();
    Something {
        param_type,
        original_type,
        generic_params,
        components,
    }
}

impl Something {
    pub fn to_function_sel_format(&self) -> String {
        let generics = if self.generic_params.is_empty() {
            "".to_string()
        } else {
            let generic_params = self
                .generic_params
                .iter()
                .map(|arg| arg.to_function_sel_format())
                .collect::<Vec<_>>()
                .join(",");

            format!("<{}>", generic_params)
        };

        let components = self
            .components
            .iter()
            .map(|component| component.to_function_sel_format())
            .collect::<Vec<_>>()
            .join(",");

        match &self.param_type {
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
            ParamType::Array(inner_type, len) => {
                format!("a[{components};{len}]")
            }
            ParamType::Struct(_) => {
                format!("s{generics}({components})")
            }
            ParamType::Enum(_) => {
                format!("e{generics}({components})")
            }
            ParamType::Tuple(_) => {
                format!("({components})")
            }
            ParamType::Generic(_name) => {
                panic!("ParamType::Generic cannot appear in a function selector!")
                // format!("generic {_name}")
                //
            }
        }
    }
}

fn resolve_function_arg(arg: &TypeApplication, types: &HashMap<usize, TypeDeclaration>) -> String {
    resolve_type_application(arg, types).to_function_sel_format()
}

pub fn resolve_function_selector(
    function: &ABIFunction,
    types: &HashMap<usize, TypeDeclaration>,
) -> String {
    let fun_args = function
        .inputs
        .iter()
        .map(|input| resolve_function_arg(input, types))
        .collect::<Vec<_>>()
        .join(",");

    format!("{}({})", function.name, fun_args)
}

#[cfg(test)]
mod tests {
    use super::*;
    use fuels_types::{ABIFunction, ProgramABI};
    use std::fs;

    #[test]
    fn simple_case() -> anyhow::Result<()> {
        let program_abi = load_abi("simple")?;
        let fun = program_abi.functions.first().unwrap();

        let types = program_abi
            .types
            .iter()
            .map(|decl| (decl.type_id, decl.clone()))
            .collect::<HashMap<_, _>>();

        let result = resolve_function_selector(&fun, &types);

        assert_eq!(result, "test_function(s<u32>(u64,u32))");

        Ok(())
    }

    #[test]
    fn generic_parameter_reused_later() -> anyhow::Result<()> {
        let program_abi = load_abi("generic_arg_used_again_later")?;
        let fun = program_abi.functions.first().unwrap();

        let types = program_abi
            .types
            .iter()
            .map(|decl| (decl.type_id, decl.clone()))
            .collect::<HashMap<_, _>>();

        let result = resolve_function_selector(&fun, &types);

        let correct_fn_selector = "test_function(s<u8,u32>(u64,u8,u32,u8))";

        assert_eq!(result, correct_fn_selector);

        Ok(())
    }

    #[test]
    fn generic_param_forwarded_to_inner_generic() -> anyhow::Result<()> {
        let program_abi = load_abi("generic_arg_forwarded_to_nested_generic")?;
        let fun = program_abi.functions.first().unwrap();

        let types = program_abi
            .types
            .iter()
            .map(|decl| (decl.type_id, decl.clone()))
            .collect::<HashMap<_, _>>();

        let result = resolve_function_selector(&fun, &types);

        let correct_fn_selector = "test_function(s<u32>(s<u32>(u64,u32)))";

        assert_eq!(result, correct_fn_selector);

        Ok(())
    }

    #[test]
    fn edge_case_regarding_generic_names() -> anyhow::Result<()> {
        let program_abi = load_abi("extracting_generics_by_argument_name_is_faulty")?;
        let fun = program_abi.functions.first().unwrap();

        let types = program_abi
            .types
            .iter()
            .map(|decl| (decl.type_id, decl.clone()))
            .collect::<HashMap<_, _>>();

        let result = resolve_function_selector(&fun, &types);

        let correct_fn_selector = "test_function(s<u64,u32>(u64,s<u32>(u32)))";

        assert_eq!(result, correct_fn_selector);

        Ok(())
    }

    #[test]
    fn ultimate_test() -> anyhow::Result<()> {
        let program_abi = load_abi("generics")?;
        let fun = program_abi.functions.first().unwrap();

        let types = program_abi
            .types
            .iter()
            .map(|decl| (decl.type_id, decl.clone()))
            .collect::<HashMap<_, _>>();

        let result = resolve_function_selector(&fun, &types);

        let correct_fn_selector = "identity(s<u32,s<s(u8,u64)>(u64,str[15],s(u8,u64))>(s<e<u32>(u64,u32)>(u64,str[15],e<u32>(u64,u32)),s<s(u8,u64)>(u64,str[15],s(u8,u64))))";

        assert_eq!(result, correct_fn_selector);

        Ok(())
    }

    #[test]
    fn weirdly_named_mutl_call() -> anyhow::Result<()> {
        let program_abi = load_abi_w_path("/home/segfault_magnet/fuel/github/fuels-rs/packages/fuels/tests/test_projects/contract_test/out/debug/contract_test-abi.json")?;
        let mappings = [
            ("get_msg_amount", "get_msg_amount()"),
            ("initialize_counter", "initialize_counter(u64)"),
            ("increment_counter", "increment_counter(u64)"),
            ("get_counter", "get_counter()"),
            ("get", "get(u64,u64)"),
            ("get_alt", "get_alt(s(u64,u64))"),
            ("get_single", "get_single(u64)"),
            ("array_of_structs", "array_of_structs(a[s(str[4]);2])"),
            ("array_of_enums", "array_of_enums(a[e((),(),());2])"),
            ("get_array", "get_array(a[u64;2])"),
        ]
        .map(|(lhs, rhs)| (lhs.to_string(), rhs.to_string()));

        let types = program_abi
            .types
            .iter()
            .map(|decl| (decl.type_id, decl.clone()))
            .collect::<HashMap<_, _>>();

        for fun in &program_abi.functions {
            let (name, exp) = mappings.iter().find(|(lhs, _)| fun.name == *lhs).unwrap();

            let result = resolve_function_selector(&fun, &types);

            assert_eq!(result, *exp);
        }

        Ok(())
    }

    fn load_abi(name: &str) -> anyhow::Result<ProgramABI> {
        let path = format!("/home/segfault_magnet/fuel/github/fuels-rs/packages/fuels/tests/test_projects/selector_testing/{name}/out/debug/{name}-abi.json");
        load_abi_w_path(&path)
    }

    fn load_abi_w_path(path: &str) -> anyhow::Result<ProgramABI> {
        let abi_contents = fs::read_to_string(&path)?;
        let result = serde_json::from_str(&abi_contents)?;
        Ok(result)
    }
}
