use lazy_static::lazy_static;
use regex::Regex;

pub(crate) fn extract_generic_name(field: &str) -> Option<String> {
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

pub fn has_tuple_format(field: &str) -> bool {
    field.starts_with('(') && field.ends_with(')')
}

pub fn has_struct_format(field: &str) -> bool {
    field.starts_with("struct ")
}

pub fn has_enum_format(field: &str) -> bool {
    field.starts_with("enum ")
}
