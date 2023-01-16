pub(crate) use command::Command;
use itertools::{chain, Itertools};
use quote::ToTokens;
use syn::Error;
pub(crate) use unique_lit_strs::UniqueLitStrs;
pub(crate) use unique_name_values::UniqueNameValues;

mod command;
mod unique_lit_strs;
mod unique_name_values;

pub(crate) trait ErrorsExt: Iterator<Item = Error> + Sized {
    fn combine_errors(self) -> Option<Self::Item>;
    fn validate_no_errors(self) -> Result<(), Self::Item>;
}

impl<T> ErrorsExt for T
where
    T: Iterator<Item = Error> + Sized,
{
    fn combine_errors(self) -> Option<Self::Item> {
        self.reduce(|mut errors, error| {
            errors.combine(error);
            errors
        })
    }

    fn validate_no_errors(self) -> Result<(), Self::Item> {
        if let Some(err) = self.combine_errors() {
            Err(err)
        } else {
            Ok(())
        }
    }
}

fn generate_duplicate_error<T>(duplicates: &[&T]) -> Error
where
    T: ToTokens,
{
    let mut iter = duplicates.iter();

    let original_error = iter
        .next()
        .map(|first_el| Error::new_spanned(first_el, "Original defined here:"));

    let the_rest = iter.map(|duplicate| Error::new_spanned(duplicate, "Duplicate!"));

    chain!(original_error, the_rest)
        .combine_errors()
        .expect("Has to be at least one error!")
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
    T: ToTokens,
    K: Ord,
{
    group_up_duplicates(elements, key_fn)
        .into_iter()
        .map(|duplicates| generate_duplicate_error(&duplicates))
        .validate_no_errors()
}
