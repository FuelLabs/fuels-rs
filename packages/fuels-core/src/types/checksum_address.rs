use sha2::{Digest, Sha256};

use crate::types::errors::{Error, Result};

pub fn checksum_encode(address: &str) -> Result<String> {
    let trimmed = address.trim_start_matches("0x");
    pre_validate(trimmed)?;

    let lowercase = trimmed.to_ascii_lowercase();

    let hash = Sha256::digest(lowercase.as_bytes());
    let mut checksum = String::with_capacity(trimmed.len());

    for (i, addr_char) in lowercase.chars().enumerate() {
        let hash_byte = hash[i / 2];
        let hash_nibble = if i % 2 == 0 {
            // even index: high nibble
            (hash_byte >> 4) & 0x0F
        } else {
            // odd index: low nibble
            hash_byte & 0x0F
        };

        // checksum rule
        if hash_nibble > 7 {
            checksum.push(addr_char.to_ascii_uppercase());
        } else {
            checksum.push(addr_char);
        }
    }

    Ok(format!("0x{checksum}"))
}

fn pre_validate(s: &str) -> Result<()> {
    if s.len() != 64 {
        return Err(Error::Codec("invalid address length".to_string()));
    }

    if !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::Codec(
            "address contains invalid characters".to_string(),
        ));
    }

    Ok(())
}

pub fn is_checksum_valid(address: &str) -> bool {
    let Ok(checksum) = checksum_encode(address) else {
        return false;
    };

    let address_normalized = if address.starts_with("0x") {
        address.to_string()
    } else {
        format!("0x{}", address)
    };

    checksum == address_normalized
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use fuel_core_client::client::schema::Address;

    use super::*;

    const VALID_CHECKSUM: [&str; 4] = [
        "0x9cfB2CAd509D417ec40b70ebE1DD72a3624D46fdD1Ea5420dBD755CE7f4Dc897",
        "0x54944e5B8189827e470e5a8bAcFC6C3667397DC4E1EEF7EF3519d16D6D6c6610",
        "c36bE0E14d3EAf5d8D233e0F4a40b3b4e48427D25F84C460d2B03B242A38479e",
        "a1184D77D0D08A064E03b2bd9f50863e88faDdea4693A05cA1ee9B1732ea99B7",
    ];
    const INVALID_CHECKSUM: [&str; 8] = [
        "0x587aa0482482efEa0234752d1ad9a9c438D1f34D2859b8bef2d56A432cB68e33",
        "0xe10f526B192593793b7a1559aA91445faba82a1d669e3eb2DCd17f9c121b24b1",
        "6b63804cFbF9856e68e5B6e7aEf238dc8311ec55bec04df774003A2c96E0418e",
        "81f3A10b61828580D06cC4c7b0ed8f59b9Fb618bE856c55d33deCD95489A1e23",
        // all lower
        "0xf8f8b6283d7fa5b672b530cbb84fcccb4ff8dc40f8176ef4544ddb1f1952ad07",
        "7e2becd64cd598da59b4d1064b711661898656c6b1f4918a787156b8965dc83c",
        // all caps
        "0x26183FBE7375045250865947695DFC12500DCC43EFB9102B4E8C4D3C20009DCB",
        "577E424EE53A16E6A85291FEABC8443862495F74AC39A706D2DD0B9FC16955EB",
    ];
    const INVALID_LEN: [&str; 6] = [
        // too short
        "0x1234567890abcdef",
        // too long
        "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234",
        // 65 characters
        "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef1",
        // 63 characters
        "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcde",
        "",
        "0x",
    ];
    const INVALID_CHARACTERS: &str =
        "0xGHIJKL7890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

    #[test]
    fn will_detect_valid_checksums() {
        for valid in VALID_CHECKSUM.iter() {
            assert!(is_checksum_valid(valid));
        }
    }

    #[test]
    fn will_detect_invalid_checksums() {
        for invalid in INVALID_CHECKSUM.iter() {
            assert!(!is_checksum_valid(invalid));
        }
    }

    #[test]
    fn can_construct_address_from_checksum() {
        let checksum = checksum_encode(INVALID_CHECKSUM[0]).expect("should encode");
        Address::from_str(&checksum).expect("should be valid address");
    }

    #[test]
    fn will_detect_invalid_lengths() {
        for invalid in INVALID_LEN.iter() {
            let result = checksum_encode(invalid).expect_err("should not encode");
            assert!(result.to_string().contains("invalid address length"));
        }
    }

    #[test]
    fn will_detect_invalid_characters() {
        let result = checksum_encode(INVALID_CHARACTERS).expect_err("should not encode");
        assert!(
            result
                .to_string()
                .contains("address contains invalid characters")
        );
    }
}
