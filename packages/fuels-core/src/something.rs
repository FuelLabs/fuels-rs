use crate::code_gen::custom_types::extract_custom_type_name_from_abi_type_field;
use crate::code_gen::utils::{
    extract_array_len, extract_generic_name, extract_str_len, has_enum_format, has_struct_format,
    has_tuple_format,
};
use crate::utils::ident;
use fuels_types::errors::Error;
use fuels_types::param_types::{EnumVariants, ParamType};
use fuels_types::{TypeApplication, TypeDeclaration};
use itertools::Itertools;
use std::collections::HashMap;
use std::iter::zip;

pub fn determine_param_type(
    type_application: &TypeApplication,
    type_lookup: &HashMap<usize, TypeDeclaration>,
) -> Result<ParamType, Error> {
    Type::from(type_application, type_lookup).try_into()
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
    pub fn from(
        type_application: &TypeApplication,
        type_lookup: &HashMap<usize, TypeDeclaration>,
    ) -> Self {
        Self::resolve(type_application, type_lookup, &[])
    }

    fn resolve(
        type_application: &TypeApplication,
        type_lookup: &HashMap<usize, TypeDeclaration>,
        parent_generic_params: &[(usize, Type)],
    ) -> Self {
        let type_decl = type_lookup.get(&type_application.type_id).unwrap();

        if extract_generic_name(&type_decl.type_field).is_some() {
            let (_, generic_type) = parent_generic_params
                .iter()
                .find(|(id, _)| *id == type_application.type_id)
                .unwrap();

            return generic_type.clone();
        }

        // Figure out what does the current type do with the inherited generic
        // parameters and reestablish the mapping since the current type might have
        // renamed the inherited generic parameters.
        let generic_params_lookup =
            Self::determine_generics_for_type(type_application, type_lookup, parent_generic_params);

        // Resolve the enclosed components (if any) with the newly resolved generic
        // parameters.
        let components = type_decl
            .components
            .iter()
            .flatten()
            .map(|component| Self::resolve(component, type_lookup, &generic_params_lookup))
            .collect_vec();

        Type {
            type_field: type_decl.type_field.clone(),
            components,
            generic_params: generic_params_lookup
                .into_iter()
                .map(|(_, ty)| ty)
                .collect(),
        }
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
        types: &HashMap<usize, TypeDeclaration>,
        parent_generic_params: &[(usize, Type)],
    ) -> Vec<(usize, Self)> {
        let type_decl = types.get(&type_application.type_id).unwrap();
        match &type_decl.type_parameters {
            // The presence of type_parameters indicates that the current type
            // (a struct or an enum) defines some generic parameters (i.e.
            // SomeStruct<T, K>
            Some(params) if !params.is_empty() => {
                // Determine what Types the generics will resolve to.
                let generic_params_from_current_type = type_application
                    .type_arguments
                    .iter()
                    .flatten()
                    .map(|ty| Self::resolve(ty, types, parent_generic_params))
                    .collect_vec();

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

                zip(params.clone(), generics_to_use).collect()
            }
            _ => parent_generic_params.to_vec(),
        }
    }
}

impl TryFrom<Type> for ParamType {
    type Error = Error;

    fn try_from(mut the_type: Type) -> Result<Self, Self::Error> {
        let type_field = the_type.type_field;
        match type_field.as_str() {
            "byte" => return Ok(ParamType::Byte),
            "bool" => return Ok(ParamType::Bool),
            "u8" => return Ok(ParamType::U8),
            "u16" => return Ok(ParamType::U16),
            "u32" => return Ok(ParamType::U32),
            "u64" => return Ok(ParamType::U64),
            "b256" => return Ok(ParamType::B256),
            "()" => return Ok(ParamType::Unit),
            _ => {}
        }

        if let Some(len) = extract_array_len(&type_field) {
            if the_type.components.len() != 1 {
                return Err(Error::InvalidType(format!(
                    "An array must have elements of exactly one type. Array types: {:?}",
                    the_type.components
                )));
            }
            let array_type = the_type.components.remove(0).try_into()?;
            return Ok(ParamType::Array(Box::new(array_type), len));
        }

        if let Some(len) = extract_str_len(&type_field) {
            return Ok(ParamType::String(len));
        }

        let smt = |coll: Vec<Type>| {
            coll.into_iter()
                .map(|e| e.try_into())
                .collect::<Result<Vec<ParamType>, _>>()
        };

        if has_tuple_format(&type_field) {
            let tuple_elements = smt(the_type.components)?;
            return Ok(ParamType::Tuple(tuple_elements));
        }
        if has_enum_format(&type_field) {
            let generics = smt(the_type.generic_params)?;

            let variants = EnumVariants::new(smt(the_type.components)?)?;

            return Ok(ParamType::Enum { variants, generics });
        }
        if has_struct_format(&type_field) {
            let mut generics = smt(the_type.generic_params)?;

            let struct_name = extract_custom_type_name_from_abi_type_field(&type_field)?;
            if struct_name == ident("Vec") {
                if generics.len() != 1 {
                    return Err(Error::InvalidType(format!(
                        "Vec must have exactly one generic argument for its type. Found: {:?}",
                        generics
                    )));
                }
                let vec_elem_type = generics.remove(0);

                return Ok(ParamType::Vector(Box::new(vec_elem_type)));
            }

            let fields = smt(the_type.components)?;
            return Ok(ParamType::Struct { fields, generics });
        }
        Err(Error::InvalidType(
            "Type didn't match against any known ParamTypes".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handles_simple_types() -> Result<(), Error> {
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

            determine_param_type(&type_application, &type_lookup)
        };

        assert_eq!(parse_param_type("u8")?, ParamType::U8);
        assert_eq!(parse_param_type("u16")?, ParamType::U16);
        assert_eq!(parse_param_type("u32")?, ParamType::U32);
        assert_eq!(parse_param_type("u64")?, ParamType::U64);
        assert_eq!(parse_param_type("bool")?, ParamType::Bool);
        assert_eq!(parse_param_type("byte")?, ParamType::Byte);
        assert_eq!(parse_param_type("b256")?, ParamType::B256);
        assert_eq!(parse_param_type("()")?, ParamType::Unit);
        assert_eq!(parse_param_type("str[21]")?, ParamType::String(21));

        Ok(())
    }

    #[test]
    fn handles_arrays() -> Result<(), Error> {
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

        let result = determine_param_type(&type_application, &type_lookup)?;

        assert_eq!(result, ParamType::Array(Box::new(ParamType::U8), 10));

        Ok(())
    }
}
