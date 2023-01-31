use crate::{
    abigen::code_gen::abi_types::FullABIFunction,
    error::{error, Result},
};

pub(crate) fn extract_main_fn(abi: &[FullABIFunction]) -> Result<&FullABIFunction> {
    let candidates = abi
        .iter()
        .filter(|function| function.name() == "main")
        .collect::<Vec<_>>();

    match candidates.as_slice() {
        [single_main_fn] => Ok(single_main_fn),
        _ => {
            let fn_names = abi
                .iter()
                .map(|candidate| candidate.name())
                .collect::<Vec<_>>();
            Err(error!(
                "ABI must have one and only one function with the name 'main'. Got: {fn_names:?}"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abigen::code_gen::abi_types::{FullTypeApplication, FullTypeDeclaration};

    #[test]
    fn correctly_extracts_the_main_fn() {
        let functions = ["fn_1", "main", "fn_2"].map(given_a_fun_named);

        let fun = extract_main_fn(&functions).expect("Should have succeeded");

        assert_eq!(*fun, functions[1]);
    }

    #[test]
    fn fails_if_there_is_more_than_one_main_fn() {
        let functions = ["main", "another", "main"].map(given_a_fun_named);

        let err = extract_main_fn(&functions).expect_err("Should have failed.");

        assert_eq!(
            err.to_string(),
            r#"ABI must have one and only one function with the name 'main'. Got: ["main", "another", "main"]"#
        );
    }

    fn given_a_fun_named(fn_name: &str) -> FullABIFunction {
        FullABIFunction::new(
            fn_name.to_string(),
            vec![],
            FullTypeApplication {
                name: "".to_string(),
                type_decl: FullTypeDeclaration {
                    type_field: "".to_string(),
                    components: vec![],
                    type_parameters: vec![],
                },
                type_arguments: vec![],
            },
        )
        .expect("hand-crafted, should not fail!")
    }
}
