#[cfg(feature = "signer-aws-kms")]
pub mod aws;
#[cfg(feature = "signer-google-kms")]
pub mod google;

mod signature_utils {
    use fuel_crypto::{Message, Signature};
    use fuels_core::types::errors::{Error, Result};
    use k256::{
        PublicKey as K256PublicKey,
        ecdsa::{RecoveryId, Signature as K256Signature, VerifyingKey},
    };

    pub fn normalize_signature(
        signature_der: &[u8],
        message: Message,
        expected_pubkey: &K256PublicKey,
        error_prefix: &str,
    ) -> Result<(K256Signature, RecoveryId)> {
        let signature = K256Signature::from_der(signature_der)
            .map_err(|_| Error::Other(format!("{error_prefix}: Invalid DER signature")))?;

        let normalized_sig = signature.normalize_s().unwrap_or(signature);

        let recovery_id =
            determine_recovery_id(&normalized_sig, message, expected_pubkey, error_prefix)?;

        Ok((normalized_sig, recovery_id))
    }

    pub fn determine_recovery_id(
        sig: &K256Signature,
        message: Message,
        expected_pubkey: &K256PublicKey,
        error_prefix: &str,
    ) -> Result<RecoveryId> {
        let recid_even = RecoveryId::new(false, false);
        let recid_odd = RecoveryId::new(true, false);

        let expected_verifying_key: VerifyingKey = expected_pubkey.into();

        let recovered_even = VerifyingKey::recover_from_prehash(&*message, sig, recid_even);
        let recovered_odd = VerifyingKey::recover_from_prehash(&*message, sig, recid_odd);

        if recovered_even
            .map(|r| r == expected_verifying_key)
            .unwrap_or(false)
        {
            Ok(recid_even)
        } else if recovered_odd
            .map(|r| r == expected_verifying_key)
            .unwrap_or(false)
        {
            Ok(recid_odd)
        } else {
            Err(Error::Other(format!(
                "{error_prefix}: Invalid signature (could not recover correct public key)"
            )))
        }
    }

    pub fn convert_to_fuel_signature(
        signature: K256Signature,
        recovery_id: RecoveryId,
    ) -> Signature {
        let recovery_byte = recovery_id.is_y_odd() as u8;

        let mut bytes: [u8; 64] = signature.to_bytes().into();

        bytes[32] = (recovery_byte << 7) | (bytes[32] & 0x7F);

        Signature::from_bytes(bytes)
    }
}
