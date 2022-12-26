use fuels_types::errors::Error;

use crate::code_gen::abi_types::FullABIFunction;

pub(crate) fn extract_main_fn(abi: &[FullABIFunction]) -> Result<&FullABIFunction, Error> {
    let candidates = abi
        .iter()
        .filter(|function| function.name() == "main")
        .collect::<Vec<_>>();

    match candidates.as_slice() {
        [single_main_fn] => Ok(single_main_fn),
        _ => {
            let fn_names = candidates
                .iter()
                .map(|candidate| candidate.name())
                .collect::<Vec<_>>();
            Err(Error::CompilationError(format!(
                "ABI must have one and only one function with the name 'main'. Got: {fn_names:?}"
            )))
        }
    }
}
