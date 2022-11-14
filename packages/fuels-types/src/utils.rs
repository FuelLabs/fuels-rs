use crate::errors::Error;
use lazy_static::lazy_static;
use regex::Regex;

pub fn has_array_format(element: &str) -> bool {
    element.starts_with('[') && element.ends_with(']')
}

pub fn has_tuple_format(element: &str) -> bool {
    element.starts_with('(') && element.ends_with(')')
}

pub fn extract_generic_name(field: &str) -> Option<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^\s*generic\s+(\S+)\s*$").unwrap();
    }
    RE.captures(field)
        .map(|captures| String::from(&captures[1]))
}

pub fn extract_array_len(field: &str) -> Option<usize> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^\s*\[.+;\s*(\d+)\s*\]\s*$").unwrap();
    }
    RE.captures(field)
        .map(|captures| captures[1].to_string())
        .map(|length: String| {
            length.parse::<usize>().unwrap_or_else(|_| {
                panic!("Could not extract array length from {length}! Original field {field}")
            })
        })
}

pub fn extract_str_len(field: &str) -> Option<usize> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^\s*str\s*\[\s*(\d+)\s*\]\s*$").unwrap();
    }
    RE.captures(field)
        .map(|captures| captures[1].to_string())
        .map(|length: String| {
            length.parse::<usize>().unwrap_or_else(|_| {
                panic!("Could not extract string length from {length}! Original field '{field}'")
            })
        })
}

pub fn has_struct_format(field: &str) -> bool {
    field.starts_with("struct ")
}

pub fn has_enum_format(field: &str) -> bool {
    field.starts_with("enum ")
}

// A custom type name should be passed to this function as `{struct,enum} $name`,
pub fn custom_type_name(type_field: &str) -> Result<String, Error> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(?:struct|enum)\s*(.*)").unwrap();
    }

    RE.captures(type_field)
        .map(|captures| String::from(&captures[1]))
        .ok_or_else(|| {
            Error::InvalidData(
                "The declared type was not in the format `(enum|struct) name`".to_string(),
            )
        })
}
