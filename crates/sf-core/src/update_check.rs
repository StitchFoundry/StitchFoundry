//! Update check, pure logic only (issue 237, slice 1): no network, no UI.
//!
//! A release manifest names a flavor, a version, the SHA-256 and size of the
//! download, and an Ed25519 signature over the text
//! `stitch-release-v1\n<flavor>\n<version>\n<sha256>\n<size>`. Nothing is
//! trusted until the signature verifies; anything unreadable means "no update".

use ed25519_dalek::{Signature, VerifyingKey};
use sha2::{Digest, Sha256};

/// Tag on the first line of the signed text.
pub const SIGNED_TAG: &str = "stitch-release-v1";

/// A release as the manifest describes it. All fields are untrusted text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub flavor: String,
    pub version: String,
    /// SHA-256 of the download, lowercase hex.
    pub sha256: String,
    pub size: u64,
}

/// Why a manifest or download was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UpdateCheckError {
    #[error("the public key is not valid")]
    BadKey,
    #[error("the signature is not valid hex of 64 bytes")]
    BadSignature,
    #[error("the signature does not match the release")]
    Forged,
    #[error("the download does not match the signed hash or size")]
    DownloadMismatch,
}

/// Parse `x.y.z` (optional leading `v`). Anything else is `None`.
pub fn parse_version(text: &str) -> Option<(u64, u64, u64)> {
    let t = text.trim();
    let t = t.strip_prefix('v').unwrap_or(t);
    let mut it = t.split('.');
    let mut next = || {
        let part = it.next()?;
        if part.is_empty() || part.len() > 9 || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        part.parse::<u64>().ok()
    };
    let v = (next()?, next()?, next()?);
    if it.next().is_some() {
        return None;
    }
    Some(v)
}

/// True only when `candidate` is a strictly newer x.y.z than `running`.
/// Equal, older or unreadable versions are "not newer".
pub fn is_newer(running: &str, candidate: &str) -> bool {
    match (parse_version(running), parse_version(candidate)) {
        (Some(r), Some(c)) => c > r,
        _ => false,
    }
}

/// The exact bytes the manifest signature covers.
pub fn signed_text(r: &Release) -> String {
    format!("{SIGNED_TAG}\n{}\n{}\n{}\n{}", r.flavor, r.version, r.sha256, r.size)
}

fn from_hex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) || !s.is_ascii() {
        return None;
    }
    (0..s.len()).step_by(2).map(|i| s.get(i..i + 2).and_then(|p| u8::from_str_radix(p, 16).ok())).collect()
}

/// Check the release signature (`signature_hex`, 64 bytes) against the
/// publisher key (`public_key_hex`, 32 bytes).
pub fn verify_release(r: &Release, signature_hex: &str, public_key_hex: &str) -> Result<(), UpdateCheckError> {
    let key: [u8; 32] = from_hex(public_key_hex).and_then(|b| b.try_into().ok()).ok_or(UpdateCheckError::BadKey)?;
    let key = VerifyingKey::from_bytes(&key).map_err(|_| UpdateCheckError::BadKey)?;
    let sig: [u8; 64] = from_hex(signature_hex).and_then(|b| b.try_into().ok()).ok_or(UpdateCheckError::BadSignature)?;
    let sig = Signature::from_bytes(&sig);
    key.verify_strict(signed_text(r).as_bytes(), &sig).map_err(|_| UpdateCheckError::Forged)
}

/// SHA-256 of `bytes` as lowercase hex.
pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// Check a download against the (already verified) release.
pub fn verify_download(r: &Release, bytes: &[u8]) -> Result<(), UpdateCheckError> {
    if bytes.len() as u64 != r.size || !sha256_hex(bytes).eq_ignore_ascii_case(r.sha256.trim()) {
        return Err(UpdateCheckError::DownloadMismatch);
    }
    Ok(())
}

/// The whole decision: `Some(version)` only when the signature verifies and the
/// version is strictly newer than the running one and for the same flavor.
pub fn available_update(running_version: &str, flavor: &str, r: &Release, signature_hex: &str, public_key_hex: &str) -> Option<String> {
    if r.flavor != flavor || !is_newer(running_version, &r.version) {
        return None;
    }
    verify_release(r, signature_hex, public_key_hex).ok()?;
    Some(r.version.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }
    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }
    fn pubhex() -> String {
        hex(key().verifying_key().as_bytes())
    }
    fn release(version: &str, data: &[u8]) -> Release {
        Release { flavor: "desktop".into(), version: version.into(), sha256: sha256_hex(data), size: data.len() as u64 }
    }
    fn sign(r: &Release) -> String {
        hex(&key().sign(signed_text(r).as_bytes()).to_bytes())
    }

    #[test]
    fn signed_text_format() {
        let r = Release { flavor: "f".into(), version: "1.2.3".into(), sha256: "ab".into(), size: 9 };
        assert_eq!(signed_text(&r), "stitch-release-v1\nf\n1.2.3\nab\n9");
    }

    #[test]
    fn only_strictly_newer_counts() {
        assert!(is_newer("1.2.3", "1.2.4"));
        assert!(is_newer("1.9.0", "v1.10.0"));
        assert!(!is_newer("1.2.3", "1.2.3"));
        assert!(!is_newer("1.2.3", "1.2.2"), "downgrade");
        assert!(!is_newer("2.0.0", "1.99.99"), "downgrade");
    }

    #[test]
    fn garbled_versions_are_not_newer() {
        for bad in ["", "abc", "1.2", "1.2.3.4", "1.2.x", "1..3", "-1.2.3", "1.2.3-beta", "99999999999999999999.0.0", "１.0.0"] {
            assert!(!is_newer("0.0.1", bad), "{bad:?}");
            assert!(!is_newer(bad, "9.9.9"), "{bad:?}");
        }
    }

    #[test]
    fn good_signature_verifies() {
        let r = release("1.3.0", b"installer");
        assert_eq!(verify_release(&r, &sign(&r), &pubhex()), Ok(()));
        assert_eq!(available_update("1.2.0", "desktop", &r, &sign(&r), &pubhex()), Some("1.3.0".into()));
    }

    #[test]
    fn forged_signature_is_refused() {
        let r = release("1.3.0", b"installer");
        let mut forged = r.clone();
        forged.sha256 = sha256_hex(b"malware");
        // signature made for the real release does not cover the changed hash
        assert_eq!(verify_release(&forged, &sign(&r), &pubhex()), Err(UpdateCheckError::Forged));
        // a different key signing the same text
        let other = SigningKey::from_bytes(&[9u8; 32]);
        let sig = hex(&other.sign(signed_text(&r).as_bytes()).to_bytes());
        assert_eq!(verify_release(&r, &sig, &pubhex()), Err(UpdateCheckError::Forged));
        assert_eq!(available_update("1.2.0", "desktop", &forged, &sign(&r), &pubhex()), None);
    }

    #[test]
    fn unreadable_signature_or_key() {
        let r = release("1.3.0", b"x");
        assert_eq!(verify_release(&r, "zz", &pubhex()), Err(UpdateCheckError::BadSignature));
        assert_eq!(verify_release(&r, "", &pubhex()), Err(UpdateCheckError::BadSignature));
        assert_eq!(verify_release(&r, &sign(&r), "abcd"), Err(UpdateCheckError::BadKey));
        assert_eq!(verify_release(&r, "é", "é"), Err(UpdateCheckError::BadKey));
    }

    #[test]
    fn downgrade_and_wrong_flavor_show_nothing() {
        let r = release("1.1.0", b"x");
        assert_eq!(available_update("1.2.0", "desktop", &r, &sign(&r), &pubhex()), None);
        let r = release("1.3.0", b"x");
        assert_eq!(available_update("1.2.0", "web", &r, &sign(&r), &pubhex()), None);
    }

    #[test]
    fn download_check() {
        let r = release("1.3.0", b"installer");
        assert_eq!(verify_download(&r, b"installer"), Ok(()));
        assert_eq!(verify_download(&r, b"installer!"), Err(UpdateCheckError::DownloadMismatch));
        assert_eq!(verify_download(&r, b"installeR"), Err(UpdateCheckError::DownloadMismatch));
    }
}
