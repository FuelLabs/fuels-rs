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
    // eprintln!(
    //     "doing type_appl: {} with generics: {}",
    //     type_application.type_id,
    //     print_lookup(generics)
    // );
    let param_type = ParamType::from_type_declaration(&type_decl, &types).unwrap();

    if let ParamType::Generic(name) = &param_type {
        let lookup = generics.iter().cloned().collect::<HashMap<_, _>>();
        let option = lookup.get(&type_application.type_id);
        // if option.is_none() {
        //     eprintln!(
        //         "Came across a generic {name} in type_appl {} that had no match! Lookup was: {}",
        //         type_application.type_id,
        //         print_lookup(&generics)
        //     );
        // } else {
        //     eprintln!(
        //         "Resolved generic {name} id: {} to type {:?}",
        //         type_application.type_id,
        //         option.unwrap()
        //     )
        // }
        return option.unwrap().clone();
    }

    // eprintln!("Resolving type_arguments to get the generics needed");
    let some_generics = type_application
        .type_arguments
        .iter()
        .flatten()
        .map(|ty| resolve_type_application(ty, types, &generics))
        .collect_vec();

    let final_generics = if some_generics.is_empty() {
        // eprintln!("Type arguments had nothing, proceeding to use parent generics");
        generics.iter().map(|(_, ty)| ty.clone()).collect_vec()
    } else {
        some_generics
    };

    let final_generics = match &type_decl.type_parameters {
        Some(params) if !params.is_empty() => {
            let final_generics =
                zip(params.clone(), final_generics.iter().map(|ty| ty.clone())).collect_vec();
            // eprintln!(
            //     "After mapping to {params:?} generics are: {}",
            //     print_lookup(&final_generics)
            // );
            final_generics
        }
        _ => generics.clone(),
    };

    // eprintln!(
    //     "After applying generics to the type arguments we're ready to resolve components with generics: {}",
    //     print_lookup(&final_generics)
    // );

    let components = type_decl
        .components
        .iter()
        .flatten()
        .map(|component| resolve_type_application(component, types, &final_generics))
        .collect_vec();

    // eprintln!("Resolved type id {}", type_application.type_id);
    let x = Type {
        param_type,
        type_id: type_decl.type_id,
        components,
        generic_params: final_generics
            .clone()
            .iter()
            .map(|(_, ty)| ty.clone())
            .collect(),
    };
    eprintln!(
        "Resolved type id {} = {}",
        type_application.type_id,
        x.to_fn_selector_format()
    );
    x
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

fn print_lookup(arg: &[(usize, Type)]) -> String {
    let lookup = arg.iter().cloned().collect::<HashMap<_, _>>();
    let ids = arg.iter().map(|(id, _)| id).collect_vec();
    let map = serde_json::to_string_pretty(&lookup).unwrap();
    format!("order {ids:?} {map}")
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

        let mappings = [("struct_with_single_generic", "struct_with_single_generic(s<u32>(u32))"),
        ("struct_with_multiple_generics", "struct_with_multiple_generics(s<u32,u64>(u32,u64))"),
        ("struct_passing_the_generic_on", "struct_passing_the_generic_on(s<u32>(s<u32>(u32)))"),
        ("enum_with_single_generic", "enum_with_single_generic(e<u32>((),u32))"),
        ("generics_in_tuple", "generics_in_tuple(s<s<u64>(s<u64>(u64))>((u64,s<u64>(s<u64>(u64)))))"),
        ("complex_test", "complex_test(s<s<u64>(s<u64>(u64)),s<str[2]>(str[2])>((a[s<str[2]>(str[2]);2],s<u64>(s<u64>(u64)))))")]
            .map(|(lhs, rhs)| (lhs.to_string(), rhs.to_string()));

        let types = program_abi
            .types
            .iter()
            .map(|decl| (decl.type_id, decl.clone()))
            .collect::<HashMap<_, _>>();

        for fun in &program_abi.functions {
            let (name, exp) = mappings.iter().find(|(lhs, _)| fun.name == *lhs).unwrap();
            if name != "complex_test" {
                continue;
            }

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
