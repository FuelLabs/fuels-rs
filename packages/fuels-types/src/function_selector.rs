use std::collections::HashMap;

use crate::errors::Error;
use crate::utils::{has_array_format, has_tuple_format};
use crate::TypeDeclaration;

pub struct FunctionSelector {
    pub signature: String,
    pub encoded_selector: String,
    pub final_selector: Vec<u8>,
}

/// Builds a string representation of a function selector,
/// i.e: <fn_name>(<type_1>, <type_2>, ..., <type_n>)
pub fn build_fn_selector(
    fn_name: &str,
    params: &[TypeDeclaration],
    types: &HashMap<usize, TypeDeclaration>,
) -> Result<String, Error> {
    let fn_selector = fn_name.to_owned();

    let mut result: String = format!("{}(", fn_selector);

    for (idx, param) in params.iter().enumerate() {
        result.push_str(&build_fn_selector_params(param, types));
        if idx + 1 < params.len() {
            result.push(',');
        }
    }

    result.push(')');

    Ok(result)
}

fn build_fn_selector_params(
    prop: &TypeDeclaration,
    types: &HashMap<usize, TypeDeclaration>,
) -> String {
    let mut result: String = String::new();

    let type_declaration = types.get(&prop.type_id).expect("couldn't find type");

    if type_declaration.is_custom_type(types) {
        // Custom type, need to break down inner fields.
        // Will return `"e(field_1,field_2,...,field_n)"` if the type is an `Enum`,
        // `"s(field_1,field_2,...,field_n)"` if the type is a `Struct`,
        // `"a[type;length]"` if the type is an `Array`,
        // `(type_1,type_2,...,type_n)` if the type is a `Tuple`.
        if type_declaration.is_struct_type() {
            result.push_str("s(");
        } else if type_declaration.is_enum_type() {
            result.push_str("e(");
        } else if type_declaration.has_custom_type_in_array(types) {
            result.push_str("a[");
        } else if type_declaration.has_custom_type_in_tuple(types) {
            result.push('(');
        } else {
            panic!("unexpected custom type");
        }

        for (idx, component) in type_declaration
            .components
            .as_ref()
            .expect("No components found")
            .iter()
            .enumerate()
        {
            let t = types
                .get(&component.type_field)
                .expect("couldn't find type");

            result.push_str(&build_fn_selector_params(t, types));

            if idx + 1 < type_declaration.components.as_ref().unwrap().len() {
                result.push(',');
            }
        }

        if result.starts_with("a[") {
            let array_type_field = type_declaration.type_field.clone();

            // Type field, in this case, looks like
            // "[struct Person; 2]" and we want to extract the
            // length, which in this example is 2.
            // First, get the last part after `;`: `"<length>]"`.
            let mut array_length = array_type_field.split(';').collect::<Vec<&str>>()[1]
                .trim()
                .to_string();

            array_length.pop(); // Remove the trailing "]"

            // Make sure the length is a valid number.
            let array_length = array_length.parse::<usize>().expect("Invalid array length");

            result.push(';');
            result.push_str(array_length.to_string().as_str());
            result.push(']');
            result
        } else {
            result.push(')');
            result
        }
    } else {
        // Not a custom type.

        // Tuple.
        if has_tuple_format(&type_declaration.type_field) {
            let components = type_declaration
                .components
                .as_ref()
                .expect("tuples should have components");

            // get types of components
            let component_types: Vec<String> = components
                .iter()
                .map(|component| {
                    let t = types
                        .get(&component.type_field)
                        .expect("couldn't find type");
                    t.type_field.clone()
                })
                .collect();

            // join component_types
            let joined = component_types.join(",");

            let tuple_primitive_types_only_signature = format!("{}{}{}", "(", joined, ")");
            return tuple_primitive_types_only_signature;
        }

        // Check if the parameter is an array.
        if has_array_format(&type_declaration.type_field) {
            // The representation of an array in a function selector should be `a[<type>;<length>]`.
            // Because this is coming in as `[<type>;<length>]` (not prefixed with an 'a'), here
            // we must prefix it with an 'a' so the function selector will be properly encoded.

            let array_component_type = types
                .get(&type_declaration.components.as_ref().unwrap()[0].type_field)
                .expect("couldn't find type");

            // Get array size from type field string
            let array_type_field = type_declaration.type_field.clone();
            // Get number in string
            let array_length = array_type_field.split(';').collect::<Vec<&str>>()[1]
                .trim()
                .to_string();

            let array = format!("a[{};{}", array_component_type.type_field, array_length);
            result.push_str(array.as_str());
            result
        } else {
            let param_str_no_whitespace: String = type_declaration
                .type_field
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            result.push_str(&param_str_no_whitespace);
            result
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::TypeApplication;

    use super::*;

    #[test]
    fn test_fn_selector_builder() -> Result<(), Error> {
        let fn_name = "some_abi_fn";
        let fn_params = vec![
            TypeApplication {
                name: "s1".to_string(),
                type_field: 3,
                type_arguments: None,
            },
            TypeApplication {
                name: "s2".to_string(),
                type_field: 4,
                type_arguments: None,
            },
        ];

        let all_types = vec![
            TypeDeclaration {
                type_id: 0,
                type_field: "u64".to_string(),
                type_parameters: None,
                components: None,
            },
            TypeDeclaration {
                type_id: 1,
                type_field: "b256".to_string(),
                type_parameters: None,
                components: None,
            },
            TypeDeclaration {
                type_id: 2,
                type_field: "bool".to_string(),
                type_parameters: None,
                components: None,
            },
            TypeDeclaration {
                type_id: 3,
                type_field: "struct MyStruct1".to_string(),
                type_parameters: None,
                components: Some(vec![
                    TypeApplication {
                        name: "x".to_string(),
                        type_field: 0,
                        type_arguments: None,
                    },
                    TypeApplication {
                        name: "y".to_string(),
                        type_field: 1,
                        type_arguments: None,
                    },
                ]),
            },
            TypeDeclaration {
                type_id: 4,
                type_field: "struct MyStruct2".to_string(),
                type_parameters: None,
                components: Some(vec![
                    TypeApplication {
                        name: "x".to_string(),
                        type_field: 2,
                        type_arguments: None,
                    },
                    TypeApplication {
                        name: "y".to_string(),
                        type_field: 3,
                        type_arguments: None,
                    },
                ]),
            },
        ];

        let all_types = all_types
            .into_iter()
            .map(|t| (t.type_id, t))
            .collect::<HashMap<usize, TypeDeclaration>>();

        let fn_param_types = fn_params
            .iter()
            .map(|t| all_types.get(&t.type_field).unwrap().clone())
            .collect::<Vec<TypeDeclaration>>();

        let result = build_fn_selector(fn_name, &fn_param_types, &all_types).unwrap();

        assert_eq!("some_abi_fn(s(u64,b256),s(bool,s(u64,b256)))", result);

        Ok(())
    }
}
