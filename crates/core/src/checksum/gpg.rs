use super::checksum_error::ProtoChecksumError;
use pgp::composed::{Deserializable, DetachedSignature, SignedPublicKey};
use pgp::types::KeyDetails;
use starbase_utils::fs;
use std::io::Cursor;
use std::path::Path;
use tracing::instrument;

const PUBLIC_KEY_ARMOR_BEGIN: &str = "-----BEGIN PGP PUBLIC KEY BLOCK-----";
const PUBLIC_KEY_ARMOR_END: &str = "-----END PGP PUBLIC KEY BLOCK-----";
const SIGNATURE_ARMOR_BEGIN: &str = "-----BEGIN PGP SIGNATURE-----";
const SIGNATURE_ARMOR_END: &str = "-----END PGP SIGNATURE-----";

fn parse_public_keys(keyring: &str) -> Result<Vec<SignedPublicKey>, ProtoChecksumError> {
    let handle_error = |error: pgp::errors::Error| ProtoChecksumError::Gpg {
        error: Box::new(error),
    };
    let mut public_keys = vec![];

    if !keyring.contains(PUBLIC_KEY_ARMOR_BEGIN) {
        let (keys, _) =
            SignedPublicKey::from_reader_many(keyring.as_bytes()).map_err(handle_error)?;

        for key in keys {
            public_keys.push(key.map_err(handle_error)?);
        }
    } else {
        let mut remaining = keyring;

        while let Some(begin) = remaining.find(PUBLIC_KEY_ARMOR_BEGIN) {
            if !remaining[..begin].trim().is_empty() {
                return Err(ProtoChecksumError::InvalidGpgKeyring {
                    reason: "unexpected content outside a public key block".into(),
                });
            }

            remaining = &remaining[begin..];

            let end = remaining.find(PUBLIC_KEY_ARMOR_END).ok_or_else(|| {
                ProtoChecksumError::InvalidGpgKeyring {
                    reason: "public key block is missing its footer".into(),
                }
            })? + PUBLIC_KEY_ARMOR_END.len();

            let (keys, _) = SignedPublicKey::from_armor_many(&remaining.as_bytes()[..end])
                .map_err(handle_error)?;

            for key in keys {
                public_keys.push(key.map_err(handle_error)?);
            }

            remaining = &remaining[end..];
        }

        if !remaining.trim().is_empty() {
            return Err(ProtoChecksumError::InvalidGpgKeyring {
                reason: "unexpected content outside a public key block".into(),
            });
        }
    }

    if public_keys.is_empty() {
        return Err(ProtoChecksumError::InvalidGpgKeyring {
            reason: "no public keys were found".into(),
        });
    }

    Ok(public_keys)
}

fn parse_signatures(contents: &[u8]) -> Result<Vec<DetachedSignature>, ProtoChecksumError> {
    let handle_error = |error: pgp::errors::Error| ProtoChecksumError::Gpg {
        error: Box::new(error),
    };
    let mut signatures = vec![];

    if let Ok(armored) = std::str::from_utf8(contents)
        && armored.contains(SIGNATURE_ARMOR_BEGIN)
    {
        let mut remaining = armored;

        while let Some(begin) = remaining.find(SIGNATURE_ARMOR_BEGIN) {
            if !remaining[..begin].trim().is_empty() {
                return Err(ProtoChecksumError::InvalidGpgSignatures {
                    reason: "unexpected content outside a signature block".into(),
                });
            }

            remaining = &remaining[begin..];

            let end = remaining.find(SIGNATURE_ARMOR_END).ok_or_else(|| {
                ProtoChecksumError::InvalidGpgSignatures {
                    reason: "signature block is missing its footer".into(),
                }
            })? + SIGNATURE_ARMOR_END.len();

            let (parsed, _) = DetachedSignature::from_armor_many(&remaining.as_bytes()[..end])
                .map_err(handle_error)?;

            for signature in parsed {
                signatures.push(signature.map_err(handle_error)?);
            }

            remaining = &remaining[end..];
        }

        if !remaining.trim().is_empty() {
            return Err(ProtoChecksumError::InvalidGpgSignatures {
                reason: "unexpected content outside a signature block".into(),
            });
        }
    } else {
        let (parsed, _) =
            DetachedSignature::from_reader_many(Cursor::new(contents)).map_err(handle_error)?;

        for signature in parsed {
            signatures.push(signature.map_err(handle_error)?);
        }
    }

    if signatures.is_empty() {
        return Err(ProtoChecksumError::InvalidGpgSignatures {
            reason: "no detached signatures were found".into(),
        });
    }

    Ok(signatures)
}

#[instrument(name = "verify_gpg_checksum", skip(checksum_public_key))]
pub fn verify_checksum(
    download_file: &Path,
    checksum_file: &Path,
    checksum_public_key: &str,
) -> Result<Option<String>, ProtoChecksumError> {
    let handle_error = |error: pgp::errors::Error| ProtoChecksumError::Gpg {
        error: Box::new(error),
    };

    let public_keys = parse_public_keys(checksum_public_key)?;
    let signatures = parse_signatures(&fs::read_file_bytes(checksum_file)?)?;

    for public_key in &public_keys {
        public_key.verify_bindings().map_err(handle_error)?;
    }

    for signature in &signatures {
        for public_key in &public_keys {
            if signature
                .signature
                .verify(public_key, fs::open_file(download_file)?)
                .is_ok()
            {
                return Ok(Some(format!("{:X}", public_key.fingerprint())));
            }

            for subkey in &public_key.public_subkeys {
                if signature
                    .signature
                    .verify(subkey, fs::open_file(download_file)?)
                    .is_ok()
                {
                    return Ok(Some(format!("{:X}", subkey.fingerprint())));
                }
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pgp::armor::{self, BlockType};
    use pgp::composed::{
        ArmorOptions, EncryptionCaps, KeyType, SecretKeyParamsBuilder, SubkeyParamsBuilder,
    };
    use pgp::crypto::hash::HashAlgorithm;
    use pgp::ser::Serialize;
    use pgp::types::Password;
    use rand::{SeedableRng, rngs::StdRng};
    use starbase_sandbox::create_empty_sandbox;
    use std::fs;

    const CONTENTS: &[u8] = b"proto gpg verification test";

    fn armor_public_key(public_key: &SignedPublicKey) -> String {
        public_key
            .to_armored_string(ArmorOptions::default())
            .unwrap()
    }

    fn create_key_and_signature(seed: u64) -> (SignedPublicKey, DetachedSignature) {
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
            .generate(StdRng::seed_from_u64(seed))
            .unwrap();
        let public_key = SignedPublicKey::from(secret_key.clone());
        let signature = DetachedSignature::sign_binary_data(
            StdRng::seed_from_u64(seed + 1),
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
        let (public_key, signature) = create_key_and_signature(1);
        let public_key = armor_public_key(&public_key);
        let download_file = sandbox.path().join("tool.tar.gz");
        let signature_file = sandbox.path().join("tool.tar.gz.sig");
        let mut signature_bytes = vec![];

        signature.to_writer(&mut signature_bytes).unwrap();
        fs::write(&download_file, CONTENTS).unwrap();
        fs::write(&signature_file, signature_bytes).unwrap();

        assert!(
            verify_checksum(&download_file, &signature_file, &public_key)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn verifies_armored_detached_signature() {
        let sandbox = create_empty_sandbox();
        let (public_key, signature) = create_key_and_signature(1);
        let public_key = armor_public_key(&public_key);
        let download_file = sandbox.path().join("tool.tar.gz");
        let signature_file = sandbox.path().join("tool.tar.gz.asc");

        fs::write(&download_file, CONTENTS).unwrap();
        fs::write(
            &signature_file,
            signature
                .to_armored_string(ArmorOptions::default())
                .unwrap(),
        )
        .unwrap();

        assert!(
            verify_checksum(&download_file, &signature_file, &public_key)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn records_signer_fingerprint_and_artifact_sha256() {
        let sandbox = create_empty_sandbox();
        let (public_key, signature) = create_key_and_signature(1);
        let expected_fingerprint = format!("{:X}", public_key.public_subkeys[0].fingerprint());
        let public_key = armor_public_key(&public_key);
        let download_file = sandbox.path().join("tool.tar.gz");
        let signature_file = sandbox.path().join("tool.tar.gz.asc");

        fs::write(&download_file, CONTENTS).unwrap();
        fs::write(
            &signature_file,
            signature
                .to_armored_string(ArmorOptions::default())
                .unwrap(),
        )
        .unwrap();

        let checksum = crate::checksum::verify_checksum_with_metadata(
            &download_file,
            &signature_file,
            &proto_pdk_api::Checksum::gpg(public_key),
        )
        .unwrap()
        .unwrap();

        assert_eq!(checksum.key.as_deref(), Some(expected_fingerprint.as_str()));
        assert_eq!(
            checksum.hash,
            Some(crate::checksum::hash_file_contents_sha256(&download_file).unwrap())
        );
        assert!(!checksum.to_string().contains("PGP PUBLIC KEY BLOCK"));
    }

    #[test]
    fn verifies_with_multiple_keys_in_one_armored_block() {
        let sandbox = create_empty_sandbox();
        let (unrelated_key, _) = create_key_and_signature(10);
        let (signing_key, signature) = create_key_and_signature(20);
        let download_file = sandbox.path().join("tool.tar.gz");
        let signature_file = sandbox.path().join("tool.tar.gz.asc");
        let mut public_keyring = vec![];

        armor::write(
            &vec![unrelated_key, signing_key],
            BlockType::PublicKey,
            &mut public_keyring,
            None,
            false,
        )
        .unwrap();
        fs::write(&download_file, CONTENTS).unwrap();
        fs::write(
            &signature_file,
            signature
                .to_armored_string(ArmorOptions::default())
                .unwrap(),
        )
        .unwrap();

        assert!(
            verify_checksum(
                &download_file,
                &signature_file,
                std::str::from_utf8(&public_keyring).unwrap(),
            )
            .unwrap()
            .is_some()
        );
    }

    #[test]
    fn verifies_with_multiple_concatenated_armored_blocks() {
        let sandbox = create_empty_sandbox();
        let (unrelated_key, _) = create_key_and_signature(10);
        let (signing_key, signature) = create_key_and_signature(20);
        let public_keyring = format!(
            "{}\n{}",
            armor_public_key(&unrelated_key),
            armor_public_key(&signing_key)
        );
        let download_file = sandbox.path().join("tool.tar.gz");
        let signature_file = sandbox.path().join("tool.tar.gz.asc");

        fs::write(&download_file, CONTENTS).unwrap();
        fs::write(
            &signature_file,
            signature
                .to_armored_string(ArmorOptions::default())
                .unwrap(),
        )
        .unwrap();

        assert!(
            verify_checksum(&download_file, &signature_file, &public_keyring)
                .unwrap()
                .is_some()
        );
    }

    #[test]
    fn rejects_keyring_with_a_malformed_block() {
        let sandbox = create_empty_sandbox();
        let (public_key, signature) = create_key_and_signature(1);
        let public_keyring = format!(
            "{}\n{PUBLIC_KEY_ARMOR_BEGIN}\ninvalid\n{PUBLIC_KEY_ARMOR_END}",
            armor_public_key(&public_key)
        );
        let download_file = sandbox.path().join("tool.tar.gz");
        let signature_file = sandbox.path().join("tool.tar.gz.asc");

        fs::write(&download_file, CONTENTS).unwrap();
        fs::write(
            &signature_file,
            signature
                .to_armored_string(ArmorOptions::default())
                .unwrap(),
        )
        .unwrap();

        assert!(verify_checksum(&download_file, &signature_file, &public_keyring).is_err());
    }

    #[test]
    fn rejects_invalid_detached_signature() {
        let sandbox = create_empty_sandbox();
        let (public_key, signature) = create_key_and_signature(1);
        let public_key = armor_public_key(&public_key);
        let download_file = sandbox.path().join("tool.tar.gz");
        let signature_file = sandbox.path().join("tool.tar.gz.asc");

        fs::write(&download_file, b"tampered").unwrap();
        fs::write(
            &signature_file,
            signature
                .to_armored_string(ArmorOptions::default())
                .unwrap(),
        )
        .unwrap();

        assert!(
            verify_checksum(&download_file, &signature_file, &public_key)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn verifies_any_signature_in_a_binary_file() {
        let sandbox = create_empty_sandbox();
        let (_, unrelated_signature) = create_key_and_signature(10);
        let (signing_key, signature) = create_key_and_signature(20);
        let expected_fingerprint = format!("{:X}", signing_key.public_subkeys[0].fingerprint());
        let public_key = armor_public_key(&signing_key);
        let download_file = sandbox.path().join("tool.tar.gz");
        let signature_file = sandbox.path().join("tool.tar.gz.sig");
        let mut signature_bytes = vec![];

        unrelated_signature.to_writer(&mut signature_bytes).unwrap();
        signature.to_writer(&mut signature_bytes).unwrap();
        fs::write(&download_file, CONTENTS).unwrap();
        fs::write(&signature_file, signature_bytes).unwrap();

        assert_eq!(
            verify_checksum(&download_file, &signature_file, &public_key).unwrap(),
            Some(expected_fingerprint)
        );
    }

    #[test]
    fn verifies_any_signature_in_one_armored_block() {
        let sandbox = create_empty_sandbox();
        let (_, unrelated_signature) = create_key_and_signature(10);
        let (signing_key, signature) = create_key_and_signature(20);
        let expected_fingerprint = format!("{:X}", signing_key.public_subkeys[0].fingerprint());
        let public_key = armor_public_key(&signing_key);
        let download_file = sandbox.path().join("tool.tar.gz");
        let signature_file = sandbox.path().join("tool.tar.gz.asc");
        let mut signatures = vec![];

        armor::write(
            &vec![unrelated_signature, signature],
            BlockType::Signature,
            &mut signatures,
            None,
            false,
        )
        .unwrap();
        fs::write(&download_file, CONTENTS).unwrap();
        fs::write(&signature_file, signatures).unwrap();

        assert_eq!(
            verify_checksum(&download_file, &signature_file, &public_key).unwrap(),
            Some(expected_fingerprint)
        );
    }

    #[test]
    fn verifies_any_signature_in_concatenated_armored_blocks() {
        let sandbox = create_empty_sandbox();
        let (_, unrelated_signature) = create_key_and_signature(10);
        let (signing_key, signature) = create_key_and_signature(20);
        let expected_fingerprint = format!("{:X}", signing_key.public_subkeys[0].fingerprint());
        let public_key = armor_public_key(&signing_key);
        let download_file = sandbox.path().join("tool.tar.gz");
        let signature_file = sandbox.path().join("tool.tar.gz.asc");

        fs::write(&download_file, CONTENTS).unwrap();
        fs::write(
            &signature_file,
            format!(
                "{}\n{}",
                unrelated_signature
                    .to_armored_string(ArmorOptions::default())
                    .unwrap(),
                signature
                    .to_armored_string(ArmorOptions::default())
                    .unwrap()
            ),
        )
        .unwrap();

        assert_eq!(
            verify_checksum(&download_file, &signature_file, &public_key).unwrap(),
            Some(expected_fingerprint)
        );
    }
}
