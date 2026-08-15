use super::checksum_error::ProtoChecksumError;
use pgp::composed::{Deserializable, DetachedSignature, SignedPublicKey};
use starbase_utils::fs;
use std::path::Path;
use tracing::instrument;

#[instrument(name = "verify_gpg_checksum", skip(checksum_public_key))]
pub fn verify_checksum(
    download_file: &Path,
    checksum_file: &Path,
    checksum_public_key: &str,
) -> Result<bool, ProtoChecksumError> {
    let handle_error = |error: pgp::errors::Error| ProtoChecksumError::Gpg {
        error: Box::new(error),
    };

    let (public_key, _) = SignedPublicKey::from_reader_single(checksum_public_key.as_bytes())
        .map_err(handle_error)?;
    let (signature, _) = DetachedSignature::from_reader_single(fs::open_file(checksum_file)?)
        .map_err(handle_error)?;
    let contents = fs::read_file_bytes(download_file)?;

    public_key.verify_bindings().map_err(handle_error)?;

    if signature.verify(&public_key, &contents).is_ok() {
        return Ok(true);
    }

    for subkey in &public_key.public_subkeys {
        if signature.verify(subkey, &contents).is_ok() {
            return Ok(true);
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pgp::composed::{
        ArmorOptions, EncryptionCaps, KeyType, SecretKeyParamsBuilder, SubkeyParamsBuilder,
    };
    use pgp::crypto::hash::HashAlgorithm;
    use pgp::ser::Serialize;
    use pgp::types::Password;
    use rand::{SeedableRng, rngs::StdRng};
    use starbase_sandbox::create_empty_sandbox;

    const CONTENTS: &[u8] = b"proto gpg verification test";

    fn create_key_and_signature() -> (String, DetachedSignature) {
        let mut signing_subkey = SubkeyParamsBuilder::default();
        signing_subkey
            .key_type(KeyType::Ed25519Legacy)
            .can_sign(true)
            .can_encrypt(EncryptionCaps::None)
            .can_authenticate(false);

        let mut params = SecretKeyParamsBuilder::default();
        params
            .key_type(KeyType::Ed25519Legacy)
            .can_certify(true)
            .can_sign(false)
            .can_encrypt(EncryptionCaps::None)
            .primary_user_id("Proto Test <test@moonrepo.dev>".into())
            .subkeys(vec![signing_subkey.build().unwrap()]);

        let secret_key = params
            .build()
            .unwrap()
            .generate(StdRng::seed_from_u64(1))
            .unwrap();
        let public_key = SignedPublicKey::from(secret_key.clone())
            .to_armored_string(ArmorOptions::default())
            .unwrap();
        let signature = DetachedSignature::sign_binary_data(
            StdRng::seed_from_u64(2),
            &secret_key.secret_subkeys[0].key,
            &Password::empty(),
            HashAlgorithm::Sha256,
            CONTENTS,
        )
        .unwrap();

        (public_key, signature)
    }

    #[test]
    fn verifies_binary_detached_signature() {
        let sandbox = create_empty_sandbox();
        let (public_key, signature) = create_key_and_signature();
        let download_file = sandbox.path().join("tool.tar.gz");
        let signature_file = sandbox.path().join("tool.tar.gz.sig");
        let mut signature_bytes = vec![];

        signature.to_writer(&mut signature_bytes).unwrap();
        std::fs::write(&download_file, CONTENTS).unwrap();
        std::fs::write(&signature_file, signature_bytes).unwrap();

        assert!(verify_checksum(&download_file, &signature_file, &public_key).unwrap());
    }

    #[test]
    fn verifies_armored_detached_signature() {
        let sandbox = create_empty_sandbox();
        let (public_key, signature) = create_key_and_signature();
        let download_file = sandbox.path().join("tool.tar.gz");
        let signature_file = sandbox.path().join("tool.tar.gz.asc");

        std::fs::write(&download_file, CONTENTS).unwrap();
        std::fs::write(
            &signature_file,
            signature
                .to_armored_string(ArmorOptions::default())
                .unwrap(),
        )
        .unwrap();

        assert!(verify_checksum(&download_file, &signature_file, &public_key).unwrap());
    }

    #[test]
    fn rejects_invalid_detached_signature() {
        let sandbox = create_empty_sandbox();
        let (public_key, signature) = create_key_and_signature();
        let download_file = sandbox.path().join("tool.tar.gz");
        let signature_file = sandbox.path().join("tool.tar.gz.asc");

        std::fs::write(&download_file, b"tampered").unwrap();
        std::fs::write(
            &signature_file,
            signature
                .to_armored_string(ArmorOptions::default())
                .unwrap(),
        )
        .unwrap();

        assert!(!verify_checksum(&download_file, &signature_file, &public_key).unwrap());
    }
}
