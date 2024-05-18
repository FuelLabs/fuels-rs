script;

fn main(a: u64, b: u32) -> str[5] {
    if a < b.as_u64() {
        let my_string: str[5] = __to_str_array("hello");
        my_string
    } else {
        let my_string: str[5] = __to_str_array("heyoo");
        my_string
    }
}
