use std::iter::zip;

use fuels_types::{
    core::{Bits256, Byte, EvmAddress, Identity, SizedAsciiString, B512},
    enum_variants::EnumVariants,
    param_types::ParamType,
    Address, AssetId, ContractId,
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

impl Parameterize for B512 {
    fn param_type() -> ParamType {
        ParamType::Struct {
            name: "B512".to_string(),
            fields: vec![("bytes".to_string(), <[Bits256; 2usize]>::param_type())],
            generics: vec![],
        }
    }
}

impl Parameterize for EvmAddress {
    fn param_type() -> ParamType {
        ParamType::Struct {
            name: "EvmAddress".to_string(),
            fields: vec![("value".to_string(), ParamType::B256)],
            generics: vec![],
        }
    }
}

impl Parameterize for Byte {
    fn param_type() -> ParamType {
        ParamType::Byte
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

impl Parameterize for Address {
    fn param_type() -> ParamType {
        ParamType::Struct {
            name: "Address".to_string(),
            fields: vec![("0".to_string(), ParamType::B256)],
            generics: vec![],
        }
    }
}

impl Parameterize for ContractId {
    fn param_type() -> ParamType {
        ParamType::Struct {
            name: "ContractId".to_string(),
            fields: vec![("0".to_string(), ParamType::B256)],
            generics: vec![],
        }
    }
}

impl Parameterize for AssetId {
    fn param_type() -> ParamType {
        ParamType::Struct {
            name: "AssetId".to_string(),
            fields: vec![("0".to_string(), ParamType::B256)],
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

impl<T> Parameterize for Option<T>
where
    T: Parameterize,
{
    fn param_type() -> ParamType {
        let param_types = vec![
            ("None".to_string(), ParamType::Unit),
            ("Some".to_string(), T::param_type()),
        ];
        let variants = EnumVariants::new(param_types)
            .expect("should never happen as we provided valid Option param types");
        ParamType::Enum {
            name: "Option".to_string(),
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
        let variant_param_types = zip(
            vec!["Ok".to_string(), "Err".to_string()],
            param_types.clone(),
        )
        .collect();
        let variants = EnumVariants::new(variant_param_types)
            .expect("should never happen as we provided valid Result param types");
        ParamType::Enum {
            name: "Result".to_string(),
            variants,
            generics: param_types,
        }
    }
}

impl Parameterize for Identity {
    fn param_type() -> ParamType {
        let variants = EnumVariants::new(vec![
            ("Address".to_string(), Address::param_type()),
            ("ContractId".to_string(), ContractId::param_type()),
        ])
        .expect("should never happen as we provided valid Identity param types");
        ParamType::Enum {
            name: "Identity".to_string(),
            variants,
            generics: vec![],
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
    fn test_param_type_b256() {
        assert_eq!(Bits256::param_type(), ParamType::B256);
    }

    #[test]
    fn test_param_type_evm_addr() {
        assert_eq!(
            EvmAddress::param_type(),
            ParamType::Struct {
                name: "EvmAddress".to_string(),
                fields: vec![("value".to_string(), ParamType::B256)],
                generics: vec![]
            }
        );
    }

    #[test]
    fn sized_ascii_string_is_parameterized_correctly() {
        let param_type = SizedAsciiString::<3>::param_type();

        assert!(matches!(param_type, ParamType::String(3)));
    }
}
