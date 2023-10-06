contract;

use std::bytes::Bytes;
use std::string::String;

pub enum TestError {
    Something: [u8; 5],
    Else: u64,
}

pub struct Bimbam {
    something: Bytes,
}

abi MyContract {
    fn returns_bytes_result(return_ok: bool) -> Result<Bytes, TestError>;
    fn returns_vec_result(return_ok: bool) -> Result<Vec<u64>, TestError>;
    fn returns_string_result(return_ok: bool) -> Result<String, TestError>;
    fn returns_bytes_option(return_some: bool) -> Option<Bytes>;
    fn returns_vec_option(return_some: bool) -> Option<Vec<u64>>;
    fn returns_string_option(return_some: bool) -> Option<String>;
    fn would_raise_a_memory_overflow() -> Result<Bytes, b256>;
    fn returns_a_heap_type_too_deep() -> Result<Bimbam, u64>;
}

impl MyContract for Contract {
    fn returns_bytes_result(return_ok: bool) -> Result<Bytes, TestError> {
        return if return_ok {
            let mut b = Bytes::new();
            b.push(1u8);
            b.push(1u8);
            b.push(1u8);
            b.push(1u8);
            Result::Ok(b)
        } else {
            Result::Err(TestError::Something([255u8, 255u8, 255u8, 255u8, 255u8]))
        }
    }

    fn returns_vec_result(return_ok: bool) -> Result<Vec<u64>, TestError> {
        return if return_ok {
            let mut v = Vec::new();
            v.push(2);
            v.push(2);
            v.push(2);
            v.push(2);
            v.push(2);
            Result::Ok(v)
        } else {
            Result::Err(TestError::Else(7777))
        }
    }

    fn returns_string_result(return_ok: bool) -> Result<String, TestError> {
        return if return_ok {
            let s = String::from_ascii_str("Hello World");
            Result::Ok(s)
        } else {
            Result::Err(TestError::Else(3333))
        }
    }

    fn returns_bytes_option(return_some: bool) -> Option<Bytes> {
        return if return_some {
            let mut b = Bytes::new();
            b.push(1u8);
            b.push(1u8);
            b.push(1u8);
            b.push(1u8);
            Option::Some(b)
        } else {
            Option::None
        }
    }

    fn returns_vec_option(return_some: bool) -> Option<Vec<u64>> {
        return if return_some {
            let mut v = Vec::new();
            v.push(2);
            v.push(2);
            v.push(2);
            v.push(2);
            v.push(2);
            Option::Some(v)
        } else {
            None
        }
    }

    fn returns_string_option(return_some: bool) -> Option<String> {
        return if return_some {
            let s = String::from_ascii_str("Hello World");
            Option::Some(s)
        } else {
            None
        }
    }

    fn would_raise_a_memory_overflow() -> Result<Bytes, b256> {
        Result::Err(0x1111111111111111111111111111111111111111111111111111111111111111)
    }

    fn returns_a_heap_type_too_deep() -> Result<Bimbam, u64> {
        let mut b = Bytes::new();
        b.push(2u8);
        b.push(2u8);
        b.push(2u8);
        Result::Ok(Bimbam { something: b })
    }
}
