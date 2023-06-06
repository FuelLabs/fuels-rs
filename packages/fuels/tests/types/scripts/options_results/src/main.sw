script;

pub enum TestError {
    ZimZam: str[5],
}

fn main(bim: Option<u32>, _bam: Option<u64>) -> Result<Option<bool>, TestError> {
    if let Option::Some(42) = bim {
        Result::Ok(Option::Some(true))
    } else if let Option::Some(_) = bim {
        Result::Ok(Option::None)
    } else {
        Result::Err(TestError::ZimZam("error"))
    }
}
