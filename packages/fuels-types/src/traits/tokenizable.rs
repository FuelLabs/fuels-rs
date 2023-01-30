use fuel_types::{Address, AssetId, ContractId};

use crate::{
    core::{Bits256, Byte, RawSlice, SizedAsciiString, StringToken, Token},
    errors::Error,
    param_types::ParamType,
    traits::Parameterize,
};

pub trait Tokenizable {
    /// Converts a `Token` into expected type.
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized;
    /// Converts a specified type back into token.
    fn into_token(self) -> Token;
}

impl Tokenizable for Token {
    fn from_token(token: Token) -> Result<Self, Error> {
        Ok(token)
    }
    fn into_token(self) -> Token {
        self
    }
}

impl Tokenizable for Bits256 {
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        match token {
            Token::B256(data) => Ok(Bits256(data)),
            _ => Err(Error::InvalidData(format!(
                "Bits256 cannot be constructed from token {token}"
            ))),
        }
    }

    fn into_token(self) -> Token {
        Token::B256(self.0)
    }
}

impl Tokenizable for Byte {
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        match token {
            Token::Byte(value) => Ok(Byte(value)),
            _ => Err(Error::InvalidData(format!(
                "Byte::from_token failed! Can only handle Token::Byte, got {token:?}"
            ))),
        }
    }

    fn into_token(self) -> Token {
        Token::Byte(self.0)
    }
}

impl<T: Tokenizable> Tokenizable for Vec<T> {
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let Token::Vector(tokens) = token {
            tokens.into_iter().map(Tokenizable::from_token).collect()
        } else {
            Err(Error::InvalidData(format!(
                "Vec::from_token must only be given a Token::Vector. Got: {token}"
            )))
        }
    }

    fn into_token(self) -> Token {
        let tokens = self.into_iter().map(Tokenizable::into_token).collect();
        Token::Vector(tokens)
    }
}

impl Tokenizable for bool {
    fn from_token(token: Token) -> Result<Self, Error> {
        match token {
            Token::Bool(data) => Ok(data),
            other => Err(Error::InstantiationError(format!(
                "Expected `bool`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Token::Bool(self)
    }
}

impl Tokenizable for () {
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        match token {
            Token::Unit => Ok(()),
            other => Err(Error::InstantiationError(format!(
                "Expected `Unit`, got {:?}",
                other
            ))),
        }
    }

    fn into_token(self) -> Token {
        Token::Unit
    }
}

impl Tokenizable for u8 {
    fn from_token(token: Token) -> Result<Self, Error> {
        match token {
            Token::U8(data) => Ok(data),
            other => Err(Error::InstantiationError(format!(
                "Expected `u8`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Token::U8(self)
    }
}

impl Tokenizable for u16 {
    fn from_token(token: Token) -> Result<Self, Error> {
        match token {
            Token::U16(data) => Ok(data),
            other => Err(Error::InstantiationError(format!(
                "Expected `u16`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Token::U16(self)
    }
}

impl Tokenizable for u32 {
    fn from_token(token: Token) -> Result<Self, Error> {
        match token {
            Token::U32(data) => Ok(data),
            other => Err(Error::InstantiationError(format!(
                "Expected `u32`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Token::U32(self)
    }
}

impl Tokenizable for u64 {
    fn from_token(token: Token) -> Result<Self, Error> {
        match token {
            Token::U64(data) => Ok(data),
            other => Err(Error::InstantiationError(format!(
                "Expected `u64`, got {:?}",
                other
            ))),
        }
    }
    fn into_token(self) -> Token {
        Token::U64(self)
    }
}

impl Tokenizable for RawSlice {
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        match token {
            Token::RawSlice(contents) => Ok(Self(contents)),
            _ => Err(Error::InvalidData(format!(
                "RawSlice::from_token expected a token of the variant Token::RawSlice, got: {token}"
            ))),
        }
    }

    fn into_token(self) -> Token {
        Token::RawSlice(Vec::from(self))
    }
}

// Here we implement `Tokenizable` for a given tuple of a given length.
// This is done this way because we can't use `impl<T> Tokenizable for (T,)`.
// So we implement `Tokenizable` for each tuple length, covering
// a reasonable range of tuple lengths.
macro_rules! impl_tokenizable_tuples {
    ($num: expr, $( $ty: ident : $no: tt, )+) => {
        impl<$($ty, )+> Tokenizable for ($($ty,)+) where
            $(
                $ty: Tokenizable,
            )+
        {
            fn from_token(token: Token) -> Result<Self, Error> {
                match token {
                    Token::Tuple(tokens) => {
                        let mut it = tokens.into_iter();
                        let mut next_token = move || {
                            it.next().ok_or_else(|| {
                                Error::InstantiationError("Ran out of tokens before tuple could be constructed".to_string())
                            })
                        };
                        Ok(($(
                          $ty::from_token(next_token()?)?,
                        )+))
                    },
                    other => Err(Error::InstantiationError(format!(
                        "Expected `Tuple`, got {:?}",
                        other,
                    ))),
                }
            }

            fn into_token(self) -> Token {
                Token::Tuple(vec![
                    $( self.$no.into_token(), )+
                ])
            }
        }

    }
}

// And where we actually implement the `Tokenizable` for tuples
// from size 1 to size 16.
impl_tokenizable_tuples!(1, A:0, );
impl_tokenizable_tuples!(2, A:0, B:1, );
impl_tokenizable_tuples!(3, A:0, B:1, C:2, );
impl_tokenizable_tuples!(4, A:0, B:1, C:2, D:3, );
impl_tokenizable_tuples!(5, A:0, B:1, C:2, D:3, E:4, );
impl_tokenizable_tuples!(6, A:0, B:1, C:2, D:3, E:4, F:5, );
impl_tokenizable_tuples!(7, A:0, B:1, C:2, D:3, E:4, F:5, G:6, );
impl_tokenizable_tuples!(8, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, );
impl_tokenizable_tuples!(9, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, );
impl_tokenizable_tuples!(10, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, );
impl_tokenizable_tuples!(11, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, );
impl_tokenizable_tuples!(12, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, );
impl_tokenizable_tuples!(13, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12, );
impl_tokenizable_tuples!(14, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12, N:13, );
impl_tokenizable_tuples!(15, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12, N:13, O:14, );
impl_tokenizable_tuples!(16, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12, N:13, O:14, P:15, );

impl Tokenizable for ContractId {
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let Token::Struct(tokens) = token {
            if let [Token::B256(data)] = tokens.as_slice() {
                Ok(ContractId::from(*data))
            } else {
                Err(Error::InstantiationError(format!(
                    "ContractId expected one `Token::B256`, got {tokens:?}"
                )))
            }
        } else {
            Err(Error::InstantiationError(format!(
                "Address expected `Token::Struct` got {token:?}",
            )))
        }
    }

    fn into_token(self) -> Token {
        let underlying_data: &[u8; 32] = &self;
        Token::Struct(vec![Bits256(*underlying_data).into_token()])
    }
}

impl Tokenizable for Address {
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let Token::Struct(tokens) = token {
            if let [Token::B256(data)] = tokens.as_slice() {
                Ok(Address::from(*data))
            } else {
                Err(Error::InstantiationError(format!(
                    "Address expected one `Token::B256`, got {tokens:?}"
                )))
            }
        } else {
            Err(Error::InstantiationError(format!(
                "Address expected `Token::Struct` got {token:?}",
            )))
        }
    }

    fn into_token(self) -> Token {
        let underlying_data: &[u8; 32] = &self;

        Token::Struct(vec![Bits256(*underlying_data).into_token()])
    }
}

impl Tokenizable for AssetId {
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let Token::Struct(tokens) = token {
            if let [Token::B256(data)] = tokens.as_slice() {
                Ok(AssetId::from(*data))
            } else {
                Err(Error::InstantiationError(format!(
                    "AssetId expected one `Token::B256`, got {tokens:?}"
                )))
            }
        } else {
            Err(Error::InstantiationError(format!(
                "AssetId expected `Token::Struct` got {token:?}",
            )))
        }
    }

    fn into_token(self) -> Token {
        let underlying_data: &[u8; 32] = &self;
        Token::Struct(vec![Bits256(*underlying_data).into_token()])
    }
}

impl<T> Tokenizable for Option<T>
where
    T: Tokenizable + Parameterize,
{
    fn from_token(token: Token) -> Result<Self, Error> {
        if let Token::Enum(enum_selector) = token {
            match *enum_selector {
                (0u8, _, _) => Ok(None),
                (1u8, token, _) => Ok(Option::<T>::Some(T::from_token(token)?)),
                (_, _, _) => Err(Error::InstantiationError(format!(
                    "Could not construct Option from enum_selector. Received: {:?}",
                    enum_selector
                ))),
            }
        } else {
            Err(Error::InstantiationError(format!(
                "Could not construct Option from token. Received: {token:?}",
            )))
        }
    }
    fn into_token(self) -> Token {
        let (dis, tok) = match self {
            None => (0u8, Token::Unit),
            Some(value) => (1u8, value.into_token()),
        };
        if let ParamType::Enum { variants, .. } = Self::param_type() {
            let selector = (dis, tok, variants);
            Token::Enum(Box::new(selector))
        } else {
            panic!("should never happen as Option::param_type() returns valid Enum variants");
        }
    }
}

impl<T, E> Tokenizable for Result<T, E>
where
    T: Tokenizable + Parameterize,
    E: Tokenizable + Parameterize,
{
    fn from_token(token: Token) -> Result<Self, Error> {
        if let Token::Enum(enum_selector) = token {
            match *enum_selector {
                (0u8, token, _) => Ok(Result::<T, E>::Ok(T::from_token(token)?)),
                (1u8, token, _) => Ok(Result::<T, E>::Err(E::from_token(token)?)),
                (_, _, _) => Err(Error::InstantiationError(format!(
                    "Could not construct Result from enum_selector. Received: {:?}",
                    enum_selector
                ))),
            }
        } else {
            Err(Error::InstantiationError(format!(
                "Could not construct Result from token. Received: {token:?}",
            )))
        }
    }
    fn into_token(self) -> Token {
        let (dis, tok) = match self {
            Ok(value) => (0u8, value.into_token()),
            Err(value) => (1u8, value.into_token()),
        };
        if let ParamType::Enum { variants, .. } = Self::param_type() {
            let selector = (dis, tok, variants);
            Token::Enum(Box::new(selector))
        } else {
            panic!("should never happen as Result::param_type() returns valid Enum variants");
        }
    }
}

impl<const SIZE: usize, T: Tokenizable> Tokenizable for [T; SIZE] {
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        let gen_error = |reason| {
            Error::InvalidData(format!(
                "While constructing an array of size {SIZE}: {reason}"
            ))
        };

        match token {
            Token::Array(elements) => {
                let len = elements.len();
                if len != SIZE {
                    return Err(gen_error(format!(
                        "Was given a Token::Array with wrong number of elements: {len}"
                    )));
                }

                let detokenized = elements
                    .into_iter()
                    .map(Tokenizable::from_token)
                    .collect::<Result<Vec<T>, _>>()
                    .map_err(|err| {
                        gen_error(format!(", not all elements could be detokenized: {err}"))
                    })?;

                Ok(detokenized.try_into().unwrap_or_else(|_| {
                    panic!("This should never fail since we're checking the length beforehand.")
                }))
            }
            _ => Err(gen_error(format!("Expected a Token::Array, got {token}"))),
        }
    }

    fn into_token(self) -> Token {
        Token::Array(self.map(Tokenizable::into_token).to_vec())
    }
}

impl<const LEN: usize> Tokenizable for SizedAsciiString<LEN> {
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        match token {
            Token::String(contents) => {
                let expected_len = contents.get_encodable_str()?.len() ;
                if expected_len!= LEN {
                    return Err(Error::InvalidData(format!("SizedAsciiString<{LEN}>::from_token got a Token::String whose expected length({}) is != {LEN}", expected_len)))
                }
                Self::new(contents.try_into()?)
            },
            _ => {
                Err(Error::InvalidData(format!("SizedAsciiString<{LEN}>::from_token expected a token of the variant Token::String, got: {token}")))
            }
        }
    }

    fn into_token(self) -> Token {
        Token::String(StringToken::new(self.into(), LEN))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_token_b256() -> Result<(), Error> {
        let data = [1u8; 32];
        let token = Token::B256(data);

        let bits256 = Bits256::from_token(token)?;

        assert_eq!(bits256.0, data);

        Ok(())
    }

    #[test]
    fn test_into_token_b256() {
        let bytes = [1u8; 32];
        let bits256 = Bits256(bytes);

        let token = bits256.into_token();

        assert_eq!(token, Token::B256(bytes));
    }

    #[test]
    fn test_from_token_raw_slice() -> Result<(), Error> {
        let data = vec![42; 11];
        let token = Token::RawSlice(data.clone());

        let slice = RawSlice::from_token(token)?;

        assert_eq!(slice, data);

        Ok(())
    }

    #[test]
    fn test_into_token_raw_slice() {
        let data = vec![13; 32];
        let raw_slice_token = Token::RawSlice(data.clone());

        let token = raw_slice_token.into_token();

        assert_eq!(token, Token::RawSlice(data));
    }

    #[test]
    fn sized_ascii_string_is_tokenized_correctly() -> Result<(), Error> {
        let sut = SizedAsciiString::<3>::new("abc".to_string())?;

        let token = sut.into_token();

        match token {
            Token::String(string_token) => {
                let contents = string_token.get_encodable_str()?;
                assert_eq!(contents, "abc");
            }
            _ => {
                panic!("Not tokenized correctly! Should have gotten a Token::String")
            }
        }

        Ok(())
    }

    #[test]
    fn sized_ascii_string_is_detokenized_correctly() -> Result<(), Error> {
        let token = Token::String(StringToken::new("abc".to_string(), 3));

        let sized_ascii_string =
            SizedAsciiString::<3>::from_token(token).expect("Should have succeeded");

        assert_eq!(sized_ascii_string, "abc");

        Ok(())
    }
}
