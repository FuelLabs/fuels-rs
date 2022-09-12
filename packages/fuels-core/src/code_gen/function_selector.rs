use fuels_types::param_types::ParamType;
use fuels_types::{ABIFunction, TypeApplication, TypeDeclaration};
use itertools::Itertools;
use serde::Serialize;
use std::collections::HashMap;
use std::iter::zip;
use std::ptr::replace;

pub fn resolve_fn_selector(
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

#[derive(Debug, Clone, Serialize)]
struct Type {
    param_type: ParamType,
    type_id: usize,
    generic_params: Vec<Type>,
    components: Vec<Type>,
}

fn resolve_type_application(
    type_application: &TypeApplication,
    types: &HashMap<usize, TypeDeclaration>,
    generics: &Vec<(usize, Type)>,
) -> Type {
    let type_decl = types.get(&type_application.type_id).unwrap();
    let param_type = ParamType::from_type_declaration(&type_decl, &types).unwrap();

    if let ParamType::Generic(_) = &param_type {
        let lookup = generics.iter().cloned().collect::<HashMap<_, _>>();
        return lookup.get(&type_application.type_id).unwrap().clone();
    }

    let some_generics = type_application
        .type_arguments
        .iter()
        .flatten()
        .map(|ty| resolve_type_application(ty, types, &generics))
        .collect_vec();

    let final_generics = if some_generics.is_empty() {
        generics.iter().map(|(_, ty)| ty.clone()).collect_vec()
    } else {
        some_generics
    };

    let final_generics = match &type_decl.type_parameters {
        Some(params) if !params.is_empty() => {
            zip(params.clone(), final_generics.iter().map(|ty| ty.clone())).collect_vec()
        }
        _ => generics.clone(),
    };

    let components = type_decl
        .components
        .iter()
        .flatten()
        .map(|component| resolve_type_application(component, types, &final_generics))
        .collect_vec();

    Type {
        param_type,
        type_id: type_decl.type_id,
        components,
        generic_params: final_generics
            .clone()
            .iter()
            .map(|(_, ty)| ty.clone())
            .collect(),
    }
}

impl Type {
    pub fn to_fn_selector_format(&self) -> String {
        let get_components = || {
            self.components
                .iter()
                .map(|component| component.to_fn_selector_format())
                .collect::<Vec<_>>()
                .join(",")
        };

        let get_generics = || {
            let generics = self
                .generic_params
                .iter()
                .map(|arg| arg.to_fn_selector_format())
                .collect::<Vec<_>>()
                .join(",");

            if generics.is_empty() {
                String::new()
            } else {
                format!("<{generics}>")
            }
        };

        let a = match &self.param_type {
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
            ParamType::Array(_, len) => {
                let components = get_components();
                format!("a[{components};{len}]")
            }
            ParamType::Struct(_) => {
                let generics = get_generics();
                let components = get_components();
                format!("s{generics}({components})")
            }
            ParamType::Enum(_) => {
                let generics = get_generics();
                let components = get_components();
                format!("e{generics}({components})")
            }
            ParamType::Tuple(_) => {
                let components = get_components();
                format!("({components})")
            }
            ParamType::Generic(_name) => {
                panic!("ParamType::Generic cannot appear in a function selector!")
            }
        };
        a
    }
}

fn resolve_function_arg(arg: &TypeApplication, types: &HashMap<usize, TypeDeclaration>) -> String {
    resolve_type_application(arg, types, &Default::default()).to_fn_selector_format()
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

        let result = resolve_fn_selector(&fun, &types);

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

        let result = resolve_fn_selector(&fun, &types);

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

        let result = resolve_fn_selector(&fun, &types);

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

        let result = resolve_fn_selector(&fun, &types);

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

        let result = resolve_fn_selector(&fun, &types);

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

            let result = resolve_fn_selector(&fun, &types);

            assert_eq!(result, *exp);
        }

        Ok(())
    }

    #[test]
    fn understanding() -> anyhow::Result<()> {
        let program_abi = load_abi_w_path("/home/segfault_magnet/tmp/out/debug/tmp-abi.json")?;
        let mappings = [(
            "test_function",
            "test_function(s<u64,u32>(a[s<s<u32>(u32)>(s<u32>(u32));2],u64))",
        )]
        .map(|(lhs, rhs)| (lhs.to_string(), rhs.to_string()));

        let types = program_abi
            .types
            .iter()
            .map(|decl| (decl.type_id, decl.clone()))
            .collect::<HashMap<_, _>>();

        for fun in &program_abi.functions {
            let (name, exp) = mappings.iter().find(|(lhs, _)| fun.name == *lhs).unwrap();

            let result = resolve_fn_selector(&fun, &types);

            assert_eq!(result, *exp);
        }

        Ok(())
    }

    #[test]
    fn super_generics() -> anyhow::Result<()> {
        let program_abi = load_abi_w_path("/home/segfault_magnet/fuel/github/fuels-rs/packages/fuels/tests/test_projects/generics/out/debug/generics-abi.json")?;

        let mappings = [("struct_w_generic", "struct_w_generic(s<u64>(u64))"),
        ("struct_delegating_generic", "struct_delegating_generic(s<str[3]>(s<str[3]>(str[3])))"),
        ("struct_w_generic_in_array", "struct_w_generic_in_array(s<u32>(a[u32;2]))"),
        ("struct_w_generic_in_tuple", "struct_w_generic_in_tuple(s<u32>((u32,u32)))"),
        ("enum_w_generic", "enum_w_generic(e<u64>(u64,u64))"),
        ("complex_test", "complex_test(s<str[2],b256>((a[b256;2],str[2]),(a[e<s<s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2])>((s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2]),s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2])))>(u64,s<s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2])>((s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2]),s<s<str[2]>(s<str[2]>(str[2]))>(a[s<str[2]>(s<str[2]>(str[2]));2]))));1],u32)))")]
        .map(|(lhs, rhs)| (lhs.to_string(), rhs.to_string()));

        let types = program_abi
            .types
            .iter()
            .map(|decl| (decl.type_id, decl.clone()))
            .collect::<HashMap<_, _>>();

        for fun in &program_abi.functions {
            let (name, exp) = mappings.iter().find(|(lhs, _)| fun.name == *lhs).unwrap();

            let result = resolve_fn_selector(&fun, &types);

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
