mod enum_gen;
mod struct_gen;
mod utils;

pub use enum_gen::expand_custom_enum;
pub use fuels_types::utils::custom_type_name;
pub use struct_gen::expand_custom_struct;
pub use utils::{param_type_calls, single_param_type_call, Component};

// Doing string -> TokenStream -> string isn't pretty but gives us the opportunity to
// have a better understanding of the generated code so we consider it ok.
// To generate the expected examples, output of the functions were taken
// with code @9ca376, and formatted in-IDE using rustfmt. It should be noted that
// rustfmt added an extra `,` after the last struct/enum field, which is not added
// by the `expand_custom_*` functions, and so was removed from the expected string.
// TODO(vnepveu): append extra `,` to last enum/struct field so it is aligned with rustfmt
#[cfg(test)]
mod tests {
    use std::{collections::HashMap, str::FromStr};

    use super::*;
    use crate::Error;

    use anyhow::anyhow;
    use fuels_types::{ProgramABI, TypeApplication, TypeDeclaration};
    use proc_macro2::TokenStream;

    #[test]
    fn test_expand_custom_enum() -> Result<(), Error> {
        let p = TypeDeclaration {
            type_id: 0,
            type_field: String::from("enum MatchaTea"),
            components: Some(vec![
                TypeApplication {
                    name: String::from("LongIsland"),
                    type_id: 1,
                    ..Default::default()
                },
                TypeApplication {
                    name: String::from("MoscowMule"),
                    type_id: 2,
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };
        let types = [
            (0, p.clone()),
            (
                1,
                TypeDeclaration {
                    type_id: 1,
                    type_field: String::from("u64"),
                    ..Default::default()
                },
            ),
            (
                2,
                TypeDeclaration {
                    type_id: 2,
                    type_field: String::from("bool"),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let actual = expand_custom_enum(&p, &types)?.to_string();
        let expected = TokenStream::from_str(
            r#"
            # [derive (Clone , Debug , Eq , PartialEq)] pub enum MatchaTea < > { LongIsland (u64) , MoscowMule (bool) } impl < > Parameterize for MatchaTea < > { fn param_type () -> ParamType { let variants = [("LongIsland" . to_string () , < u64 > :: param_type ()) , ("MoscowMule" . to_string () , < bool > :: param_type ())] . to_vec () ; let variants = EnumVariants :: new (variants) . unwrap_or_else (| _ | panic ! ("{} has no variants which isn't allowed!" , "MatchaTea")) ; ParamType :: Enum { name : "MatchaTea" . to_string () , variants , generics : [] . to_vec () } } } impl < > Tokenizable for MatchaTea < > { fn from_token (token : Token) -> Result < Self , SDKError > where Self : Sized , { let gen_err = | msg | { SDKError :: InvalidData (format ! ("Error while instantiating {} from token! {}" , "MatchaTea" , msg)) } ; match token { Token :: Enum (selector) => { let (discriminant , variant_token , _) = * selector ; match discriminant { 0u8 => Ok (Self :: LongIsland (< u64 > :: from_token (variant_token) ?)) , 1u8 => Ok (Self :: MoscowMule (< bool > :: from_token (variant_token) ?)) , _ => Err (gen_err (format ! ("Discriminant {} doesn't point to any of the enums variants." , discriminant))) , } } _ => Err (gen_err (format ! ("Given token ({}) is not of the type Token::Enum!" , token))) , } } fn into_token (self) -> Token { let (discriminant , token) = match self { Self :: LongIsland (inner) => (0u8 , inner . into_token ()) , Self :: MoscowMule (inner) => (1u8 , inner . into_token ()) } ; let variants = match Self :: param_type () { ParamType :: Enum { variants , .. } => variants , other => panic ! ("Calling {}::param_type() must return a ParamType::Enum but instead it returned: {:?}" , "MatchaTea" , other) } ; Token :: Enum (Box :: new ((discriminant , token , variants))) } } impl < > TryFrom < & [u8] > for MatchaTea < > { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl < > TryFrom < & Vec < u8 >> for MatchaTea < > { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } } impl < > TryFrom < Vec < u8 >> for MatchaTea < > { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
            "#,
        )?
        .to_string();

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_enum_with_no_variants_cannot_be_constructed() -> anyhow::Result<()> {
        let p = TypeDeclaration {
            type_id: 0,
            type_field: "enum SomeEmptyEnum".to_string(),
            components: Some(vec![]),
            ..Default::default()
        };
        let types = [(0, p.clone())].into_iter().collect::<HashMap<_, _>>();

        let err = expand_custom_enum(&p, &types)
            .err()
            .ok_or_else(|| anyhow!("Was able to construct an enum without variants"))?;

        assert!(
            matches!(err, Error::InvalidData(_)),
            "Expected the error to be of the type 'InvalidData'"
        );

        Ok(())
    }

    #[test]
    fn test_expand_struct_inside_enum() -> Result<(), Error> {
        let inner_struct = TypeApplication {
            name: String::from("Infrastructure"),
            type_id: 1,
            ..Default::default()
        };
        let enum_components = vec![
            inner_struct,
            TypeApplication {
                name: "Service".to_string(),
                type_id: 2,
                ..Default::default()
            },
        ];
        let p = TypeDeclaration {
            type_id: 0,
            type_field: String::from("enum Amsterdam"),
            components: Some(enum_components),
            ..Default::default()
        };

        let types = [
            (0, p.clone()),
            (
                1,
                TypeDeclaration {
                    type_id: 1,
                    type_field: String::from("struct Building"),
                    components: Some(vec![
                        TypeApplication {
                            name: String::from("Rooms"),
                            type_id: 3,
                            ..Default::default()
                        },
                        TypeApplication {
                            name: String::from("Floors"),
                            type_id: 4,
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                },
            ),
            (
                2,
                TypeDeclaration {
                    type_id: 2,
                    type_field: String::from("u32"),
                    ..Default::default()
                },
            ),
            (
                3,
                TypeDeclaration {
                    type_id: 3,
                    type_field: String::from("u8"),
                    ..Default::default()
                },
            ),
            (
                4,
                TypeDeclaration {
                    type_id: 4,
                    type_field: String::from("u16"),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let actual = expand_custom_enum(&p, &types)?.to_string();
        let expected = TokenStream::from_str(
            r#"
            # [derive (Clone , Debug , Eq , PartialEq)] pub enum Amsterdam < > { Infrastructure (Building) , Service (u32) } impl < > Parameterize for Amsterdam < > { fn param_type () -> ParamType { let variants = [("Infrastructure" . to_string () , < Building > :: param_type ()) , ("Service" . to_string () , < u32 > :: param_type ())] . to_vec () ; let variants = EnumVariants :: new (variants) . unwrap_or_else (| _ | panic ! ("{} has no variants which isn't allowed!" , "Amsterdam")) ; ParamType :: Enum { name : "Amsterdam" . to_string () , variants , generics : [] . to_vec () } } } impl < > Tokenizable for Amsterdam < > { fn from_token (token : Token) -> Result < Self , SDKError > where Self : Sized , { let gen_err = | msg | { SDKError :: InvalidData (format ! ("Error while instantiating {} from token! {}" , "Amsterdam" , msg)) } ; match token { Token :: Enum (selector) => { let (discriminant , variant_token , _) = * selector ; match discriminant { 0u8 => Ok (Self :: Infrastructure (< Building > :: from_token (variant_token) ?)) , 1u8 => Ok (Self :: Service (< u32 > :: from_token (variant_token) ?)) , _ => Err (gen_err (format ! ("Discriminant {} doesn't point to any of the enums variants." , discriminant))) , } } _ => Err (gen_err (format ! ("Given token ({}) is not of the type Token::Enum!" , token))) , } } fn into_token (self) -> Token { let (discriminant , token) = match self { Self :: Infrastructure (inner) => (0u8 , inner . into_token ()) , Self :: Service (inner) => (1u8 , inner . into_token ()) } ; let variants = match Self :: param_type () { ParamType :: Enum { variants , .. } => variants , other => panic ! ("Calling {}::param_type() must return a ParamType::Enum but instead it returned: {:?}" , "Amsterdam" , other) } ; Token :: Enum (Box :: new ((discriminant , token , variants))) } } impl < > TryFrom < & [u8] > for Amsterdam < > { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl < > TryFrom < & Vec < u8 >> for Amsterdam < > { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } } impl < > TryFrom < Vec < u8 >> for Amsterdam < > { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
            "#,
        )?.to_string();

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_expand_array_inside_enum() -> Result<(), Error> {
        let enum_components = vec![TypeApplication {
            name: "SomeArr".to_string(),
            type_id: 1,
            ..Default::default()
        }];
        let p = TypeDeclaration {
            type_id: 0,
            type_field: String::from("enum SomeEnum"),
            components: Some(enum_components),
            ..Default::default()
        };
        let types = [
            (0, p.clone()),
            (
                1,
                TypeDeclaration {
                    type_id: 1,
                    type_field: "[u64; 7]".to_string(),
                    components: Some(vec![TypeApplication {
                        type_id: 2,
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
            ),
            (
                2,
                TypeDeclaration {
                    type_id: 2,
                    type_field: "u64".to_string(),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let actual = expand_custom_enum(&p, &types)?.to_string();
        let expected = TokenStream::from_str(
            r#"
            # [derive (Clone , Debug , Eq , PartialEq)] pub enum SomeEnum < > { SomeArr ([u64 ; 7usize]) } impl < > Parameterize for SomeEnum < > { fn param_type () -> ParamType { let variants = [("SomeArr" . to_string () , < [u64 ; 7usize] > :: param_type ())] . to_vec () ; let variants = EnumVariants :: new (variants) . unwrap_or_else (| _ | panic ! ("{} has no variants which isn't allowed!" , "SomeEnum")) ; ParamType :: Enum { name : "SomeEnum" . to_string () , variants , generics : [] . to_vec () } } } impl < > Tokenizable for SomeEnum < > { fn from_token (token : Token) -> Result < Self , SDKError > where Self : Sized , { let gen_err = | msg | { SDKError :: InvalidData (format ! ("Error while instantiating {} from token! {}" , "SomeEnum" , msg)) } ; match token { Token :: Enum (selector) => { let (discriminant , variant_token , _) = * selector ; match discriminant { 0u8 => Ok (Self :: SomeArr (< [u64 ; 7usize] > :: from_token (variant_token) ?)) , _ => Err (gen_err (format ! ("Discriminant {} doesn't point to any of the enums variants." , discriminant))) , } } _ => Err (gen_err (format ! ("Given token ({}) is not of the type Token::Enum!" , token))) , } } fn into_token (self) -> Token { let (discriminant , token) = match self { Self :: SomeArr (inner) => (0u8 , inner . into_token ()) } ; let variants = match Self :: param_type () { ParamType :: Enum { variants , .. } => variants , other => panic ! ("Calling {}::param_type() must return a ParamType::Enum but instead it returned: {:?}" , "SomeEnum" , other) } ; Token :: Enum (Box :: new ((discriminant , token , variants))) } } impl < > TryFrom < & [u8] > for SomeEnum < > { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl < > TryFrom < & Vec < u8 >> for SomeEnum < > { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } } impl < > TryFrom < Vec < u8 >> for SomeEnum < > { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
            "#,
        )?.to_string();

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_expand_custom_enum_with_enum() -> Result<(), Error> {
        let p = TypeDeclaration {
            type_id: 3,
            type_field: String::from("enum EnumLevel3"),
            components: Some(vec![TypeApplication {
                name: String::from("El2"),
                type_id: 2,
                ..Default::default()
            }]),
            ..Default::default()
        };
        let types = [
            (3, p.clone()),
            (
                2,
                TypeDeclaration {
                    type_id: 2,
                    type_field: String::from("enum EnumLevel2"),
                    components: Some(vec![TypeApplication {
                        name: String::from("El1"),
                        type_id: 1,
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
            ),
            (
                1,
                TypeDeclaration {
                    type_id: 1,
                    type_field: String::from("enum EnumLevel1"),
                    components: Some(vec![TypeApplication {
                        name: String::from("Num"),
                        type_id: 0,
                        ..Default::default()
                    }]),
                    ..Default::default()
                },
            ),
            (
                0,
                TypeDeclaration {
                    type_id: 0,
                    type_field: String::from("u32"),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let actual = expand_custom_enum(&p, &types)?.to_string();
        let expected = TokenStream::from_str(
            r#"
            # [derive (Clone , Debug , Eq , PartialEq)] pub enum EnumLevel3 < > { El2 (EnumLevel2) } impl < > Parameterize for EnumLevel3 < > { fn param_type () -> ParamType { let variants = [("El2" . to_string () , < EnumLevel2 > :: param_type ())] . to_vec () ; let variants = EnumVariants :: new (variants) . unwrap_or_else (| _ | panic ! ("{} has no variants which isn't allowed!" , "EnumLevel3")) ; ParamType :: Enum { name : "EnumLevel3" . to_string () , variants , generics : [] . to_vec () } } } impl < > Tokenizable for EnumLevel3 < > { fn from_token (token : Token) -> Result < Self , SDKError > where Self : Sized , { let gen_err = | msg | { SDKError :: InvalidData (format ! ("Error while instantiating {} from token! {}" , "EnumLevel3" , msg)) } ; match token { Token :: Enum (selector) => { let (discriminant , variant_token , _) = * selector ; match discriminant { 0u8 => Ok (Self :: El2 (< EnumLevel2 > :: from_token (variant_token) ?)) , _ => Err (gen_err (format ! ("Discriminant {} doesn't point to any of the enums variants." , discriminant))) , } } _ => Err (gen_err (format ! ("Given token ({}) is not of the type Token::Enum!" , token))) , } } fn into_token (self) -> Token { let (discriminant , token) = match self { Self :: El2 (inner) => (0u8 , inner . into_token ()) } ; let variants = match Self :: param_type () { ParamType :: Enum { variants , .. } => variants , other => panic ! ("Calling {}::param_type() must return a ParamType::Enum but instead it returned: {:?}" , "EnumLevel3" , other) } ; Token :: Enum (Box :: new ((discriminant , token , variants))) } } impl < > TryFrom < & [u8] > for EnumLevel3 < > { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl < > TryFrom < & Vec < u8 >> for EnumLevel3 < > { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } } impl < > TryFrom < Vec < u8 >> for EnumLevel3 < > { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
            "#,
        )?.to_string();

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_expand_custom_struct() -> Result<(), Error> {
        let p = TypeDeclaration {
            type_field: String::from("struct Cocktail"),
            components: Some(vec![
                TypeApplication {
                    name: String::from("long_island"),
                    type_id: 1,
                    ..Default::default()
                },
                TypeApplication {
                    name: String::from("cosmopolitan"),
                    type_id: 2,
                    ..Default::default()
                },
                TypeApplication {
                    name: String::from("mojito"),
                    type_id: 3,
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };
        let types = [
            (0, p.clone()),
            (
                1,
                TypeDeclaration {
                    type_id: 1,
                    type_field: String::from("bool"),
                    ..Default::default()
                },
            ),
            (
                2,
                TypeDeclaration {
                    type_id: 2,
                    type_field: String::from("u64"),
                    ..Default::default()
                },
            ),
            (
                3,
                TypeDeclaration {
                    type_id: 3,
                    type_field: String::from("u32"),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let actual = expand_custom_struct(&p, &types)?.to_string();
        let expected = TokenStream::from_str(
            r#"
            # [derive (Clone , Debug , Eq , PartialEq)] pub struct Cocktail < > { pub long_island : bool , pub cosmopolitan : u64 , pub mojito : u32 } impl < > Parameterize for Cocktail < > { fn param_type () -> ParamType { let types = [("long_island" . to_string () , < bool > :: param_type ()) , ("cosmopolitan" . to_string () , < u64 > :: param_type ()) , ("mojito" . to_string () , < u32 > :: param_type ())] . to_vec () ; ParamType :: Struct { name : "Cocktail" . to_string () , fields : types , generics : [] . to_vec () } } } impl < > Tokenizable for Cocktail < > { fn into_token (self) -> Token { let tokens = [self . long_island . into_token () , self . cosmopolitan . into_token () , self . mojito . into_token ()] . to_vec () ; Token :: Struct (tokens) } fn from_token (token : Token) -> Result < Self , SDKError > { match token { Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { SDKError :: InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "Cocktail")) }) } ; Ok (Self { long_island : < bool > :: from_token (next_token () ?) ? , cosmopolitan : < u64 > :: from_token (next_token () ?) ? , mojito : < u32 > :: from_token (next_token () ?) ? , }) } , other => Err (SDKError :: InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "Cocktail" , other))) , } } } impl < > TryFrom < & [u8] > for Cocktail < > { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl < > TryFrom < & Vec < u8 >> for Cocktail < > { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } } impl < > TryFrom < Vec < u8 >> for Cocktail < > { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
            "#,
        )?.to_string();

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_struct_with_no_fields_can_be_constructed() -> Result<(), Error> {
        let p = TypeDeclaration {
            type_id: 0,
            type_field: "struct SomeEmptyStruct".to_string(),
            components: Some(vec![]),
            ..Default::default()
        };
        let types = [(0, p.clone())].into_iter().collect::<HashMap<_, _>>();

        let actual = expand_custom_struct(&p, &types)?.to_string();

        let expected = TokenStream::from_str(
            r#"
            # [derive (Clone , Debug , Eq , PartialEq)] pub struct SomeEmptyStruct < > { } impl < > Parameterize for SomeEmptyStruct < > { fn param_type () -> ParamType { let types = [] . to_vec () ; ParamType :: Struct { name : "SomeEmptyStruct" . to_string () , fields : types , generics : [] . to_vec () } } } impl < > Tokenizable for SomeEmptyStruct < > { fn into_token (self) -> Token { let tokens = [] . to_vec () ; Token :: Struct (tokens) } fn from_token (token : Token) -> Result < Self , SDKError > { match token { Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { SDKError :: InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "SomeEmptyStruct")) }) } ; Ok (Self { }) } , other => Err (SDKError :: InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "SomeEmptyStruct" , other))) , } } } impl < > TryFrom < & [u8] > for SomeEmptyStruct < > { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl < > TryFrom < & Vec < u8 >> for SomeEmptyStruct < > { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } } impl < > TryFrom < Vec < u8 >> for SomeEmptyStruct < > { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
            "#,
        )?.to_string();

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_expand_custom_struct_with_struct() -> Result<(), Error> {
        let p = TypeDeclaration {
            type_id: 0,
            type_field: String::from("struct Cocktail"),
            components: Some(vec![
                TypeApplication {
                    name: String::from("long_island"),
                    type_id: 1,
                    ..Default::default()
                },
                TypeApplication {
                    name: String::from("mojito"),
                    type_id: 4,
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };
        let types = [
            (0, p.clone()),
            (
                1,
                TypeDeclaration {
                    type_id: 1,
                    type_field: String::from("struct Shaker"),
                    components: Some(vec![
                        TypeApplication {
                            name: String::from("cosmopolitan"),
                            type_id: 2,
                            ..Default::default()
                        },
                        TypeApplication {
                            name: String::from("bimbap"),
                            type_id: 3,
                            ..Default::default()
                        },
                    ]),
                    ..Default::default()
                },
            ),
            (
                2,
                TypeDeclaration {
                    type_id: 2,
                    type_field: String::from("bool"),
                    ..Default::default()
                },
            ),
            (
                3,
                TypeDeclaration {
                    type_id: 3,
                    type_field: String::from("u64"),
                    ..Default::default()
                },
            ),
            (
                4,
                TypeDeclaration {
                    type_id: 4,
                    type_field: String::from("u32"),
                    ..Default::default()
                },
            ),
        ]
        .into_iter()
        .collect::<HashMap<_, _>>();
        let actual = expand_custom_struct(&p, &types)?.to_string();
        let expected = TokenStream::from_str(
            r#"
            # [derive (Clone , Debug , Eq , PartialEq)] pub struct Cocktail < > { pub long_island : Shaker , pub mojito : u32 } impl < > Parameterize for Cocktail < > { fn param_type () -> ParamType { let types = [("long_island" . to_string () , < Shaker > :: param_type ()) , ("mojito" . to_string () , < u32 > :: param_type ())] . to_vec () ; ParamType :: Struct { name : "Cocktail" . to_string () , fields : types , generics : [] . to_vec () } } } impl < > Tokenizable for Cocktail < > { fn into_token (self) -> Token { let tokens = [self . long_island . into_token () , self . mojito . into_token ()] . to_vec () ; Token :: Struct (tokens) } fn from_token (token : Token) -> Result < Self , SDKError > { match token { Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { SDKError :: InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "Cocktail")) }) } ; Ok (Self { long_island : < Shaker > :: from_token (next_token () ?) ? , mojito : < u32 > :: from_token (next_token () ?) ? , }) } , other => Err (SDKError :: InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "Cocktail" , other))) , } } } impl < > TryFrom < & [u8] > for Cocktail < > { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl < > TryFrom < & Vec < u8 >> for Cocktail < > { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } } impl < > TryFrom < Vec < u8 >> for Cocktail < > { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
            "#,
        )?.to_string();

        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn test_expand_struct_new_abi() -> Result<(), Error> {
        let s = r#"
            {
                "types": [
                  {
                    "typeId": 6,
                    "type": "u64",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 8,
                    "type": "b256",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 6,
                    "type": "u64",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 8,
                    "type": "b256",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 10,
                    "type": "bool",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 12,
                    "type": "struct MyStruct1",
                    "components": [
                      {
                        "name": "x",
                        "type": 6,
                        "typeArguments": null
                      },
                      {
                        "name": "y",
                        "type": 8,
                        "typeArguments": null
                      }
                    ],
                    "typeParameters": null
                  },
                  {
                    "typeId": 6,
                    "type": "u64",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 8,
                    "type": "b256",
                    "components": null,
                    "typeParameters": null
                  },
                  {
                    "typeId": 2,
                    "type": "struct MyStruct1",
                    "components": [
                      {
                        "name": "x",
                        "type": 6,
                        "typeArguments": null
                      },
                      {
                        "name": "y",
                        "type": 8,
                        "typeArguments": null
                      }
                    ],
                    "typeParameters": null
                  },
                  {
                    "typeId": 3,
                    "type": "struct MyStruct2",
                    "components": [
                      {
                        "name": "x",
                        "type": 10,
                        "typeArguments": null
                      },
                      {
                        "name": "y",
                        "type": 12,
                        "typeArguments": []
                      }
                    ],
                    "typeParameters": null
                  },
                  {
                    "typeId": 26,
                    "type": "struct MyStruct1",
                    "components": [
                      {
                        "name": "x",
                        "type": 6,
                        "typeArguments": null
                      },
                      {
                        "name": "y",
                        "type": 8,
                        "typeArguments": null
                      }
                    ],
                    "typeParameters": null
                  }
                ],
                "functions": [
                  {
                    "type": "function",
                    "inputs": [
                      {
                        "name": "s1",
                        "type": 2,
                        "typeArguments": []
                      },
                      {
                        "name": "s2",
                        "type": 3,
                        "typeArguments": []
                      }
                    ],
                    "name": "some_abi_funct",
                    "output": {
                      "name": "",
                      "type": 26,
                      "typeArguments": []
                    }
                  }
                ]
              }
    "#;
        let parsed_abi: ProgramABI = serde_json::from_str(s)?;
        let all_types = parsed_abi
            .types
            .into_iter()
            .map(|t| (t.type_id, t))
            .collect::<HashMap<usize, TypeDeclaration>>();

        let s1 = all_types.get(&2).unwrap();

        let actual = expand_custom_struct(s1, &all_types)?.to_string();

        let expected = TokenStream::from_str(
            r#"
            # [derive (Clone , Debug , Eq , PartialEq)] pub struct MyStruct1 < > { pub x : u64 , pub y : Bits256 } impl < > Parameterize for MyStruct1 < > { fn param_type () -> ParamType { let types = [("x" . to_string () , < u64 > :: param_type ()) , ("y" . to_string () , < Bits256 > :: param_type ())] . to_vec () ; ParamType :: Struct { name : "MyStruct1" . to_string () , fields : types , generics : [] . to_vec () } } } impl < > Tokenizable for MyStruct1 < > { fn into_token (self) -> Token { let tokens = [self . x . into_token () , self . y . into_token ()] . to_vec () ; Token :: Struct (tokens) } fn from_token (token : Token) -> Result < Self , SDKError > { match token { Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { SDKError :: InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "MyStruct1")) }) } ; Ok (Self { x : < u64 > :: from_token (next_token () ?) ? , y : < Bits256 > :: from_token (next_token () ?) ? , }) } , other => Err (SDKError :: InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "MyStruct1" , other))) , } } } impl < > TryFrom < & [u8] > for MyStruct1 < > { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl < > TryFrom < & Vec < u8 >> for MyStruct1 < > { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } } impl < > TryFrom < Vec < u8 >> for MyStruct1 < > { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
            "#,
            )?.to_string();

        assert_eq!(actual, expected);

        let s2 = all_types.get(&3).unwrap();

        let actual = expand_custom_struct(s2, &all_types)?.to_string();

        let expected = TokenStream::from_str(
            r#"
            # [derive (Clone , Debug , Eq , PartialEq)] pub struct MyStruct2 < > { pub x : bool , pub y : MyStruct1 } impl < > Parameterize for MyStruct2 < > { fn param_type () -> ParamType { let types = [("x" . to_string () , < bool > :: param_type ()) , ("y" . to_string () , < MyStruct1 > :: param_type ())] . to_vec () ; ParamType :: Struct { name : "MyStruct2" . to_string () , fields : types , generics : [] . to_vec () } } } impl < > Tokenizable for MyStruct2 < > { fn into_token (self) -> Token { let tokens = [self . x . into_token () , self . y . into_token ()] . to_vec () ; Token :: Struct (tokens) } fn from_token (token : Token) -> Result < Self , SDKError > { match token { Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { SDKError :: InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "MyStruct2")) }) } ; Ok (Self { x : < bool > :: from_token (next_token () ?) ? , y : < MyStruct1 > :: from_token (next_token () ?) ? , }) } , other => Err (SDKError :: InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "MyStruct2" , other))) , } } } impl < > TryFrom < & [u8] > for MyStruct2 < > { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl < > TryFrom < & Vec < u8 >> for MyStruct2 < > { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } } impl < > TryFrom < Vec < u8 >> for MyStruct2 < > { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
            "#,
            )?.to_string();

        assert_eq!(actual, expected);

        Ok(())
    }
}
