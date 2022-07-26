pub fn has_array_format(element: &str) -> bool {
    element.starts_with('[') && element.ends_with(']')
}
