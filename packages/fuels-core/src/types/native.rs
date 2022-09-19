use crate::{Bits256, Parameterize, Token, Tokenizable};
use fuels_types::errors::Error;
use fuels_types::param_types::ParamType;

impl<T: Parameterize> Parameterize for Vec<T> {
    fn param_type() -> ParamType {
        ParamType::Vector(Box::new(T::param_type()))
    }
}

impl<T: Tokenizable> Tokenizable for Vec<T> {
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        match token {
            Token::Vector(tokens) => tokens.into_iter().map(Tokenizable::from_token).collect(),
            _ => Err(Error::InvalidData(format!(
                "The only type of token a Vec can be created out of is Token::Vector, got: {token}"
            ))),
        }
    }

    fn into_token(self) -> Token {
        Token::Vector(self.into_iter().map(Tokenizable::into_token).collect())
    }
}

impl<const SIZE: usize, T: Parameterize> Parameterize for [T; SIZE] {
    fn param_type() -> ParamType {
        ParamType::Array(Box::new(T::param_type()), SIZE)
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

impl Tokenizable for fuel_tx::ContractId {
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let Token::Struct(tokens) = token {
            let first_token = tokens.into_iter().next();
            if let Some(Token::B256(id)) = first_token {
                Ok(fuel_tx::ContractId::from(id))
            } else {
                Err(Error::InstantiationError(format!(
                    "Expected `b256`, got {:?}",
                    first_token
                )))
            }
        } else {
            Err(Error::InstantiationError(format!(
                "Expected `ContractId`, got {:?}",
                token
            )))
        }
    }

    fn into_token(self) -> Token {
        let underlying_data: &[u8; 32] = &self;
        Token::Struct(vec![Bits256(*underlying_data).into_token()])
    }
}

impl Tokenizable for fuel_tx::Address {
    fn from_token(t: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let Token::Struct(tokens) = t {
            let first_token = tokens.into_iter().next();
            if let Some(Token::B256(id)) = first_token {
                Ok(fuel_tx::Address::from(id))
            } else {
                Err(Error::InstantiationError(format!(
                    "Expected `b256`, got {:?}",
                    first_token
                )))
            }
        } else {
            Err(Error::InstantiationError(format!(
                "Expected `Address`, got {:?}",
                t
            )))
        }
    }

    fn into_token(self) -> Token {
        let underlying_data: &[u8; 32] = &self;

        Token::Struct(vec![Bits256(*underlying_data).into_token()])
    }
}

impl Tokenizable for fuel_tx::AssetId {
    fn from_token(token: Token) -> Result<Self, Error>
    where
        Self: Sized,
    {
        if let Token::Struct(inner_tokens) = token {
            let first_token = inner_tokens.into_iter().next();
            if let Some(Token::B256(id)) = first_token {
                Ok(Self::from(id))
            } else {
                Err(Error::InstantiationError(format!("Could not construct 'AssetId' from token. Wrong token inside of Struct '{:?} instead of B256'", first_token)))
            }
        } else {
            Err(Error::InstantiationError(format!("Could not construct 'AssetId' from token. Instead of a Struct with a B256 inside, received: {:?}", token)))
        }
    }

    fn into_token(self) -> Token {
        let underlying_data: &[u8; 32] = &self;
        Token::Struct(vec![Bits256(*underlying_data).into_token()])
    }
}

impl Parameterize for fuel_tx::Address {
    fn param_type() -> ParamType {
        ParamType::Struct(vec![ParamType::B256])
    }
}

impl Parameterize for fuel_tx::ContractId {
    fn param_type() -> ParamType {
        ParamType::Struct(vec![ParamType::B256])
    }
}

impl Parameterize for fuel_tx::AssetId {
    fn param_type() -> ParamType {
        ParamType::Struct(vec![ParamType::B256])
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

// Here we implement `Tokenizable` for a given tuple of a given length.
// This is done this way because we can't use `impl<T> Tokenizable for (T,)`.
// So we implement `Tokenizable` for each tuple length, covering
// a reasonable range of tuple lengths.
macro_rules! impl_tuples {
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

// And where we actually implement the `Tokenizable` for tuples
// from size 1 to size 16.
impl_tuples!(1, A:0, );
impl_tuples!(2, A:0, B:1, );
impl_tuples!(3, A:0, B:1, C:2, );
impl_tuples!(4, A:0, B:1, C:2, D:3, );
impl_tuples!(5, A:0, B:1, C:2, D:3, E:4, );
impl_tuples!(6, A:0, B:1, C:2, D:3, E:4, F:5, );
impl_tuples!(7, A:0, B:1, C:2, D:3, E:4, F:5, G:6, );
impl_tuples!(8, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, );
impl_tuples!(9, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, );
impl_tuples!(10, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, );
impl_tuples!(11, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, );
impl_tuples!(12, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, );
impl_tuples!(13, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12, );
impl_tuples!(14, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12, N:13, );
impl_tuples!(15, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12, N:13, O:14, );
impl_tuples!(16, A:0, B:1, C:2, D:3, E:4, F:5, G:6, H:7, I:8, J:9, K:10, L:11, M:12, N:13, O:14, P:15, );
