contract;

use std::bytes::Bytes;

pub enum TestError {
    Something: [u8; 5],
    Else: u64,
}

pub enum Something {
    a: (),
    b: u64,
}

abi MyContract {
    fn returns_bytes_result(return_ok: bool) -> Result<Bytes, TestError>;
    fn returns_vec_result(return_ok: bool) -> Result<Vec<u64>, TestError>;
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
}
