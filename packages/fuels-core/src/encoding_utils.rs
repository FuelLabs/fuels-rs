use crate::{ParamType, WORD_SIZE};

/// If given a non-empty slice it will return the number of `WORD`s needed to fit the biggest of the provided types.
pub fn max_by_encoding_width(params: &[ParamType]) -> Option<usize> {
    params.iter().map(encoding_width).max()
}

/// Calculates the number of `WORD`s the VM expects this parameter to be encoded in.
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
            max_by_encoding_width(variants.param_types()).unwrap() + DISCRIMINANT_WORD_SIZE
        }
        ParamType::Tuple(params) => params.iter().map(encoding_width).sum(),
    }
}

#[cfg(test)]
mod tests {
    const WIDTH_OF_B256: usize = 4;
    const WIDTH_OF_U32: usize = 1;
    const WIDTH_OF_BOOL: usize = 1;
    use crate::encoding_utils::encoding_width;
    use crate::{EnumVariants, ParamType};

    #[test]
    fn array_size_dependent_on_num_of_elements() {
        const NUM_ELEMENTS: usize = 11;
        let param = ParamType::Array(Box::new(ParamType::B256), NUM_ELEMENTS);

        let width = encoding_width(&param);

        let expected = NUM_ELEMENTS * WIDTH_OF_B256;
        assert_eq!(expected, width);
    }

    #[test]
    fn string_size_dependent_on_num_of_elements() {
        const NUM_ASCII_CHARS: usize = 9;
        let param = ParamType::String(NUM_ASCII_CHARS);

        let width = encoding_width(&param);

        // 2 WORDS or 16 B are enough to fit 9 ascii chars
        assert_eq!(2, width);
    }

    #[test]
    fn structs_are_just_all_elements_combined() {
        let inner_struct = ParamType::Struct(vec![ParamType::U32, ParamType::U32]);

        let a_struct = ParamType::Struct(vec![ParamType::B256, ParamType::Bool, inner_struct]);

        let width = encoding_width(&a_struct);

        const INNER_STRUCT_WIDTH: usize = WIDTH_OF_U32 * 2;
        const EXPECTED_WIDTH: usize = WIDTH_OF_B256 + WIDTH_OF_BOOL + INNER_STRUCT_WIDTH;
        assert_eq!(EXPECTED_WIDTH, width);
    }

    #[test]
    fn enums_are_as_big_as_their_biggest_variant_plus_a_word() {
        let inner_struct = ParamType::Struct(vec![ParamType::B256]);
        let param = ParamType::Enum(EnumVariants::new(vec![ParamType::U32, inner_struct]).unwrap());

        let width = encoding_width(&param);

        const INNER_STRUCT_SIZE: usize = WIDTH_OF_B256;
        const EXPECTED_WIDTH: usize = INNER_STRUCT_SIZE + 1;
        assert_eq!(EXPECTED_WIDTH, width);
    }

    #[test]
    fn tuples_are_just_all_elements_combined() {
        let inner_tuple = ParamType::Tuple(vec![ParamType::B256]);
        let param = ParamType::Tuple(vec![ParamType::U32, inner_tuple]);

        let width = encoding_width(&param);

        const INNER_TUPLE_WIDTH: usize = WIDTH_OF_B256;
        const EXPECTED_WIDTH: usize = WIDTH_OF_U32 + INNER_TUPLE_WIDTH;
        assert_eq!(EXPECTED_WIDTH, width);
    }
}
