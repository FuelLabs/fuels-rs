use crate::{ParamType, WORD_SIZE};

/// If given a non-empty slice it will return the number of `WORD`s needed to fit the biggest of the provided types.
///
///
/// returns: Option<usize>
///
pub fn max_by_encoding_width(params: &[ParamType]) -> Option<usize> {
    params.iter().map(encoding_width).max()
}

/// Calculates the number of `WORD`s the VM expects this parameter to be encoded in.
///
/// # Arguments
///
/// * `param`: the parameter that you plan on encoding
///
/// returns: usize
///
/// # Panics
/// Calculating will panic if you pass an ParamType::Enum without any variants since that is an invalid type.
pub fn encoding_width(param: &ParamType) -> usize {
    const fn count_words(bytes: usize) -> usize {
        let q = bytes / WORD_SIZE;
        let r = bytes % WORD_SIZE;
        match r == 0 {
            true => q,
            false => q + 1,
        }
    }

    match param {
        ParamType::Unit => 0,
        ParamType::U8
        | ParamType::U16
        | ParamType::U32
        | ParamType::U64
        | ParamType::Bool
        | ParamType::Byte => 1,
        ParamType::B256 => 4,
        ParamType::Array(param, count) => encoding_width(param) * count,
        ParamType::String(len) => count_words(*len),
        ParamType::Struct(params) => params.iter().map(encoding_width).sum(),
        ParamType::Enum(variants) => {
            const DISCRIMINANT_WORD_SIZE: usize = 1;
            max_by_encoding_width(variants).unwrap() + DISCRIMINANT_WORD_SIZE
        }
        ParamType::Tuple(params) => params.iter().map(encoding_width).sum(),
    }
}

#[cfg(test)]
mod tests {
    use crate::encoding_utils::encoding_width;
    use crate::ParamType;

    #[test]
    fn array_size_dependent_on_num_of_elements() {
        let param = ParamType::Array(Box::new(ParamType::B256), 11);

        let width = encoding_width(&param);

        assert_eq!(44, width);
    }

    #[test]
    fn string_size_dependent_on_num_of_elements() {
        let param = ParamType::String(9);

        let width = encoding_width(&param);

        assert_eq!(2, width);
    }

    #[test]
    fn structs_are_just_all_elements_combined() {
        let inner_struct = ParamType::Struct(vec![ParamType::U32, ParamType::U32]);
        let param = ParamType::Struct(vec![ParamType::B256, ParamType::Bool, inner_struct]);

        let width = encoding_width(&param);

        assert_eq!(7, width);
    }

    #[test]
    fn enums_are_as_big_as_their_biggest_variant_plus_a_word() {
        let inner_struct = ParamType::Struct(vec![ParamType::B256]);
        let param = ParamType::Enum(vec![ParamType::U32, inner_struct]);

        let width = encoding_width(&param);

        assert_eq!(5, width);
    }

    #[test]
    fn tuples_are_just_all_elements_combined() {
        let inner_tuple = ParamType::Tuple(vec![ParamType::B256]);
        let param = ParamType::Tuple(vec![ParamType::U32, inner_tuple]);

        let width = encoding_width(&param);

        assert_eq!(5, width);
    }
}
