use std::{collections::HashMap, fmt, iter::zip};

use fuel_abi_types::{
    abi::program::{TypeApplication, TypeDeclaration},
    utils::{extract_array_len, extract_generic_name, extract_str_len, has_tuple_format},
};
use itertools::{chain, Itertools};

use crate::{
    constants::WORD_SIZE,
    types::{
        enum_variants::EnumVariants,
        errors::{error, Error, Result},
    },
};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ParamType {
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    Bool,
    B256,
    Unit,
    Array(Box<ParamType>, usize),
    Vector(Box<ParamType>),
    StringSlice,
    StringArray(usize),
    Struct {
        fields: Vec<ParamType>,
        generics: Vec<ParamType>,
    },
    Enum {
        variants: EnumVariants,
        generics: Vec<ParamType>,
    },
    Tuple(Vec<ParamType>),
    RawSlice,
    Bytes,
    String,
}

pub enum ReturnLocation {
    Return,
    ReturnData,
}

impl ParamType {
    // Depending on the type, the returned value will be stored
    // either in `Return` or `ReturnData`.
    pub fn get_return_location(&self) -> ReturnLocation {
        match self {
            Self::Unit | Self::U8 | Self::U16 | Self::U32 | Self::U64 | Self::Bool => {
                ReturnLocation::Return
            }

            _ => ReturnLocation::ReturnData,
        }
    }

    /// Given a [ParamType], return the number of elements of that [ParamType] that can fit in
    /// `available_bytes`: it is the length of the corresponding heap type.
    pub fn calculate_num_of_elements(
        param_type: &ParamType,
        available_bytes: usize,
    ) -> Result<usize> {
        let encoding_width = param_type.compute_encoding_width()?;
        let memory_size = encoding_width
            .checked_mul(WORD_SIZE)
            .ok_or_else(|| error!(InvalidData, "Overflow error while encoding {param_type:?}"))?;
        if memory_size == 0 {
            return Err(error!(
                InvalidType,
                "Cannot calculate the number of elements because the type is zero-sized."
            ));
        }
        let remainder = available_bytes % memory_size;
        if remainder != 0 {
            return Err(error!(
                InvalidData,
                "{remainder} extra bytes detected while decoding heap type"
            ));
        }
        let num_of_elements = available_bytes
            .checked_div(memory_size)
            .ok_or_else(|| error!(InvalidData, "Type {param_type:?} has a memory_size of 0"))?;
        Ok(num_of_elements)
    }

    pub fn children_need_extra_receipts(&self) -> bool {
        match self {
            ParamType::Array(inner, _) | ParamType::Vector(inner) => {
                inner.is_extra_receipt_needed(false)
            }
            ParamType::Struct { fields, .. } => fields
                .iter()
                .any(|param_type| param_type.is_extra_receipt_needed(false)),
            ParamType::Enum { variants, .. } => variants
                .param_types()
                .iter()
                .any(|param_type| param_type.is_extra_receipt_needed(false)),
            ParamType::Tuple(inner_types) => inner_types
                .iter()
                .any(|param_type| param_type.is_extra_receipt_needed(false)),
            _ => false,
        }
    }

    pub fn validate_is_decodable(&self, max_depth: usize) -> Result<()> {
        match self {
            ParamType::Enum { variants, .. } => {
                let all_param_types = variants.param_types();
                let grandchildren_need_receipts = all_param_types
                    .iter()
                    .any(|child| child.children_need_extra_receipts());
                if grandchildren_need_receipts {
                    return Err(error!(
                        InvalidType,
                        "Enums currently support only one level deep heap types."
                    ));
                }

                let num_of_children_needing_receipts = all_param_types
                    .iter()
                    .filter(|param_type| param_type.is_extra_receipt_needed(false))
                    .count();
                if num_of_children_needing_receipts > 1 {
                    Err(error!(
                        InvalidType,
                        "Enums currently support only one heap-type variant. Found: \
                        {num_of_children_needing_receipts}"
                    ))
                } else {
                    Ok(())
                }
            }
            _ if self.children_need_extra_receipts() => Err(error!(
                InvalidType,
                "type {:?} is not decodable: nested heap types are currently not supported except in Enums.",
                DebugWithDepth::new(self, max_depth)
            )),
            _ => Ok(()),
        }
    }

    pub fn is_extra_receipt_needed(&self, top_level_type: bool) -> bool {
        match self {
            ParamType::Vector(_) | ParamType::Bytes | ParamType::String => true,
            ParamType::Array(inner, _) => inner.is_extra_receipt_needed(false),
            ParamType::Struct { fields, generics } => {
                chain!(fields, generics).any(|param_type| param_type.is_extra_receipt_needed(false))
            }
            ParamType::Enum { variants, generics } => chain!(variants.param_types(), generics)
                .any(|param_type| param_type.is_extra_receipt_needed(false)),
            ParamType::Tuple(elements) => elements
                .iter()
                .any(|param_type| param_type.is_extra_receipt_needed(false)),
            ParamType::RawSlice | ParamType::StringSlice => !top_level_type,
            _ => false,
        }
    }

    /// Compute the inner memory size of a containing heap type (`Bytes` or `Vec`s).
    pub fn heap_inner_element_size(&self, top_level_type: bool) -> Result<Option<usize>> {
        match &self {
            ParamType::Vector(inner_param_type) => {
                let width = inner_param_type.compute_encoding_width()?;
                width
                    .checked_mul(WORD_SIZE)
                    .map(Some)
                    .ok_or_else(|| error!(InvalidData, "overflow while multiplying"))
            }
            // `Bytes` type is byte-packed in the VM, so it's the size of an u8
            ParamType::Bytes | ParamType::String => Ok(Some(std::mem::size_of::<u8>())),
            ParamType::RawSlice if !top_level_type => {
                ParamType::U64.compute_encoding_width().map(Some)
            }
            ParamType::StringSlice if !top_level_type => Ok(Some(std::mem::size_of::<u8>())),
            _ => Ok(None),
        }
    }

    /// Calculates the number of `WORD`s the VM expects this parameter to be encoded in.
    pub fn compute_encoding_width(&self) -> Result<usize> {
        const fn count_words(bytes: usize) -> usize {
            let q = bytes / WORD_SIZE;
            let r = bytes % WORD_SIZE;
            match r == 0 {
                true => q,
                false => q + 1,
            }
        }

        match &self {
            ParamType::Unit
            | ParamType::U8
            | ParamType::U16
            | ParamType::U32
            | ParamType::U64
            | ParamType::Bool => Ok(1),
            ParamType::U128 | ParamType::RawSlice | ParamType::StringSlice => Ok(2),
            ParamType::Vector(_) | ParamType::Bytes | ParamType::String => Ok(3),
            ParamType::U256 | ParamType::B256 => Ok(4),
            ParamType::Array(param, count) => param
                .compute_encoding_width()?
                .checked_mul(*count)
                .ok_or_else(|| {
                    error!(
                        InvalidData,
                        "overflow while calculating encoding width for Array({param:?}, {count})"
                    )
                }),
            ParamType::StringArray(len) => Ok(count_words(*len)),
            ParamType::Struct { fields, .. } => fields
                .iter()
                .map(|param_type| param_type.compute_encoding_width())
                .process_results(|iter| iter.sum()),
            ParamType::Enum { variants, .. } => variants.compute_encoding_width_of_enum(),
            ParamType::Tuple(params) => params
                .iter()
                .map(|param_type| param_type.compute_encoding_width())
                .process_results(|iter| iter.sum()),
        }
    }

    /// For when you need to convert a ABI JSON's TypeApplication into a ParamType.
    ///
    /// # Arguments
    ///
    /// * `type_application`: The TypeApplication you wish to convert into a ParamType
    /// * `type_lookup`: A HashMap of TypeDeclarations mentioned in the
    ///                  TypeApplication where the type id is the key.
    pub fn try_from_type_application(
        type_application: &TypeApplication,
        type_lookup: &HashMap<usize, TypeDeclaration>,
    ) -> Result<Self> {
        Type::try_from(type_application, type_lookup)?.try_into()
    }
}

#[derive(Debug, Clone)]
struct Type {
    type_field: String,
    generic_params: Vec<Type>,
    components: Vec<Type>,
}

impl Type {
    /// Will recursively drill down the given generic parameters until all types are
    /// resolved.
    ///
    /// # Arguments
    ///
    /// * `type_application`: the type we wish to resolve
    /// * `types`: all types used in the function call
    pub fn try_from(
        type_application: &TypeApplication,
        type_lookup: &HashMap<usize, TypeDeclaration>,
    ) -> Result<Self> {
        Self::resolve(type_application, type_lookup, &[])
    }

    fn resolve(
        type_application: &TypeApplication,
        type_lookup: &HashMap<usize, TypeDeclaration>,
        parent_generic_params: &[(usize, Type)],
    ) -> Result<Self> {
        let type_declaration = type_lookup.get(&type_application.type_id).ok_or_else(|| {
            error!(
                InvalidData,
                "type id {} not found in type lookup", type_application.type_id
            )
        })?;

        if extract_generic_name(&type_declaration.type_field).is_some() {
            let (_, generic_type) = parent_generic_params
                .iter()
                .find(|(id, _)| *id == type_application.type_id)
                .ok_or_else(|| {
                    error!(
                        InvalidData,
                        "type id {} not found in parent's generic parameters",
                        type_application.type_id
                    )
                })?;

            return Ok(generic_type.clone());
        }

        // Figure out what does the current type do with the inherited generic
        // parameters and reestablish the mapping since the current type might have
        // renamed the inherited generic parameters.
        let generic_params_lookup = Self::determine_generics_for_type(
            type_application,
            type_lookup,
            type_declaration,
            parent_generic_params,
        )?;

        // Resolve the enclosed components (if any) with the newly resolved generic
        // parameters.
        let components = type_declaration
            .components
            .iter()
            .flatten()
            .map(|component| Self::resolve(component, type_lookup, &generic_params_lookup))
            .collect::<Result<Vec<_>>>()?;

        Ok(Type {
            type_field: type_declaration.type_field.clone(),
            components,
            generic_params: generic_params_lookup
                .into_iter()
                .map(|(_, ty)| ty)
                .collect(),
        })
    }

    /// For the given type generates generic_type_id -> Type mapping describing to
    /// which types generic parameters should be resolved.
    ///
    /// # Arguments
    ///
    /// * `type_application`: The type on which the generic parameters are defined.
    /// * `types`: All types used.
    /// * `parent_generic_params`: The generic parameters as inherited from the
    ///                            enclosing type (a struct/enum/array etc.).
    fn determine_generics_for_type(
        type_application: &TypeApplication,
        type_lookup: &HashMap<usize, TypeDeclaration>,
        type_declaration: &TypeDeclaration,
        parent_generic_params: &[(usize, Type)],
    ) -> Result<Vec<(usize, Self)>> {
        match &type_declaration.type_parameters {
            // The presence of type_parameters indicates that the current type
            // (a struct or an enum) defines some generic parameters (i.e. SomeStruct<T, K>).
            Some(params) if !params.is_empty() => {
                // Determine what Types the generics will resolve to.
                let generic_params_from_current_type = type_application
                    .type_arguments
                    .iter()
                    .flatten()
                    .map(|ty| Self::resolve(ty, type_lookup, parent_generic_params))
                    .collect::<Result<Vec<_>>>()?;

                let generics_to_use = if !generic_params_from_current_type.is_empty() {
                    generic_params_from_current_type
                } else {
                    // Types such as arrays and enums inherit and forward their
                    // generic parameters, without declaring their own.
                    parent_generic_params
                        .iter()
                        .map(|(_, ty)| ty)
                        .cloned()
                        .collect()
                };

                // All inherited but unused generic types are dropped. The rest are
                // re-mapped to new type_ids since child types are free to rename
                // the generic parameters as they see fit -- i.e.
                // struct ParentStruct<T>{
                //     b: ChildStruct<T>
                // }
                // struct ChildStruct<K> {
                //     c: K
                // }

                Ok(zip(params.clone(), generics_to_use).collect())
            }
            _ => Ok(parent_generic_params.to_vec()),
        }
    }
}

impl TryFrom<Type> for ParamType {
    type Error = Error;

    fn try_from(value: Type) -> Result<Self> {
        (&value).try_into()
    }
}

impl TryFrom<&Type> for ParamType {
    type Error = Error;

    fn try_from(the_type: &Type) -> Result<Self> {
        let matched_param_type = [
            try_primitive,
            try_array,
            try_str_array,
            try_str_slice,
            try_tuple,
            try_vector,
            try_bytes,
            try_std_string,
            try_raw_slice,
            try_enum,
            try_u128,
            try_u256,
            try_struct,
        ]
        .into_iter()
        .map(|fun| fun(the_type))
        .flat_map(|result| result.ok().flatten())
        .next();

        matched_param_type.map(Ok).unwrap_or_else(|| {
            Err(error!(
                InvalidType,
                "Type {} couldn't be converted into a ParamType", the_type.type_field
            ))
        })
    }
}

fn convert_into_param_types(coll: &[Type]) -> Result<Vec<ParamType>> {
    coll.iter().map(ParamType::try_from).collect()
}

fn try_struct(the_type: &Type) -> Result<Option<ParamType>> {
    let result = if has_struct_format(&the_type.type_field) {
        let generics = param_types(&the_type.generic_params)?;

        let fields = convert_into_param_types(&the_type.components)?;
        Some(ParamType::Struct { fields, generics })
    } else {
        None
    };

    Ok(result)
}

fn has_struct_format(field: &str) -> bool {
    field.starts_with("struct ")
}

fn try_vector(the_type: &Type) -> Result<Option<ParamType>> {
    if !["struct std::vec::Vec", "struct Vec"].contains(&the_type.type_field.as_str()) {
        return Ok(None);
    }

    if the_type.generic_params.len() != 1 {
        return Err(error!(
            InvalidType,
            "Vec must have exactly one generic argument for its type. Found: {:?}",
            the_type.generic_params
        ));
    }

    let vec_elem_type = convert_into_param_types(&the_type.generic_params)?.remove(0);

    Ok(Some(ParamType::Vector(Box::new(vec_elem_type))))
}

fn try_u128(the_type: &Type) -> Result<Option<ParamType>> {
    Ok(["struct std::u128::U128", "struct U128"]
        .contains(&the_type.type_field.as_str())
        .then_some(ParamType::U128))
}

fn try_u256(the_type: &Type) -> Result<Option<ParamType>> {
    Ok(["struct std::u256::U256", "struct U256"]
        .contains(&the_type.type_field.as_str())
        .then_some(ParamType::U256))
}

fn try_bytes(the_type: &Type) -> Result<Option<ParamType>> {
    Ok(["struct std::bytes::Bytes", "struct Bytes"]
        .contains(&the_type.type_field.as_str())
        .then_some(ParamType::Bytes))
}

fn try_std_string(the_type: &Type) -> Result<Option<ParamType>> {
    Ok(["struct std::string::String", "struct String"]
        .contains(&the_type.type_field.as_str())
        .then_some(ParamType::String))
}

fn try_raw_slice(the_type: &Type) -> Result<Option<ParamType>> {
    Ok((the_type.type_field == "raw untyped slice").then_some(ParamType::RawSlice))
}

fn try_enum(the_type: &Type) -> Result<Option<ParamType>> {
    let field = &the_type.type_field;
    let result = if field.starts_with("enum ") {
        let generics = param_types(&the_type.generic_params)?;

        let components = convert_into_param_types(&the_type.components)?;
        let variants = EnumVariants::new(components)?;

        Some(ParamType::Enum { variants, generics })
    } else {
        None
    };

    Ok(result)
}

fn try_tuple(the_type: &Type) -> Result<Option<ParamType>> {
    let result = if has_tuple_format(&the_type.type_field) {
        let tuple_elements = param_types(&the_type.components)?;
        Some(ParamType::Tuple(tuple_elements))
    } else {
        None
    };

    Ok(result)
}

fn param_types(coll: &[Type]) -> Result<Vec<ParamType>> {
    coll.iter().map(|e| e.try_into()).collect()
}

fn try_str_array(the_type: &Type) -> Result<Option<ParamType>> {
    Ok(extract_str_len(&the_type.type_field).map(ParamType::StringArray))
}

fn try_str_slice(the_type: &Type) -> Result<Option<ParamType>> {
    Ok(if the_type.type_field == "str" {
        Some(ParamType::StringSlice)
    } else {
        None
    })
}

fn try_array(the_type: &Type) -> Result<Option<ParamType>> {
    if let Some(len) = extract_array_len(&the_type.type_field) {
        return match the_type.components.as_slice() {
            [single_type] => {
                let array_type = single_type.try_into()?;
                Ok(Some(ParamType::Array(Box::new(array_type), len)))
            }
            _ => Err(error!(
                InvalidType,
                "An array must have elements of exactly one type. Array types: {:?}",
                the_type.components
            )),
        };
    }
    Ok(None)
}

fn try_primitive(the_type: &Type) -> Result<Option<ParamType>> {
    let result = match the_type.type_field.as_str() {
        "bool" => Some(ParamType::Bool),
        "u8" => Some(ParamType::U8),
        "u16" => Some(ParamType::U16),
        "u32" => Some(ParamType::U32),
        "u64" => Some(ParamType::U64),
        "b256" => Some(ParamType::B256),
        "()" => Some(ParamType::Unit),
        "str" => Some(ParamType::StringSlice),
        _ => None,
    };

    Ok(result)
}

/// Allows `Debug` formatting of arbitrary-depth nested `ParamTypes` by
/// omitting the details of inner types if max depth is exceeded.
pub(crate) struct DebugWithDepth<'a> {
    param_type: &'a ParamType,
    depth_left: usize,
}

impl<'a> DebugWithDepth<'a> {
    pub(crate) fn new(param_type: &'a ParamType, depth_left: usize) -> Self {
        Self {
            param_type,
            depth_left,
        }
    }

    fn descend(&'a self, param_type: &'a ParamType) -> Self {
        Self {
            param_type,
            depth_left: self.depth_left - 1,
        }
    }
}

impl<'a> fmt::Debug for DebugWithDepth<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.depth_left == 0 {
            return write!(f, "...");
        }

        match &self.param_type {
            ParamType::Array(inner, size) => f
                .debug_tuple("Array")
                .field(&self.descend(inner))
                .field(&size)
                .finish(),
            ParamType::Struct { fields, generics } => f
                .debug_struct("Struct")
                .field(
                    "fields",
                    &fields
                        .iter()
                        .map(|field| self.descend(field))
                        .collect::<Vec<_>>(),
                )
                .field(
                    "generics",
                    &generics
                        .iter()
                        .map(|generic| self.descend(generic))
                        .collect::<Vec<_>>(),
                )
                .finish(),
            ParamType::Enum { variants, generics } => f
                .debug_struct("Enum")
                .field(
                    "variants",
                    &variants
                        .param_types()
                        .iter()
                        .map(|variant| self.descend(variant))
                        .collect::<Vec<_>>(),
                )
                .field(
                    "generics",
                    &generics
                        .iter()
                        .map(|generic| self.descend(generic))
                        .collect::<Vec<_>>(),
                )
                .finish(),
            ParamType::Tuple(inner) => f
                .debug_tuple("Tuple")
                .field(
                    &inner
                        .iter()
                        .map(|param_type| self.descend(param_type))
                        .collect::<Vec<_>>(),
                )
                .finish(),
            ParamType::Vector(inner) => {
                f.debug_tuple("Vector").field(&self.descend(inner)).finish()
            }
            _ => write!(f, "{:?}", self.param_type),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{codec::DecoderConfig, types::param_types::ParamType};

    const WIDTH_OF_B256: usize = 4;
    const WIDTH_OF_U32: usize = 1;
    const WIDTH_OF_BOOL: usize = 1;

    #[test]
    fn array_size_dependent_on_num_of_elements() {
        const NUM_ELEMENTS: usize = 11;
        let param = ParamType::Array(Box::new(ParamType::B256), NUM_ELEMENTS);

        let width = param.compute_encoding_width().unwrap();

        let expected = NUM_ELEMENTS * WIDTH_OF_B256;
        assert_eq!(expected, width);
    }

    #[test]
    fn string_size_dependent_on_num_of_elements() {
        const NUM_ASCII_CHARS: usize = 9;
        let param = ParamType::StringArray(NUM_ASCII_CHARS);

        let width = param.compute_encoding_width().unwrap();

        // 2 WORDS or 16 B are enough to fit 9 ascii chars
        assert_eq!(2, width);
    }

    #[test]
    fn structs_are_just_all_elements_combined() {
        let inner_struct = ParamType::Struct {
            fields: vec![ParamType::U32, ParamType::U32],
            generics: vec![],
        };

        let a_struct = ParamType::Struct {
            fields: vec![ParamType::B256, ParamType::Bool, inner_struct],
            generics: vec![],
        };

        let width = a_struct.compute_encoding_width().unwrap();

        const INNER_STRUCT_WIDTH: usize = WIDTH_OF_U32 * 2;
        const EXPECTED_WIDTH: usize = WIDTH_OF_B256 + WIDTH_OF_BOOL + INNER_STRUCT_WIDTH;
        assert_eq!(EXPECTED_WIDTH, width);
    }

    #[test]
    fn enums_are_as_big_as_their_biggest_variant_plus_a_word() -> Result<()> {
        let fields = vec![ParamType::B256];
        let inner_struct = ParamType::Struct {
            fields,
            generics: vec![],
        };
        let types = vec![ParamType::U32, inner_struct];
        let param = ParamType::Enum {
            variants: EnumVariants::new(types)?,
            generics: vec![],
        };

        let width = param.compute_encoding_width().unwrap();

        const INNER_STRUCT_SIZE: usize = WIDTH_OF_B256;
        const EXPECTED_WIDTH: usize = INNER_STRUCT_SIZE + 1;
        assert_eq!(EXPECTED_WIDTH, width);
        Ok(())
    }

    #[test]
    fn tuples_are_just_all_elements_combined() {
        let inner_tuple = ParamType::Tuple(vec![ParamType::B256]);
        let param = ParamType::Tuple(vec![ParamType::U32, inner_tuple]);

        let width = param.compute_encoding_width().unwrap();

        const INNER_TUPLE_WIDTH: usize = WIDTH_OF_B256;
        const EXPECTED_WIDTH: usize = WIDTH_OF_U32 + INNER_TUPLE_WIDTH;
        assert_eq!(EXPECTED_WIDTH, width);
    }

    #[test]
    fn handles_simple_types() -> Result<()> {
        let parse_param_type = |type_field: &str| {
            let type_application = TypeApplication {
                name: "".to_string(),
                type_id: 0,
                type_arguments: None,
            };

            let declarations = [TypeDeclaration {
                type_id: 0,
                type_field: type_field.to_string(),
                components: None,
                type_parameters: None,
            }];

            let type_lookup = declarations
                .into_iter()
                .map(|decl| (decl.type_id, decl))
                .collect::<HashMap<_, _>>();

            ParamType::try_from_type_application(&type_application, &type_lookup)
        };

        assert_eq!(parse_param_type("u8")?, ParamType::U8);
        assert_eq!(parse_param_type("u16")?, ParamType::U16);
        assert_eq!(parse_param_type("u32")?, ParamType::U32);
        assert_eq!(parse_param_type("u64")?, ParamType::U64);
        assert_eq!(parse_param_type("bool")?, ParamType::Bool);
        assert_eq!(parse_param_type("b256")?, ParamType::B256);
        assert_eq!(parse_param_type("()")?, ParamType::Unit);
        assert_eq!(parse_param_type("str[21]")?, ParamType::StringArray(21));
        assert_eq!(parse_param_type("str")?, ParamType::StringSlice);

        Ok(())
    }

    #[test]
    fn handles_arrays() -> Result<()> {
        // given
        let type_application = TypeApplication {
            name: "".to_string(),
            type_id: 0,
            type_arguments: None,
        };

        let declarations = [
            TypeDeclaration {
                type_id: 0,
                type_field: "[_; 10]".to_string(),
                components: Some(vec![TypeApplication {
                    name: "__array_element".to_string(),
                    type_id: 1,
                    type_arguments: None,
                }]),
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 1,
                type_field: "u8".to_string(),
                components: None,
                type_parameters: None,
            },
        ];

        let type_lookup = declarations
            .into_iter()
            .map(|decl| (decl.type_id, decl))
            .collect::<HashMap<_, _>>();

        // when
        let result = ParamType::try_from_type_application(&type_application, &type_lookup)?;

        // then
        assert_eq!(result, ParamType::Array(Box::new(ParamType::U8), 10));

        Ok(())
    }

    #[test]
    fn handles_vectors() -> Result<()> {
        // given
        let declarations = [
            TypeDeclaration {
                type_id: 1,
                type_field: "generic T".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 2,
                type_field: "raw untyped ptr".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 3,
                type_field: "struct std::vec::RawVec".to_string(),
                components: Some(vec![
                    TypeApplication {
                        name: "ptr".to_string(),
                        type_id: 2,
                        type_arguments: None,
                    },
                    TypeApplication {
                        name: "cap".to_string(),
                        type_id: 5,
                        type_arguments: None,
                    },
                ]),
                type_parameters: Some(vec![1]),
            },
            TypeDeclaration {
                type_id: 4,
                type_field: "struct std::vec::Vec".to_string(),
                components: Some(vec![
                    TypeApplication {
                        name: "buf".to_string(),
                        type_id: 3,
                        type_arguments: Some(vec![TypeApplication {
                            name: "".to_string(),
                            type_id: 1,
                            type_arguments: None,
                        }]),
                    },
                    TypeApplication {
                        name: "len".to_string(),
                        type_id: 5,
                        type_arguments: None,
                    },
                ]),
                type_parameters: Some(vec![1]),
            },
            TypeDeclaration {
                type_id: 5,
                type_field: "u64".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 6,
                type_field: "u8".to_string(),
                components: None,
                type_parameters: None,
            },
        ];

        let type_application = TypeApplication {
            name: "arg".to_string(),
            type_id: 4,
            type_arguments: Some(vec![TypeApplication {
                name: "".to_string(),
                type_id: 6,
                type_arguments: None,
            }]),
        };

        let type_lookup = declarations
            .into_iter()
            .map(|decl| (decl.type_id, decl))
            .collect::<HashMap<_, _>>();

        // when
        let result = ParamType::try_from_type_application(&type_application, &type_lookup)?;

        // then
        assert_eq!(result, ParamType::Vector(Box::new(ParamType::U8)));

        Ok(())
    }

    #[test]
    fn handles_structs() -> Result<()> {
        // given
        let declarations = [
            TypeDeclaration {
                type_id: 1,
                type_field: "generic T".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 2,
                type_field: "struct SomeStruct".to_string(),
                components: Some(vec![TypeApplication {
                    name: "field".to_string(),
                    type_id: 1,
                    type_arguments: None,
                }]),
                type_parameters: Some(vec![1]),
            },
            TypeDeclaration {
                type_id: 3,
                type_field: "u8".to_string(),
                components: None,
                type_parameters: None,
            },
        ];

        let type_application = TypeApplication {
            name: "arg".to_string(),
            type_id: 2,
            type_arguments: Some(vec![TypeApplication {
                name: "".to_string(),
                type_id: 3,
                type_arguments: None,
            }]),
        };

        let type_lookup = declarations
            .into_iter()
            .map(|decl| (decl.type_id, decl))
            .collect::<HashMap<_, _>>();

        // when
        let result = ParamType::try_from_type_application(&type_application, &type_lookup)?;

        // then
        assert_eq!(
            result,
            ParamType::Struct {
                fields: vec![ParamType::U8],
                generics: vec![ParamType::U8]
            }
        );

        Ok(())
    }

    #[test]
    fn handles_enums() -> Result<()> {
        // given
        let declarations = [
            TypeDeclaration {
                type_id: 1,
                type_field: "generic T".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 2,
                type_field: "enum SomeEnum".to_string(),
                components: Some(vec![TypeApplication {
                    name: "variant".to_string(),
                    type_id: 1,
                    type_arguments: None,
                }]),
                type_parameters: Some(vec![1]),
            },
            TypeDeclaration {
                type_id: 3,
                type_field: "u8".to_string(),
                components: None,
                type_parameters: None,
            },
        ];

        let type_application = TypeApplication {
            name: "arg".to_string(),
            type_id: 2,
            type_arguments: Some(vec![TypeApplication {
                name: "".to_string(),
                type_id: 3,
                type_arguments: None,
            }]),
        };

        let type_lookup = declarations
            .into_iter()
            .map(|decl| (decl.type_id, decl))
            .collect::<HashMap<_, _>>();

        // when
        let result = ParamType::try_from_type_application(&type_application, &type_lookup)?;

        // then
        assert_eq!(
            result,
            ParamType::Enum {
                variants: EnumVariants::new(vec![ParamType::U8])?,
                generics: vec![ParamType::U8]
            }
        );

        Ok(())
    }

    #[test]
    fn handles_tuples() -> Result<()> {
        // given
        let declarations = [
            TypeDeclaration {
                type_id: 1,
                type_field: "(_, _)".to_string(),
                components: Some(vec![
                    TypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 3,
                        type_arguments: None,
                    },
                    TypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 2,
                        type_arguments: None,
                    },
                ]),
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 2,
                type_field: "str[15]".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 3,
                type_field: "u8".to_string(),
                components: None,
                type_parameters: None,
            },
        ];

        let type_application = TypeApplication {
            name: "arg".to_string(),
            type_id: 1,
            type_arguments: None,
        };
        let type_lookup = declarations
            .into_iter()
            .map(|decl| (decl.type_id, decl))
            .collect::<HashMap<_, _>>();

        // when
        let result = ParamType::try_from_type_application(&type_application, &type_lookup)?;

        // then
        assert_eq!(
            result,
            ParamType::Tuple(vec![ParamType::U8, ParamType::StringArray(15)])
        );

        Ok(())
    }

    #[test]
    fn ultimate_example() -> Result<()> {
        // given
        let declarations = [
            TypeDeclaration {
                type_id: 1,
                type_field: "(_, _)".to_string(),
                components: Some(vec![
                    TypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 11,
                        type_arguments: None,
                    },
                    TypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 11,
                        type_arguments: None,
                    },
                ]),
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 2,
                type_field: "(_, _)".to_string(),
                components: Some(vec![
                    TypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 4,
                        type_arguments: None,
                    },
                    TypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 24,
                        type_arguments: None,
                    },
                ]),
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 3,
                type_field: "(_, _)".to_string(),
                components: Some(vec![
                    TypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 5,
                        type_arguments: None,
                    },
                    TypeApplication {
                        name: "__tuple_element".to_string(),
                        type_id: 13,
                        type_arguments: None,
                    },
                ]),
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 4,
                type_field: "[_; 1]".to_string(),
                components: Some(vec![TypeApplication {
                    name: "__array_element".to_string(),
                    type_id: 8,
                    type_arguments: Some(vec![TypeApplication {
                        name: "".to_string(),
                        type_id: 22,
                        type_arguments: Some(vec![TypeApplication {
                            name: "".to_string(),
                            type_id: 21,
                            type_arguments: Some(vec![TypeApplication {
                                name: "".to_string(),
                                type_id: 18,
                                type_arguments: Some(vec![TypeApplication {
                                    name: "".to_string(),
                                    type_id: 13,
                                    type_arguments: None,
                                }]),
                            }]),
                        }]),
                    }]),
                }]),
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 5,
                type_field: "[_; 2]".to_string(),
                components: Some(vec![TypeApplication {
                    name: "__array_element".to_string(),
                    type_id: 14,
                    type_arguments: None,
                }]),
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 6,
                type_field: "[_; 2]".to_string(),
                components: Some(vec![TypeApplication {
                    name: "__array_element".to_string(),
                    type_id: 10,
                    type_arguments: None,
                }]),
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 7,
                type_field: "b256".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 8,
                type_field: "enum EnumWGeneric".to_string(),
                components: Some(vec![
                    TypeApplication {
                        name: "a".to_string(),
                        type_id: 25,
                        type_arguments: None,
                    },
                    TypeApplication {
                        name: "b".to_string(),
                        type_id: 12,
                        type_arguments: None,
                    },
                ]),
                type_parameters: Some(vec![12]),
            },
            TypeDeclaration {
                type_id: 9,
                type_field: "generic K".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 10,
                type_field: "generic L".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 11,
                type_field: "generic M".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 12,
                type_field: "generic N".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 13,
                type_field: "generic T".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 14,
                type_field: "generic U".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 15,
                type_field: "raw untyped ptr".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 16,
                type_field: "str[2]".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 17,
                type_field: "struct MegaExample".to_string(),
                components: Some(vec![
                    TypeApplication {
                        name: "a".to_string(),
                        type_id: 3,
                        type_arguments: None,
                    },
                    TypeApplication {
                        name: "b".to_string(),
                        type_id: 23,
                        type_arguments: Some(vec![TypeApplication {
                            name: "".to_string(),
                            type_id: 2,
                            type_arguments: None,
                        }]),
                    },
                ]),
                type_parameters: Some(vec![13, 14]),
            },
            TypeDeclaration {
                type_id: 18,
                type_field: "struct PassTheGenericOn".to_string(),
                components: Some(vec![TypeApplication {
                    name: "one".to_string(),
                    type_id: 20,
                    type_arguments: Some(vec![TypeApplication {
                        name: "".to_string(),
                        type_id: 9,
                        type_arguments: None,
                    }]),
                }]),
                type_parameters: Some(vec![9]),
            },
            TypeDeclaration {
                type_id: 19,
                type_field: "struct std::vec::RawVec".to_string(),
                components: Some(vec![
                    TypeApplication {
                        name: "ptr".to_string(),
                        type_id: 15,
                        type_arguments: None,
                    },
                    TypeApplication {
                        name: "cap".to_string(),
                        type_id: 25,
                        type_arguments: None,
                    },
                ]),
                type_parameters: Some(vec![13]),
            },
            TypeDeclaration {
                type_id: 20,
                type_field: "struct SimpleGeneric".to_string(),
                components: Some(vec![TypeApplication {
                    name: "single_generic_param".to_string(),
                    type_id: 13,
                    type_arguments: None,
                }]),
                type_parameters: Some(vec![13]),
            },
            TypeDeclaration {
                type_id: 21,
                type_field: "struct StructWArrayGeneric".to_string(),
                components: Some(vec![TypeApplication {
                    name: "a".to_string(),
                    type_id: 6,
                    type_arguments: None,
                }]),
                type_parameters: Some(vec![10]),
            },
            TypeDeclaration {
                type_id: 22,
                type_field: "struct StructWTupleGeneric".to_string(),
                components: Some(vec![TypeApplication {
                    name: "a".to_string(),
                    type_id: 1,
                    type_arguments: None,
                }]),
                type_parameters: Some(vec![11]),
            },
            TypeDeclaration {
                type_id: 23,
                type_field: "struct std::vec::Vec".to_string(),
                components: Some(vec![
                    TypeApplication {
                        name: "buf".to_string(),
                        type_id: 19,
                        type_arguments: Some(vec![TypeApplication {
                            name: "".to_string(),
                            type_id: 13,
                            type_arguments: None,
                        }]),
                    },
                    TypeApplication {
                        name: "len".to_string(),
                        type_id: 25,
                        type_arguments: None,
                    },
                ]),
                type_parameters: Some(vec![13]),
            },
            TypeDeclaration {
                type_id: 24,
                type_field: "u32".to_string(),
                components: None,
                type_parameters: None,
            },
            TypeDeclaration {
                type_id: 25,
                type_field: "u64".to_string(),
                components: None,
                type_parameters: None,
            },
        ];

        let type_lookup = declarations
            .into_iter()
            .map(|decl| (decl.type_id, decl))
            .collect::<HashMap<_, _>>();

        let type_application = TypeApplication {
            name: "arg1".to_string(),
            type_id: 17,
            type_arguments: Some(vec![
                TypeApplication {
                    name: "".to_string(),
                    type_id: 16,
                    type_arguments: None,
                },
                TypeApplication {
                    name: "".to_string(),
                    type_id: 7,
                    type_arguments: None,
                },
            ]),
        };

        // when
        let result = ParamType::try_from_type_application(&type_application, &type_lookup)?;

        // then
        let expected_param_type = {
            let fields = vec![ParamType::Struct {
                fields: vec![ParamType::StringArray(2)],
                generics: vec![ParamType::StringArray(2)],
            }];
            let pass_the_generic_on = ParamType::Struct {
                fields,
                generics: vec![ParamType::StringArray(2)],
            };

            let fields = vec![ParamType::Array(Box::from(pass_the_generic_on.clone()), 2)];
            let struct_w_array_generic = ParamType::Struct {
                fields,
                generics: vec![pass_the_generic_on],
            };

            let fields = vec![ParamType::Tuple(vec![
                struct_w_array_generic.clone(),
                struct_w_array_generic.clone(),
            ])];
            let struct_w_tuple_generic = ParamType::Struct {
                fields,
                generics: vec![struct_w_array_generic],
            };

            let types = vec![ParamType::U64, struct_w_tuple_generic.clone()];
            let fields = vec![
                ParamType::Tuple(vec![
                    ParamType::Array(Box::from(ParamType::B256), 2),
                    ParamType::StringArray(2),
                ]),
                ParamType::Vector(Box::from(ParamType::Tuple(vec![
                    ParamType::Array(
                        Box::from(ParamType::Enum {
                            variants: EnumVariants::new(types).unwrap(),
                            generics: vec![struct_w_tuple_generic],
                        }),
                        1,
                    ),
                    ParamType::U32,
                ]))),
            ];
            ParamType::Struct {
                fields,
                generics: vec![ParamType::StringArray(2), ParamType::B256],
            }
        };

        assert_eq!(result, expected_param_type);

        Ok(())
    }

    #[test]
    fn validate_is_decodable_simple_types() -> Result<()> {
        let max_depth = DecoderConfig::default().max_depth;
        assert!(ParamType::U8.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::U16.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::U32.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::U64.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::U128.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::U256.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::Bool.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::B256.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::Unit.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::StringSlice
            .validate_is_decodable(max_depth)
            .is_ok());
        assert!(ParamType::StringArray(10)
            .validate_is_decodable(max_depth)
            .is_ok());
        assert!(ParamType::RawSlice.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::Bytes.validate_is_decodable(max_depth).is_ok());
        assert!(ParamType::String.validate_is_decodable(max_depth).is_ok());
        Ok(())
    }

    #[test]
    fn validate_is_decodable_complex_types_containing_bytes() -> Result<()> {
        let param_types_containing_bytes = vec![ParamType::Bytes, ParamType::U64, ParamType::Bool];
        let param_types_no_bytes = vec![ParamType::U64, ParamType::U32];
        let max_depth = DecoderConfig::default().max_depth;
        let nested_heap_type_error_message = |p: ParamType| {
            format!(
                "Invalid type: type {:?} is not decodable: nested heap types are currently not \
        supported except in Enums.",
                DebugWithDepth::new(&p, max_depth)
            )
        };
        let cannot_be_decoded = |p: ParamType| {
            assert_eq!(
                p.validate_is_decodable(max_depth)
                    .expect_err(&format!("Should not be decodable: {:?}", p))
                    .to_string(),
                nested_heap_type_error_message(p)
            )
        };
        let can_be_decoded = |p: ParamType| p.validate_is_decodable(max_depth).is_ok();

        can_be_decoded(ParamType::Array(Box::new(ParamType::U64), 10usize));
        cannot_be_decoded(ParamType::Array(Box::new(ParamType::Bytes), 10usize));

        can_be_decoded(ParamType::Vector(Box::new(ParamType::U64)));
        cannot_be_decoded(ParamType::Vector(Box::new(ParamType::Bytes)));

        can_be_decoded(ParamType::Struct {
            generics: param_types_no_bytes.clone(),
            fields: param_types_no_bytes.clone(),
        });
        cannot_be_decoded(ParamType::Struct {
            fields: param_types_containing_bytes.clone(),
            generics: param_types_no_bytes.clone(),
        });

        can_be_decoded(ParamType::Tuple(param_types_no_bytes.clone()));
        cannot_be_decoded(ParamType::Tuple(param_types_containing_bytes.clone()));

        Ok(())
    }

    #[test]
    fn validate_is_decodable_enum_containing_bytes() -> Result<()> {
        let max_depth = DecoderConfig::default().max_depth;
        let can_be_decoded = |p: ParamType| p.validate_is_decodable(max_depth).is_ok();
        let param_types_containing_bytes = vec![ParamType::Bytes, ParamType::U64, ParamType::Bool];
        let param_types_no_bytes = vec![ParamType::U64, ParamType::U32];
        let variants_no_bytes_type = EnumVariants::new(param_types_no_bytes.clone())?;
        let variants_one_bytes_type = EnumVariants::new(param_types_containing_bytes.clone())?;
        let variants_two_bytes_type = EnumVariants::new(vec![ParamType::Bytes, ParamType::Bytes])?;
        can_be_decoded(ParamType::Enum {
            variants: variants_no_bytes_type.clone(),
            generics: param_types_no_bytes.clone(),
        });
        can_be_decoded(ParamType::Enum {
            variants: variants_one_bytes_type.clone(),
            generics: param_types_no_bytes.clone(),
        });
        let expected = "Invalid type: Enums currently support only one heap-type variant. Found: 2"
            .to_string();
        assert_eq!(
            ParamType::Enum {
                variants: variants_two_bytes_type.clone(),
                generics: param_types_no_bytes.clone(),
            }
            .validate_is_decodable(max_depth)
            .expect_err("Should not be decodable")
            .to_string(),
            expected
        );
        can_be_decoded(ParamType::Enum {
            variants: variants_no_bytes_type,
            generics: param_types_containing_bytes.clone(),
        });
        can_be_decoded(ParamType::Enum {
            variants: variants_one_bytes_type,
            generics: param_types_containing_bytes.clone(),
        });
        let expected = "Invalid type: Enums currently support only one heap-type variant. Found: 2"
            .to_string();
        assert_eq!(
            ParamType::Enum {
                variants: variants_two_bytes_type.clone(),
                generics: param_types_containing_bytes.clone(),
            }
            .validate_is_decodable(max_depth)
            .expect_err("Should not be decodable")
            .to_string(),
            expected
        );

        Ok(())
    }

    #[test]
    fn validate_is_decodable_complex_types_containing_vector() -> Result<()> {
        let max_depth = DecoderConfig::default().max_depth;
        let param_types_containing_vector = vec![
            ParamType::Vector(Box::new(ParamType::U32)),
            ParamType::U64,
            ParamType::Bool,
        ];
        let param_types_no_vector = vec![ParamType::U64, ParamType::U32];
        let nested_heap_type_error_message = |p: ParamType| {
            format!(
                "Invalid type: type {:?} is not decodable: nested heap types \
        are currently not supported except in Enums.",
                DebugWithDepth::new(&p, max_depth)
            )
        };
        let cannot_be_decoded = |p: ParamType| {
            assert_eq!(
                p.validate_is_decodable(max_depth)
                    .expect_err(&format!("Should not be decodable: {:?}", p))
                    .to_string(),
                nested_heap_type_error_message(p)
            )
        };
        let can_be_decoded = |p: ParamType| p.validate_is_decodable(max_depth).is_ok();

        can_be_decoded(ParamType::Array(Box::new(ParamType::U64), 10usize));
        cannot_be_decoded(ParamType::Array(
            Box::new(ParamType::Vector(Box::new(ParamType::U8))),
            10usize,
        ));

        can_be_decoded(ParamType::Vector(Box::new(ParamType::U64)));
        cannot_be_decoded(ParamType::Vector(Box::new(ParamType::Vector(Box::new(
            ParamType::U8,
        )))));

        can_be_decoded(ParamType::Struct {
            fields: param_types_no_vector.clone(),
            generics: param_types_no_vector.clone(),
        });
        cannot_be_decoded(ParamType::Struct {
            generics: param_types_no_vector.clone(),
            fields: param_types_containing_vector.clone(),
        });

        can_be_decoded(ParamType::Tuple(param_types_no_vector.clone()));
        cannot_be_decoded(ParamType::Tuple(param_types_containing_vector.clone()));

        Ok(())
    }

    #[test]
    fn validate_is_decodable_enum_containing_vector() -> Result<()> {
        let max_depth = DecoderConfig::default().max_depth;
        let can_be_decoded = |p: ParamType| p.validate_is_decodable(max_depth).is_ok();
        let param_types_containing_vector = vec![
            ParamType::Vector(Box::new(ParamType::Bool)),
            ParamType::U64,
            ParamType::Bool,
        ];
        let param_types_no_vector = vec![ParamType::U64, ParamType::U32];
        let variants_no_vector_type = EnumVariants::new(param_types_no_vector.clone())?;
        let variants_one_vector_type = EnumVariants::new(param_types_containing_vector.clone())?;
        let variants_two_vector_type = EnumVariants::new(vec![
            ParamType::Vector(Box::new(ParamType::U8)),
            ParamType::Vector(Box::new(ParamType::U16)),
        ])?;
        can_be_decoded(ParamType::Enum {
            variants: variants_no_vector_type.clone(),
            generics: param_types_no_vector.clone(),
        });
        can_be_decoded(ParamType::Enum {
            variants: variants_one_vector_type.clone(),
            generics: param_types_no_vector.clone(),
        });
        let expected = "Invalid type: Enums currently support only one heap-type variant. Found: 2"
            .to_string();
        assert_eq!(
            ParamType::Enum {
                variants: variants_two_vector_type.clone(),
                generics: param_types_no_vector.clone(),
            }
            .validate_is_decodable(max_depth)
            .expect_err("Should not be decodable")
            .to_string(),
            expected
        );
        can_be_decoded(ParamType::Enum {
            variants: variants_no_vector_type,
            generics: param_types_containing_vector.clone(),
        });
        can_be_decoded(ParamType::Enum {
            variants: variants_one_vector_type,
            generics: param_types_containing_vector.clone(),
        });
        let expected = "Invalid type: Enums currently support only one heap-type variant. Found: 2"
            .to_string();
        assert_eq!(
            ParamType::Enum {
                variants: variants_two_vector_type.clone(),
                generics: param_types_containing_vector.clone(),
            }
            .validate_is_decodable(max_depth)
            .expect_err("Should not be decodable")
            .to_string(),
            expected
        );

        Ok(())
    }
    #[test]
    fn try_vector_is_type_path_backward_compatible() {
        // TODO: To be removed once https://github.com/FuelLabs/fuels-rs/issues/881 is unblocked.
        let the_type = given_generic_type_with_path("Vec");

        let param_type = try_vector(&the_type).unwrap().unwrap();

        assert_eq!(param_type, ParamType::Vector(Box::new(ParamType::U8)));
    }

    #[test]
    fn try_vector_correctly_resolves_param_type() {
        let the_type = given_generic_type_with_path("std::vec::Vec");

        let param_type = try_vector(&the_type).unwrap().unwrap();

        assert_eq!(param_type, ParamType::Vector(Box::new(ParamType::U8)));
    }

    #[test]
    fn try_bytes_is_type_path_backward_compatible() {
        // TODO: To be removed once https://github.com/FuelLabs/fuels-rs/issues/881 is unblocked.
        let the_type = given_type_with_path("Bytes");

        let param_type = try_bytes(&the_type).unwrap().unwrap();

        assert_eq!(param_type, ParamType::Bytes);
    }

    #[test]
    fn try_bytes_correctly_resolves_param_type() {
        let the_type = given_type_with_path("std::bytes::Bytes");

        let param_type = try_bytes(&the_type).unwrap().unwrap();

        assert_eq!(param_type, ParamType::Bytes);
    }

    #[test]
    fn try_raw_slice_correctly_resolves_param_type() {
        let the_type = Type {
            type_field: "raw untyped slice".to_string(),
            generic_params: vec![],
            components: vec![],
        };

        let param_type = try_raw_slice(&the_type).unwrap().unwrap();

        assert_eq!(param_type, ParamType::RawSlice);
    }

    #[test]
    fn try_std_string_correctly_resolves_param_type() {
        let the_type = given_type_with_path("std::string::String");

        let param_type = try_std_string(&the_type).unwrap().unwrap();

        assert_eq!(param_type, ParamType::String);
    }

    #[test]
    fn try_std_string_is_type_path_backward_compatible() {
        // TODO: To be removed once https://github.com/FuelLabs/fuels-rs/issues/881 is unblocked.
        let the_type = given_type_with_path("String");

        let param_type = try_std_string(&the_type).unwrap().unwrap();

        assert_eq!(param_type, ParamType::String);
    }

    fn given_type_with_path(path: &str) -> Type {
        Type {
            type_field: format!("struct {path}"),
            generic_params: vec![],
            components: vec![],
        }
    }

    fn given_generic_type_with_path(path: &str) -> Type {
        Type {
            type_field: format!("struct {path}"),
            generic_params: vec![Type {
                type_field: "u8".to_string(),
                generic_params: vec![],
                components: vec![],
            }],
            components: vec![],
        }
    }
}
