use fuel_crypto::Hasher;
use fuel_tx::{Address, Bytes32, Bytes64};
use fuel_vm::crypto::secp256k1_sign_compact_recover;
use fuels_core::Bits256;
use std::{convert::TryFrom, fmt, str::FromStr};
use thiserror::Error;

/// An error involving a signature.
#[derive(Debug, Error)]
pub enum SignatureError {
    /// Invalid length
    #[error("invalid signature length, got {0}, expected 64")]
    InvalidLength(usize),
    /// When parsing a signature from string to hex
    #[error(transparent)]
    DecodingError(#[from] hex::FromHexError),
    /// Thrown when signature verification failed (i.e. when the address that
    /// produced the signature did not match the expected address)
    #[error("Signature verification failed. Expected {0}, got {1}")]
    VerificationError(Address, Address),
    /// Error in recovering public key from signature
    #[error("Public key recovery error")]
    RecoveryError,
}

/// Recovery message data.
#[derive(Clone, Debug, PartialEq)]
pub enum RecoveryMessage {
    /// Message bytes
    Data(Vec<u8>),
    /// Message hash
    Hash(Bits256),
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
/// An ECDSA signature. Encoded as:
///
/// ```plaintext
/// |    32 bytes   ||           32 bytes           |
/// [256-bit r value][1-bit v value][255-bit s value]
/// ```
///
/// The encoding of the signature was derived from
/// [Compact Signature Representation](https://eips.ethereum.org/EIPS/eip-2098).
///
/// Signatures are represented as the `r` and `s` (each 32 bytes),
/// and `v` (1-bit) values of the signature. `r` and `s` take on
/// their usual meaning while `v` is used for recovering the public
/// key from a signature more quickly.
pub struct Signature {
    pub compact: Bytes64,
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.compact))
    }
}

impl Signature {
    /// Verifies that signature on `message` was produced by `address`
    pub fn verify<M, A>(&self, message: M, address: A) -> Result<(), SignatureError>
    where
        M: Into<RecoveryMessage>,
        A: Into<Address>,
    {
        let address = address.into();
        let recovered = self.recover(message)?;

        if recovered != address {
            return Err(SignatureError::VerificationError(address, recovered));
        }

        Ok(())
    }

    /// Recovers the Fuel address which was used
    /// to sign the given message. Note that this message
    /// can be either the original message or its digest (hashed message).
    /// Both can be used to recover the address of the signature. E.g.:
    ///
    /// `let recovered_address = signature.recover(message).unwrap();`
    /// Where `message` is a `&str`. Or
    ///
    /// `let recovered_address = signature.recover(&tx.id()).unwrap();`
    /// Where `&tx.id()` is a `&Bytes32` representing the hash of the tx.
    pub fn recover<M>(&self, message: M) -> Result<Address, SignatureError>
    where
        M: Into<RecoveryMessage>,
    {
        let message = message.into();
        let message_hash = match message {
            RecoveryMessage::Data(ref message) => Hasher::hash(&message[..]),
            RecoveryMessage::Hash(hash) => hash.into(),
        };

        let recovered =
            secp256k1_sign_compact_recover(self.compact.as_ref(), message_hash.as_ref()).unwrap();

        let hashed = Hasher::hash(recovered);

        Ok(Address::new(*hashed))
    }
}

impl<'a> TryFrom<&'a [u8]> for Signature {
    type Error = SignatureError;

    fn try_from(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        if bytes.len() != 64 {
            return Err(SignatureError::InvalidLength(bytes.len()));
        }

        Ok(Signature {
            compact: unsafe { Bytes64::from_slice_unchecked(bytes) },
        })
    }
}

impl FromStr for Signature {
    type Err = SignatureError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s)?;
        Signature::try_from(&bytes[..])
    }
}

impl From<&[u8]> for RecoveryMessage {
    fn from(s: &[u8]) -> Self {
        s.to_owned().into()
    }
}

impl From<Vec<u8>> for RecoveryMessage {
    fn from(s: Vec<u8>) -> Self {
        RecoveryMessage::Data(s)
    }
}

impl From<&str> for RecoveryMessage {
    fn from(s: &str) -> Self {
        s.as_bytes().to_owned().into()
    }
}

impl From<String> for RecoveryMessage {
    fn from(s: String) -> Self {
        RecoveryMessage::Data(s.into_bytes())
    }
}

impl From<[u8; 32]> for RecoveryMessage {
    fn from(hash: [u8; 32]) -> Self {
        RecoveryMessage::Hash(hash)
    }
}

impl From<&Bytes32> for RecoveryMessage {
    fn from(hash: &Bytes32) -> Self {
        RecoveryMessage::Hash(**hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify() {
        let msg = RecoveryMessage::Data("Some data".into());

        let address =
            Address::from_str("0x014587212741268ad0b1bc727efce9711dbde69c484a9db38bd83bb1b3017c05")
                .unwrap();

        let signature = Signature::from_str(
            "64d8b60c08a7ecab307cb11a31a7153ec7e4ff06a8fb78b4fe9c982d44c731efe63303ec5c7686a56445bacdd4ee89f592f1b3e68bded25ea404cd6806205db4"
        ).expect("could not parse signature");

        signature.verify(msg, address).unwrap();
    }

    #[test]
    fn recover_signature() {
        let signature = Signature::from_str(
            "64d8b60c08a7ecab307cb11a31a7153ec7e4ff06a8fb78b4fe9c982d44c731efe63303ec5c7686a56445bacdd4ee89f592f1b3e68bded25ea404cd6806205db4"
        ).expect("could not parse signature");

        assert_eq!(
            signature.recover("Some data").unwrap(),
            Address::from_str("0x014587212741268ad0b1bc727efce9711dbde69c484a9db38bd83bb1b3017c05")
                .unwrap()
        );
    }

    #[test]
    fn signature_from_str() {
        let s1 = Signature::from_str(
            "0x64d8b60c08a7ecab307cb11a31a7153ec7e4ff06a8fb78b4fe9c982d44c731efe63303ec5c7686a56445bacdd4ee89f592f1b3e68bded25ea404cd6806205db4"
        ).expect("could not parse 0x-prefixed signature");

        let s2 = Signature::from_str(
            "64d8b60c08a7ecab307cb11a31a7153ec7e4ff06a8fb78b4fe9c982d44c731efe63303ec5c7686a56445bacdd4ee89f592f1b3e68bded25ea404cd6806205db4"
        ).expect("could not parse non-prefixed signature");

        assert_eq!(s1, s2);
    }
}
