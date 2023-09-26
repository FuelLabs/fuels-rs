script;

fn main(a: u64, b: u32) -> str[5] {
    if a < b.as_u64() {
        let my_string: str[5] = "hello".try_as_str_array().unwrap();
        my_string
    } else {
        let my_string: str[5] = "heyoo".try_as_str_array().unwrap();
        my_string
    }
}
