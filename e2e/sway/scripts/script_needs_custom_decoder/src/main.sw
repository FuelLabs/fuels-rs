script;

fn main(log_instead_of_return: bool) -> Option<[u8; 1000]> {
    let arr: [u8; 1000] = [0; 1000];

    if log_instead_of_return {
        log(arr);

        return None;
    }

    Some(arr)
}
