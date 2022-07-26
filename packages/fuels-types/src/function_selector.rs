use crate::errors::Error;
use crate::utils::has_array_format;
use crate::Property;

pub struct FunctionSelector {
    pub signature: String,
    pub encoded_selector: String,
    pub final_selector: Vec<u8>,
}

/// Builds a string representation of a function selector,
/// i.e: <fn_name>(<type_1>, <type_2>, ..., <type_n>)
pub fn build_fn_selector(fn_name: &str, params: &[Property]) -> Result<String, Error> {
    let fn_selector = fn_name.to_owned();

    let mut result: String = format!("{}(", fn_selector);

    for (idx, param) in params.iter().enumerate() {
        result.push_str(&build_fn_selector_params(param));
        if idx + 1 < params.len() {
            result.push(',');
        }
    }

    result.push(')');

    Ok(result)
}

fn build_fn_selector_params(prop: &Property) -> String {
    let mut result: String = String::new();

    if prop.is_custom_type() {
        // Custom type, need to break down inner fields.
        // Will return `"e(field_1,field_2,...,field_n)"` if the type is an `Enum`,
        // `"s(field_1,field_2,...,field_n)"` if the type is a `Struct`,
        // `"a[type;length]"` if the type is an `Array`,
        // `(type_1,type_2,...,type_n)` if the type is a `Tuple`.
        if prop.is_struct_type() {
            result.push_str("s(");
        } else if prop.is_enum_type() {
            result.push_str("e(");
        } else if prop.has_custom_type_in_array() {
            result.push_str("a[");
        } else if prop.has_custom_type_in_tuple() {
            result.push('(');
        } else {
            panic!("unexpected custom type");
        }

        for (idx, component) in prop
            .components
            .as_ref()
            .expect("No components found")
            .iter()
            .enumerate()
        {
            result.push_str(&build_fn_selector_params(component));

            if idx + 1 < prop.components.as_ref().unwrap().len() {
                result.push(',');
            }
        }

        if result.starts_with("a[") {
            let array_type_field = prop.type_field.clone();

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
        } else {
            result.push(')');
        }
    } else {
        // Not a custom type.
        let param_str_no_whitespace: String = prop
            .type_field
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();

        // Check if the parameter is an array.
        if has_array_format(&param_str_no_whitespace) {
            // The representation of an array in a function selector should be `a[<type>;<length>]`.
            // Because this is coming in as `[<type>;<length>]` (not prefixed with an 'a'), here
            // we must prefix it with an 'a' so the function selector will be properly encoded.
            let array = format!("{}{}", "a", param_str_no_whitespace);
            result.push_str(array.as_str());
        } else {
            result.push_str(&param_str_no_whitespace);
        }
    }
    result
}
