use fuel_abi_types::abi::full_program::FullABIFunction;

use crate::error::{Result, error};

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
                "`abi` must have only one function with the name 'main'. Got: {fn_names:?}"
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use fuel_abi_types::abi::full_program::{FullTypeApplication, FullTypeDeclaration};

    use super::*;

    #[test]
    fn correctly_extracts_the_main_fn() {
        let functions = ["fn_1", "main", "fn_2"].map(given_a_fun_named);

        let fun = extract_main_fn(&functions).expect("should have succeeded");

        assert_eq!(*fun, functions[1]);
    }

    #[test]
    fn fails_if_there_is_more_than_one_main_fn() {
        let functions = ["main", "another", "main"].map(given_a_fun_named);

        let err = extract_main_fn(&functions).expect_err("should have failed");

        assert_eq!(
            err.to_string(),
            r#"`abi` must have only one function with the name 'main'. Got: ["main", "another", "main"]"#
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
                error_message: None,
            },
            vec![],
        )
        .expect("hand-crafted, should not fail!")
    }
}
