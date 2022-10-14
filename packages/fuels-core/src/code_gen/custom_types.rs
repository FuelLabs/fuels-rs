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
    // TODO: Move tests using the old abigen to the new one.
    // Currently, they will be skipped. Even though we're not fully testing these at
    // unit level, they're tested at integration level, in the main harness.rs file.

    // #[test]
    // fn test_extract_custom_type_name_from_abi_property_bad_data() {
    //     let p: Property = Default::default();
    //     let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Enum));
    //     assert!(matches!(result, Err(Error::InvalidData(_))));

    //     let p = Property {
    //         name: String::from("foo"),
    //         type_field: String::from("nowhitespacehere"),
    //         components: None,
    //     };
    //     let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Enum));
    //     assert!(matches!(result, Err(Error::InvalidData(_))));
    // }

    // #[test]
    // fn test_extract_struct_name_from_abi_property_wrong_type() {
    //     let p = Property {
    //         name: String::from("foo"),
    //         type_field: String::from("enum something"),
    //         components: None,
    //     };
    //     let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Struct));
    //     assert!(matches!(result, Err(Error::InvalidType(_))));

    //     let p = Property {
    //         name: String::from("foo"),
    //         type_field: String::from("struct somethingelse"),
    //         components: None,
    //     };
    //     let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Enum));
    //     assert!(matches!(result, Err(Error::InvalidType(_))));
    // }

    // #[test]
    // fn test_extract_custom_type_name_from_abi_property() -> Result<(), Error> {
    //     let p = Property {
    //         name: String::from("foo"),
    //         type_field: String::from("struct bar"),
    //         components: None,
    //     };
    //     let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Struct));
    //     assert_eq!(result?, "bar");

    //     let p = Property {
    //         name: String::from("foo"),
    //         type_field: String::from("enum bar"),
    //         components: None,
    //     };
    //     let result = extract_custom_type_name_from_abi_property(&p, Some(CustomType::Enum));
    //     assert_eq!(result?, "bar");
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_custom_enum() -> Result<(), Error> {
    //     let p = Property {
    //         name: String::from("unused"),
    //         type_field: String::from("unused"),
    //         components: Some(vec![
    //             Property {
    //                 name: String::from("LongIsland"),
    //                 type_field: String::from("u64"),
    //                 components: None,
    //             },
    //             Property {
    //                 name: String::from("MoscowMule"),
    //                 type_field: String::from("bool"),
    //                 components: None,
    //             },
    //         ]),
    //     };
    //     let actual = expand_custom_enum("MatchaTea", &p)?.to_string();
    //     let expected = TokenStream::from_str(
    //         r#"
    //         # [derive (Clone , Debug , Eq , PartialEq)] pub enum MatchaTea { LongIsland (u64) , MoscowMule (bool) } impl Parameterize for MatchaTea { fn param_type () -> ParamType { let mut types = Vec :: new () ; types . push (ParamType :: U64) ; types . push (ParamType :: Bool) ; let variants = EnumVariants :: new (types) . expect (concat ! ("Enum " , "MatchaTea" , " has no variants! 'abigen!' should not have succeeded!")) ; ParamType :: Enum (variants) } } impl Tokenizable for MatchaTea { fn into_token (self) -> Token { let (dis , tok) = match self { MatchaTea :: LongIsland (value) => (0u8 , Token :: U64 (value)) , MatchaTea :: MoscowMule (value) => (1u8 , Token :: Bool (value)) , } ; let variants = match Self :: param_type () { ParamType :: Enum (variants) => variants , other => panic ! ("Calling ::param_type() on a custom enum must return a ParamType::Enum but instead it returned: {}" , other) } ; let selector = (dis , tok , variants) ; Token :: Enum (Box :: new (selector)) } fn from_token (token : Token) -> Result < Self , SDKError > { if let Token :: Enum (enum_selector) = token { match * enum_selector { (0u8 , token , _) => Ok (MatchaTea :: LongIsland (< u64 > :: from_token (token) ?)) , (1u8 , token , _) => Ok (MatchaTea :: MoscowMule (< bool > :: from_token (token) ?)) , (_ , _ , _) => Err (SDKError :: InstantiationError (format ! ("Could not construct '{}'. Failed to match with discriminant selector {:?}" , "MatchaTea" , enum_selector))) } } else { Err (SDKError :: InstantiationError (format ! ("Could not construct '{}'. Expected a token of type Token::Enum, got {:?}" , "MatchaTea" , token))) } } } impl TryFrom < & [u8] > for MatchaTea { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < & Vec < u8 >> for MatchaTea { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < Vec < u8 >> for MatchaTea { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
    //         "#,
    //     )?.to_string();

    //     assert_eq!(actual, expected);
    //     Ok(())
    // }

    // #[test]
    // fn top_lvl_enum_w_no_variants_cannot_be_constructed() -> anyhow::Result<()> {
    //     assert_enum_cannot_be_constructed_from(Some(vec![]))?;
    //     assert_enum_cannot_be_constructed_from(None)?;
    //     Ok(())
    // }
    // #[test]
    // fn nested_enum_w_no_variants_cannot_be_constructed() -> anyhow::Result<()> {
    //     let nested_enum_w_components = |components| {
    //         Some(vec![Property {
    //             name: "SomeEmptyEnum".to_string(),
    //             type_field: "enum SomeEmptyEnum".to_string(),
    //             components,
    //         }])
    //     };

    //     assert_enum_cannot_be_constructed_from(nested_enum_w_components(None))?;
    //     assert_enum_cannot_be_constructed_from(nested_enum_w_components(Some(vec![])))?;

    //     Ok(())
    // }

    // fn assert_enum_cannot_be_constructed_from(
    //     components: Option<Vec<Property>>,
    // ) -> anyhow::Result<()> {
    //     let property = Property {
    //         components,
    //         ..Property::default()
    //     };

    //     let err = expand_custom_enum("TheEmptyEnum", &property)
    //         .err()
    //         .ok_or_else(|| anyhow!("Was able to construct an enum without variants"))?;

    //     assert!(
    //         matches!(err, Error::InvalidType(_)),
    //         "Expected the error to be of the type 'InvalidType'"
    //     );

    //     Ok(())
    // }

    // #[test]
    // fn test_expand_struct_inside_enum() -> Result<(), Error> {
    //     let inner_struct = Property {
    //         name: String::from("Infrastructure"),
    //         type_field: String::from("struct Building"),
    //         components: Some(vec![
    //             Property {
    //                 name: String::from("Rooms"),
    //                 type_field: String::from("u8"),
    //                 components: None,
    //             },
    //             Property {
    //                 name: String::from("Floors"),
    //                 type_field: String::from("u16"),
    //                 components: None,
    //             },
    //         ]),
    //     };
    //     let enum_components = vec![
    //         inner_struct,
    //         Property {
    //             name: "Service".to_string(),
    //             type_field: "u32".to_string(),
    //             components: None,
    //         },
    //     ];
    //     let p = Property {
    //         name: String::from("CityComponent"),
    //         type_field: String::from("enum CityComponent"),
    //         components: Some(enum_components),
    //     };
    //     let actual = expand_custom_enum("Amsterdam", &p)?.to_string();
    //     let expected = TokenStream::from_str(
    //         r#"
    //         # [derive (Clone , Debug , Eq , PartialEq)] pub enum Amsterdam { Infrastructure (Building) , Service (u32) } impl Parameterize for Amsterdam { fn param_type () -> ParamType { let mut types = Vec :: new () ; types . push (Building :: param_type ()) ; types . push (ParamType :: U32) ; let variants = EnumVariants :: new (types) . expect (concat ! ("Enum " , "Amsterdam" , " has no variants! 'abigen!' should not have succeeded!")) ; ParamType :: Enum (variants) } } impl Tokenizable for Amsterdam { fn into_token (self) -> Token { let (dis , tok) = match self { Amsterdam :: Infrastructure (inner_struct) => (0u8 , inner_struct . into_token ()) , Amsterdam :: Service (value) => (1u8 , Token :: U32 (value)) , } ; let variants = match Self :: param_type () { ParamType :: Enum (variants) => variants , other => panic ! ("Calling ::param_type() on a custom enum must return a ParamType::Enum but instead it returned: {}" , other) } ; let selector = (dis , tok , variants) ; Token :: Enum (Box :: new (selector)) } fn from_token (token : Token) -> Result < Self , SDKError > { if let Token :: Enum (enum_selector) = token { match * enum_selector { (0u8 , token , _) => { let variant_content = < Building > :: from_token (token) ? ; Ok (Amsterdam :: Infrastructure (variant_content)) } (1u8 , token , _) => Ok (Amsterdam :: Service (< u32 > :: from_token (token) ?)) , (_ , _ , _) => Err (SDKError :: InstantiationError (format ! ("Could not construct '{}'. Failed to match with discriminant selector {:?}" , "Amsterdam" , enum_selector))) } } else { Err (SDKError :: InstantiationError (format ! ("Could not construct '{}'. Expected a token of type Token::Enum, got {:?}" , "Amsterdam" , token))) } } } impl TryFrom < & [u8] > for Amsterdam { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < & Vec < u8 >> for Amsterdam { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < Vec < u8 >> for Amsterdam { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
    //         "#,
    //     )?.to_string();

    //     assert_eq!(actual, expected);
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_array_inside_enum() -> Result<(), Error> {
    //     let enum_components = vec![Property {
    //         name: "SomeArr".to_string(),
    //         type_field: "[u64; 7]".to_string(),
    //         components: None,
    //     }];
    //     let p = Property {
    //         name: String::from("unused"),
    //         type_field: String::from("unused"),
    //         components: Some(enum_components),
    //     };
    //     let actual = expand_custom_enum("SomeEnum", &p)?.to_string();
    //     let expected = TokenStream::from_str(
    //         r#"
    //         # [derive (Clone , Debug , Eq , PartialEq)] pub enum SomeEnum { SomeArr (:: std :: vec :: Vec < u64 >) } impl Parameterize for SomeEnum { fn param_type () -> ParamType { let mut types = Vec :: new () ; types . push (ParamType :: Array (Box :: new (ParamType :: U64) , 7)) ; let variants = EnumVariants :: new (types) . expect (concat ! ("Enum " , "SomeEnum" , " has no variants! 'abigen!' should not have succeeded!")) ; ParamType :: Enum (variants) } } impl Tokenizable for SomeEnum { fn into_token (self) -> Token { let (dis , tok) = match self { SomeEnum :: SomeArr (value) => (0u8 , Token :: Array (vec ! [value . into_token ()])) , } ; let variants = match Self :: param_type () { ParamType :: Enum (variants) => variants , other => panic ! ("Calling ::param_type() on a custom enum must return a ParamType::Enum but instead it returned: {}" , other) } ; let selector = (dis , tok , variants) ; Token :: Enum (Box :: new (selector)) } fn from_token (token : Token) -> Result < Self , SDKError > { if let Token :: Enum (enum_selector) = token { match * enum_selector { (0u8 , token , _) => Ok (SomeEnum :: SomeArr (< :: std :: vec :: Vec < u64 > > :: from_token (token) ?)) , (_ , _ , _) => Err (SDKError :: InstantiationError (format ! ("Could not construct '{}'. Failed to match with discriminant selector {:?}" , "SomeEnum" , enum_selector))) } } else { Err (SDKError :: InstantiationError (format ! ("Could not construct '{}'. Expected a token of type Token::Enum, got {:?}" , "SomeEnum" , token))) } } } impl TryFrom < & [u8] > for SomeEnum { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < & Vec < u8 >> for SomeEnum { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < Vec < u8 >> for SomeEnum { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
    //         "#,
    //     )?.to_string();

    //     assert_eq!(actual, expected);
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_custom_enum_with_enum() -> Result<(), Error> {
    //     let p = Property {
    //         name: String::from("unused"),
    //         type_field: String::from("unused"),
    //         components: Some(vec![Property {
    //             name: String::from("El2"),
    //             type_field: String::from("enum EnumLevel2"),
    //             components: Some(vec![Property {
    //                 name: String::from("El1"),
    //                 type_field: String::from("enum EnumLevel1"),
    //                 components: Some(vec![Property {
    //                     name: String::from("Num"),
    //                     type_field: String::from("u32"),
    //                     components: None,
    //                 }]),
    //             }]),
    //         }]),
    //     };
    //     let actual = expand_custom_enum("EnumLevel3", &p)?.to_string();
    //     let expected = TokenStream::from_str(
    //         r#"
    //         # [derive (Clone , Debug , Eq , PartialEq)] pub enum EnumLevel3 { El2 (EnumLevel2) } impl Parameterize for EnumLevel3 { fn param_type () -> ParamType { let mut types = Vec :: new () ; types . push (EnumLevel2 :: param_type ()) ; let variants = EnumVariants :: new (types) . expect (concat ! ("Enum " , "EnumLevel3" , " has no variants! 'abigen!' should not have succeeded!")) ; ParamType :: Enum (variants) } } impl Tokenizable for EnumLevel3 { fn into_token (self) -> Token { let (dis , tok) = match self { EnumLevel3 :: El2 (inner_enum) => (0u8 , inner_enum . into_token ()) , } ; let variants = match Self :: param_type () { ParamType :: Enum (variants) => variants , other => panic ! ("Calling ::param_type() on a custom enum must return a ParamType::Enum but instead it returned: {}" , other) } ; let selector = (dis , tok , variants) ; Token :: Enum (Box :: new (selector)) } fn from_token (token : Token) -> Result < Self , SDKError > { if let Token :: Enum (enum_selector) = token { match * enum_selector { (0u8 , token , _) => { let variant_content = < EnumLevel2 > :: from_token (token) ? ; Ok (EnumLevel3 :: El2 (variant_content)) } (_ , _ , _) => Err (SDKError :: InstantiationError (format ! ("Could not construct '{}'. Failed to match with discriminant selector {:?}" , "EnumLevel3" , enum_selector))) } } else { Err (SDKError :: InstantiationError (format ! ("Could not construct '{}'. Expected a token of type Token::Enum, got {:?}" , "EnumLevel3" , token))) } } } impl TryFrom < & [u8] > for EnumLevel3 { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < & Vec < u8 >> for EnumLevel3 { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < Vec < u8 >> for EnumLevel3 { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
    //         "#,
    //     )?.to_string();

    //     assert_eq!(actual, expected);
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_custom_struct() -> Result<(), Error> {
    //     let p = Property {
    //         name: String::from("unused"),
    //         type_field: String::from("struct Cocktail"),
    //         components: Some(vec![
    //             Property {
    //                 name: String::from("long_island"),
    //                 type_field: String::from("bool"),
    //                 components: None,
    //             },
    //             Property {
    //                 name: String::from("cosmopolitan"),
    //                 type_field: String::from("u64"),
    //                 components: None,
    //             },
    //             Property {
    //                 name: String::from("mojito"),
    //                 type_field: String::from("u32"),
    //                 components: None,
    //             },
    //         ]),
    //     };
    //     let actual = expand_custom_struct(&p)?.to_string();
    //     let expected = TokenStream::from_str(
    //         r#"
    //         # [derive (Clone , Debug , Eq , PartialEq)] pub struct Cocktail { pub long_island : bool , pub cosmopolitan : u64 , pub mojito : u32 } impl Parameterize for Cocktail { fn param_type () -> ParamType { let mut types = Vec :: new () ; types . push (ParamType :: Bool) ; types . push (ParamType :: U64) ; types . push (ParamType :: U32) ; ParamType :: Struct (types) } } impl Tokenizable for Cocktail { fn into_token (self) -> Token { let mut tokens = Vec :: new () ; tokens . push (Token :: Bool (self . long_island)) ; tokens . push (Token :: U64 (self . cosmopolitan)) ; tokens . push (Token :: U32 (self . mojito)) ; Token :: Struct (tokens) } fn from_token (token : Token) -> Result < Self , SDKError > { match token { Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { SDKError :: InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "Cocktail")) }) } ; Ok (Self { long_island : < bool > :: from_token (next_token () ?) ? , cosmopolitan : < u64 > :: from_token (next_token () ?) ? , mojito : < u32 > :: from_token (next_token () ?) ? }) } , other => Err (SDKError :: InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "Cocktail" , other))) , } } } impl TryFrom < & [u8] > for Cocktail { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < & Vec < u8 >> for Cocktail { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < Vec < u8 >> for Cocktail { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
    //         "#,
    //     )?.to_string();

    //     assert_eq!(actual, expected);
    //     Ok(())
    // }

    // #[test]
    // fn test_expand_custom_struct_with_struct() -> Result<(), Error> {
    //     let p = Property {
    //         name: String::from("unused"),
    //         type_field: String::from("struct Cocktail"),
    //         components: Some(vec![
    //             Property {
    //                 name: String::from("long_island"),
    //                 type_field: String::from("struct Shaker"),
    //                 components: Some(vec![
    //                     Property {
    //                         name: String::from("cosmopolitan"),
    //                         type_field: String::from("bool"),
    //                         components: None,
    //                     },
    //                     Property {
    //                         name: String::from("bimbap"),
    //                         type_field: String::from("u64"),
    //                         components: None,
    //                     },
    //                 ]),
    //             },
    //             Property {
    //                 name: String::from("mojito"),
    //                 type_field: String::from("u32"),
    //                 components: None,
    //             },
    //         ]),
    //     };
    //     let actual = expand_custom_struct(&p)?.to_string();
    //     let expected = TokenStream::from_str(
    //         r#"
    //         # [derive (Clone , Debug , Eq , PartialEq)] pub struct Cocktail { pub long_island : Shaker , pub mojito : u32 } impl Parameterize for Cocktail { fn param_type () -> ParamType { let mut types = Vec :: new () ; types . push (Shaker :: param_type ()) ; types . push (ParamType :: U32) ; ParamType :: Struct (types) } } impl Tokenizable for Cocktail { fn into_token (self) -> Token { let mut tokens = Vec :: new () ; tokens . push (self . long_island . into_token ()) ; tokens . push (Token :: U32 (self . mojito)) ; Token :: Struct (tokens) } fn from_token (token : Token) -> Result < Self , SDKError > { match token { Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { SDKError :: InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "Cocktail")) }) } ; Ok (Self { long_island : Shaker :: from_token (next_token () ?) ? , mojito : < u32 > :: from_token (next_token () ?) ? }) } , other => Err (SDKError :: InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "Cocktail" , other))) , } } } impl TryFrom < & [u8] > for Cocktail { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < & Vec < u8 >> for Cocktail { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < Vec < u8 >> for Cocktail { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
    //         "#,
    //     )?.to_string();

    //     assert_eq!(actual, expected);
    //     Ok(())
    // }

    // TODO: FIX ME
    //     #[test]
    //     fn test_expand_struct_new_abi() -> Result<(), Error> {
    //         let s = r#"
    //         {
    //             "types": [
    //               {
    //                 "typeId": 6,
    //                 "type": "u64",
    //                 "components": null,
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 8,
    //                 "type": "b256",
    //                 "components": null,
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 6,
    //                 "type": "u64",
    //                 "components": null,
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 8,
    //                 "type": "b256",
    //                 "components": null,
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 10,
    //                 "type": "bool",
    //                 "components": null,
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 12,
    //                 "type": "struct MyStruct1",
    //                 "components": [
    //                   {
    //                     "name": "x",
    //                     "type": 6,
    //                     "typeArguments": null
    //                   },
    //                   {
    //                     "name": "y",
    //                     "type": 8,
    //                     "typeArguments": null
    //                   }
    //                 ],
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 6,
    //                 "type": "u64",
    //                 "components": null,
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 8,
    //                 "type": "b256",
    //                 "components": null,
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 2,
    //                 "type": "struct MyStruct1",
    //                 "components": [
    //                   {
    //                     "name": "x",
    //                     "type": 6,
    //                     "typeArguments": null
    //                   },
    //                   {
    //                     "name": "y",
    //                     "type": 8,
    //                     "typeArguments": null
    //                   }
    //                 ],
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 3,
    //                 "type": "struct MyStruct2",
    //                 "components": [
    //                   {
    //                     "name": "x",
    //                     "type": 10,
    //                     "typeArguments": null
    //                   },
    //                   {
    //                     "name": "y",
    //                     "type": 12,
    //                     "typeArguments": []
    //                   }
    //                 ],
    //                 "typeParameters": null
    //               },
    //               {
    //                 "typeId": 26,
    //                 "type": "struct MyStruct1",
    //                 "components": [
    //                   {
    //                     "name": "x",
    //                     "type": 6,
    //                     "typeArguments": null
    //                   },
    //                   {
    //                     "name": "y",
    //                     "type": 8,
    //                     "typeArguments": null
    //                   }
    //                 ],
    //                 "typeParameters": null
    //               }
    //             ],
    //             "functions": [
    //               {
    //                 "type": "function",
    //                 "inputs": [
    //                   {
    //                     "name": "s1",
    //                     "type": 2,
    //                     "typeArguments": []
    //                   },
    //                   {
    //                     "name": "s2",
    //                     "type": 3,
    //                     "typeArguments": []
    //                   }
    //                 ],
    //                 "name": "some_abi_funct",
    //                 "output": {
    //                   "name": "",
    //                   "type": 26,
    //                   "typeArguments": []
    //                 }
    //               }
    //             ]
    //           }
    // "#;
    //         let parsed_abi: ProgramABI = serde_json::from_str(s)?;
    //         let all_types = parsed_abi
    //             .types
    //             .into_iter()
    //             .map(|t| (t.type_id, t))
    //             .collect::<HashMap<usize, TypeDeclaration>>();
    //
    //         let s1 = all_types.get(&2).unwrap();
    //
    //         let actual = expand_custom_struct(s1, &all_types)?.to_string();
    //
    //         let expected = TokenStream::from_str(
    //             r#"
    //             # [derive (Clone , Debug , Eq , PartialEq)] pub struct MyStruct1 { pub x : u64 , pub y : [u8 ; 32] } impl Parameterize for MyStruct1 { fn param_type () -> ParamType { let mut types = Vec :: new () ; types . push (ParamType :: U64) ; types . push (ParamType :: B256) ; ParamType :: Struct (types) } } impl Tokenizable for MyStruct1 { fn into_token (self) -> Token { let mut tokens = Vec :: new () ; tokens . push (Token :: U64 (self . x)) ; tokens . push (Token :: B256 (self . y)) ; Token :: Struct (tokens) } fn from_token (token : Token) -> Result < Self , SDKError > { match token { Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { SDKError :: InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "MyStruct1")) }) } ; Ok (Self { x : < u64 > :: from_token (next_token () ?) ? , y : < [u8 ; 32] > :: from_token (next_token () ?) ? }) } , other => Err (SDKError :: InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "MyStruct1" , other))) , } } } impl TryFrom < & [u8] > for MyStruct1 { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < & Vec < u8 >> for MyStruct1 { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < Vec < u8 >> for MyStruct1 { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
    //             "#,
    //         )?.to_string();
    //
    //         assert_eq!(actual, expected);
    //
    //         let s2 = all_types.get(&3).unwrap();
    //
    //         let actual = expand_custom_struct(s2, &all_types)?.to_string();
    //
    //         let expected = TokenStream::from_str(
    //             r#"
    //             # [derive (Clone , Debug , Eq , PartialEq)] pub struct MyStruct2 { pub x : bool , pub y : MyStruct1 } impl Parameterize for MyStruct2 { fn param_type () -> ParamType { let mut types = Vec :: new () ; types . push (ParamType :: Bool) ; types . push (MyStruct1 :: param_type ()) ; ParamType :: Struct (types) } } impl Tokenizable for MyStruct2 { fn into_token (self) -> Token { let mut tokens = Vec :: new () ; tokens . push (Token :: Bool (self . x)) ; tokens . push (self . y . into_token ()) ; Token :: Struct (tokens) } fn from_token (token : Token) -> Result < Self , SDKError > { match token { Token :: Struct (tokens) => { let mut tokens_iter = tokens . into_iter () ; let mut next_token = move || { tokens_iter . next () . ok_or_else (|| { SDKError :: InstantiationError (format ! ("Ran out of tokens before '{}' has finished construction!" , "MyStruct2")) }) } ; Ok (Self { x : < bool > :: from_token (next_token () ?) ? , y : MyStruct1 :: from_token (next_token () ?) ? }) } , other => Err (SDKError :: InstantiationError (format ! ("Error while constructing '{}'. Expected token of type Token::Struct, got {:?}" , "MyStruct2" , other))) , } } } impl TryFrom < & [u8] > for MyStruct2 { type Error = SDKError ; fn try_from (bytes : & [u8]) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < & Vec < u8 >> for MyStruct2 { type Error = SDKError ; fn try_from (bytes : & Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (bytes) } } impl TryFrom < Vec < u8 >> for MyStruct2 { type Error = SDKError ; fn try_from (bytes : Vec < u8 >) -> Result < Self , Self :: Error > { try_from_bytes (& bytes) } }
    //             "#,
    //         )?.to_string();
    //
    //         assert_eq!(actual, expected);
    //
    //         Ok(())
    //     }
}
