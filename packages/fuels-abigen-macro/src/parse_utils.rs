use itertools::{chain, Itertools};
use quote::ToTokens;
use syn::Error;

pub(crate) use command::Command;
pub(crate) use unique_lit_strs::UniqueLitStrs;
pub(crate) use unique_name_values::UniqueNameValues;

mod command;
mod unique_lit_strs;
mod unique_name_values;

pub(crate) fn combine_errors<T: IntoIterator<Item = Error>>(errs: T) -> Option<Error> {
    errs.into_iter().reduce(|mut errors, error| {
        errors.combine(error);
        errors
    })
}

fn generate_duplicate_error<T, K, KeyFn>(duplicates: &[&T], key_fn: KeyFn) -> Error
where
    KeyFn: Fn(&&T) -> K,
    K: ToTokens,
{
    let mut iter = duplicates.iter();

    let original_error = iter
        .next()
        .map(|first_el| Error::new_spanned(key_fn(first_el), "Original defined here:"));

    let the_rest = iter
        .map(|duplicate| Error::new_spanned(key_fn(duplicate), "Duplicate!"))
        .collect::<Vec<_>>();

    combine_errors(chain!(original_error, the_rest)).expect("Has to be at least one error!")
}

fn group_up_duplicates<T, K, KeyFn>(name_values: &[T], key: KeyFn) -> Vec<Vec<&T>>
where
    KeyFn: Fn(&&T) -> K,
    K: Ord,
{
    name_values
        .iter()
        .sorted_by_key(&key)
        .group_by(&key)
        .into_iter()
        .filter_map(|(_, group)| {
            let group = group.collect::<Vec<_>>();

            (group.len() > 1).then_some(group)
        })
        .collect()
}

fn validate_no_duplicates<T, K, KeyFn>(elements: &[T], key_fn: KeyFn) -> syn::Result<()>
where
    KeyFn: Fn(&&T) -> K + Copy,
    K: Ord + ToTokens,
{
    let maybe_err = group_up_duplicates(elements, key_fn)
        .into_iter()
        .map(|duplicates| generate_duplicate_error(&duplicates, key_fn))
        .reduce(|mut errors, error| {
            errors.combine(error);
            errors
        });

    if let Some(err) = maybe_err {
        Err(err)
    } else {
        Ok(())
    }
}
