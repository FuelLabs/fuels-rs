use fuel_types::{Address, AssetId, ContractId};

use crate::types::{
    enum_variants::EnumVariants, param_types::ParamType, Bits256, Bytes, RawSlice, SizedAsciiString,
};

/// `abigen` requires `Parameterized` to construct nested types. It is also used by `try_from_bytes`
/// to facilitate the instantiation of custom types from bytes.
pub trait Parameterize {
    fn param_type() -> ParamType;
}

impl Parameterize for Bits256 {
    fn param_type() -> ParamType {
        ParamType::B256
    }
}

impl Parameterize for RawSlice {
    fn param_type() -> ParamType {
        ParamType::RawSlice
    }
}

impl<const SIZE: usize, T: Parameterize> Parameterize for [T; SIZE] {
    fn param_type() -> ParamType {
        ParamType::Array(Box::new(T::param_type()), SIZE)
    }
}

impl<T: Parameterize> Parameterize for Vec<T> {
    fn param_type() -> ParamType {
        ParamType::Vector(Box::new(T::param_type()))
    }
}

impl Parameterize for Bytes {
    fn param_type() -> ParamType {
        ParamType::Bytes
    }
}

impl Parameterize for Address {
    fn param_type() -> ParamType {
        ParamType::Struct {
            fields: vec![ParamType::B256],
            generics: vec![],
        }
    }
}

impl Parameterize for ContractId {
    fn param_type() -> ParamType {
        ParamType::Struct {
            fields: vec![ParamType::B256],
            generics: vec![],
        }
    }
}

impl Parameterize for AssetId {
    fn param_type() -> ParamType {
        ParamType::Struct {
            fields: vec![ParamType::B256],
            generics: vec![],
        }
    }
}

impl Parameterize for () {
    fn param_type() -> ParamType {
        ParamType::Unit
    }
}

impl Parameterize for bool {
    fn param_type() -> ParamType {
        ParamType::Bool
    }
}

impl Parameterize for u8 {
    fn param_type() -> ParamType {
        ParamType::U8
    }
}

impl Parameterize for u16 {
    fn param_type() -> ParamType {
        ParamType::U16
    }
}

impl Parameterize for u32 {
    fn param_type() -> ParamType {
        ParamType::U32
    }
}

impl Parameterize for u64 {
    fn param_type() -> ParamType {
        ParamType::U64
    }
}

impl Parameterize for u128 {
    fn param_type() -> ParamType {
        ParamType::U128
    }
}

impl<T> Parameterize for Option<T>
where
    T: Parameterize,
{
    fn param_type() -> ParamType {
        let param_types = vec![ParamType::Unit, T::param_type()];
        let variants = EnumVariants::new(param_types)
            .expect("should never happen as we provided valid Option param types");
        ParamType::Enum {
            variants,
            generics: vec![T::param_type()],
        }
    }
}

impl<T, E> Parameterize for Result<T, E>
where
    T: Parameterize,
    E: Parameterize,
{
    fn param_type() -> ParamType {
        let param_types = vec![T::param_type(), E::param_type()];
        let variants = EnumVariants::new(param_types.clone())
            .expect("should never happen as we provided valid Result param types");
        ParamType::Enum {
            variants,
            generics: param_types,
        }
    }
}

impl<const LEN: usize> Parameterize for SizedAsciiString<LEN> {
    fn param_type() -> ParamType {
        ParamType::String(LEN)
    }
}

// Here we implement `Parameterize` for a given tuple of a given length.
// This is done this way because we can't use `impl<T> Parameterize for (T,)`.
// So we implement `Parameterize` for each tuple length, covering
// a reasonable range of tuple lengths.
macro_rules! impl_parameterize_tuples {
    ($num: expr, $( $ty: ident : $no: tt, )+) => {
        impl<$($ty, )+> Parameterize for ($($ty,)+) where
            $(
                $ty: Parameterize,
            )+
        {
            fn param_type() -> ParamType {
                ParamType::Tuple(vec![
                    $( $ty::param_type(), )+
                ])
            }

        }
    }
}

// And where we actually implement the `Parameterize` for tuples
// from size 1 to size 16.
impl_parameterize_tuples!(1, A:0, );
impl_parameterize_tuples!(2, A:0, B:1, );
impl_parameterize_tuples!(3, A:0, B:1, C:2, );
impl_parameterize_tuples!(4, A:0, B:1, C:2, D:3, );
impl_parameterize_tuples!(5, A:0, B:1, C:2, D:3, E:4, );
impl_parameterize_tuples!(6, A:0, B:1, C:2, D:3, E:4, F:5, );
impl_parameterize_tuples!(7, A:0, B:1, C:2, D:3, E:4, F:5, G:6, );
impl_parameterize_tuples!(8, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, );
impl_parameterize_tuples!(9, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, );
impl_parameterize_tuples!(10, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, );
impl_parameterize_tuples!(11, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, );
impl_parameterize_tuples!(12, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, );
impl_parameterize_tuples!(13, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12, );
impl_parameterize_tuples!(14, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12, N:13, );
impl_parameterize_tuples!(15, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12, N:13, O:14, );
impl_parameterize_tuples!(16, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12, N:13, O:14, P:15, );

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sized_ascii_string_is_parameterized_correctly() {
        let param_type = SizedAsciiString::<3>::param_type();

        assert!(matches!(param_type, ParamType::String(3)));
    }

    #[test]
    fn test_param_type_b256() {
        assert_eq!(Bits256::param_type(), ParamType::B256);
    }

    #[test]
    fn test_param_type_raw_slice() {
        assert_eq!(RawSlice::param_type(), ParamType::RawSlice);
    }
}
