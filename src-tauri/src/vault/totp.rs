//! Authenticator codes (TOTP, RFC 6238): parse `otpauth://` links and compute codes.

use hmac::{KeyInit, Mac};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Algorithm {
    Sha1,
    Sha256,
    Sha512,
}

/// A 2FA account as set up from a QR link or a typed secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub issuer: String,
    pub label: String,
    pub secret: Vec<u8>,
    pub digits: u32,
    pub period: u32,
    pub algorithm: Algorithm,
}

/// RFC 4648 base32, as authenticator secrets are written (spaces and case ignored).
pub fn base32(text: &str) -> Option<Vec<u8>> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut bits = 0u64;
    let mut count = 0;
    let mut out = Vec::new();
    for c in text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-' && *c != '=')
    {
        let value = ALPHABET.iter().position(|&a| a == c.to_ascii_uppercase() as u8)? as u64;
        bits = (bits << 5) | value;
        count += 5;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
    }
    (!out.is_empty()).then_some(out)
}

fn decode(s: &str) -> String {
    url::form_urlencoded::parse(format!("x={s}").as_bytes())
        .next()
        .map(|(_, v)| v.into_owned())
        .unwrap_or_default()
}

/// From `otpauth://totp/Issuer:name?secret=…&issuer=…`, or a bare base32 secret plus a name.
pub fn parse(input: &str, name: &str) -> AppResult<Account> {
    let bad = |what: &str| AppError::InvalidInput(what.into());
    let input = input.trim();
    if !input.to_lowercase().starts_with("otpauth://") {
        let secret = base32(input).ok_or_else(|| bad("That isn't a valid secret key"))?;
        let name = name.trim();
        if name.is_empty() {
            return Err(bad("Give the account a name"));
        }
        return Ok(Account {
            issuer: name.chars().take(60).collect(),
            label: String::new(),
            secret,
            digits: 6,
            period: 30,
            algorithm: Algorithm::Sha1,
        });
    }
    let url = url::Url::parse(input).map_err(|_| bad("That isn't a valid 2FA link"))?;
    if url.host_str() != Some("totp") {
        return Err(bad("Only time-based (TOTP) codes are supported"));
    }
    let path = decode(url.path().trim_start_matches('/'));
    let (path_issuer, label) = match path.split_once(':') {
        Some((i, l)) => (i.trim().to_owned(), l.trim().to_owned()),
        None => (String::new(), path.trim().to_owned()),
    };
    let mut account = Account {
        issuer: path_issuer,
        label,
        secret: Vec::new(),
        digits: 6,
        period: 30,
        algorithm: Algorithm::Sha1,
    };
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "secret" => account.secret = base32(&value).ok_or_else(|| bad("The link's secret is invalid"))?,
            "issuer" => account.issuer = value.trim().to_owned(),
            "digits" => account.digits = value.parse().map_err(|_| bad("Invalid digits"))?,
            "period" => account.period = value.parse().map_err(|_| bad("Invalid period"))?,
            "algorithm" => {
                account.algorithm = match value.to_uppercase().as_str() {
                    "SHA1" => Algorithm::Sha1,
                    "SHA256" => Algorithm::Sha256,
                    "SHA512" => Algorithm::Sha512,
                    _ => return Err(bad("Unsupported algorithm")),
                }
            }
            _ => {}
        }
    }
    if account.secret.is_empty() {
        return Err(bad("The link has no secret"));
    }
    if !(6..=8).contains(&account.digits) || !(10..=300).contains(&account.period) {
        return Err(bad("Unsupported code length or period"));
    }
    if account.issuer.is_empty() {
        account.issuer = if name.trim().is_empty() {
            account.label.clone()
        } else {
            name.trim().to_owned()
        };
    }
    account.issuer = account.issuer.chars().take(60).collect();
    account.label = account.label.chars().take(80).collect();
    Ok(account)
}

fn hmac(algorithm: Algorithm, key: &[u8], message: &[u8]) -> Vec<u8> {
    fn run<M: Mac + KeyInit>(key: &[u8], message: &[u8]) -> Vec<u8> {
        let mut mac = <M as KeyInit>::new_from_slice(key).expect("HMAC takes any key length");
        mac.update(message);
        mac.finalize().into_bytes().to_vec()
    }
    match algorithm {
        Algorithm::Sha1 => run::<hmac::Hmac<sha1::Sha1>>(key, message),
        Algorithm::Sha256 => run::<hmac::Hmac<sha2::Sha256>>(key, message),
        Algorithm::Sha512 => run::<hmac::Hmac<sha2::Sha512>>(key, message),
    }
}

/// The code at `unix_seconds`, zero-padded to `digits`.
pub fn code(secret: &[u8], algorithm: Algorithm, digits: u32, period: u32, unix_seconds: u64) -> String {
    let counter = unix_seconds / u64::from(period.max(1));
    let hash = hmac(algorithm, secret, &counter.to_be_bytes());
    let offset = (hash[hash.len() - 1] & 0x0f) as usize;
    let binary = u32::from_be_bytes([
        hash[offset] & 0x7f,
        hash[offset + 1],
        hash[offset + 2],
        hash[offset + 3],
    ]);
    let value = binary % 10u32.pow(digits);
    format!("{value:0width$}", width = digits as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 6238 appendix B vectors (8 digits).
    #[test]
    fn matches_the_rfc_vectors() {
        let sha1 = b"12345678901234567890";
        let sha256 = b"12345678901234567890123456789012";
        let sha512 = b"1234567890123456789012345678901234567890123456789012345678901234";
        for (t, a, b, c) in [
            (59, "94287082", "46119246", "90693936"),
            (1_111_111_109, "07081804", "68084774", "25091201"),
            (2_000_000_000, "69279037", "90698825", "38618901"),
        ] {
            assert_eq!(code(sha1, Algorithm::Sha1, 8, 30, t), a);
            assert_eq!(code(sha256, Algorithm::Sha256, 8, 30, t), b);
            assert_eq!(code(sha512, Algorithm::Sha512, 8, 30, t), c);
        }
    }

    #[test]
    fn decodes_base32() {
        assert_eq!(base32("GEZDGNBVGY3TQOJQ").unwrap(), b"1234567890");
        assert_eq!(base32("gezd gnbv gy3t qojq").unwrap(), b"1234567890");
        assert!(base32("not base32!").is_none());
    }

    #[test]
    fn parses_otpauth_links() {
        let a = parse(
            "otpauth://totp/GitHub:dhyey%40example.com?secret=GEZDGNBVGY3TQOJQ&issuer=GitHub&digits=6",
            "",
        )
        .unwrap();
        assert_eq!(a.issuer, "GitHub");
        assert_eq!(a.label, "dhyey@example.com");
        assert_eq!(a.secret, b"1234567890");
        let b = parse(
            "otpauth://totp/Example?secret=GEZDGNBVGY3TQOJQ&algorithm=SHA256&period=60",
            "",
        )
        .unwrap();
        assert_eq!(b.issuer, "Example");
        assert_eq!(b.algorithm, Algorithm::Sha256);
        assert_eq!(b.period, 60);
        assert!(parse("otpauth://hotp/X?secret=GEZDGNBVGY3TQOJQ", "").is_err());
        assert!(parse("otpauth://totp/X?issuer=Y", "").is_err());
        let typed = parse("GEZD GNBV GY3T QOJQ", "Discord").unwrap();
        assert_eq!(typed.issuer, "Discord");
        assert!(parse("GEZDGNBVGY3TQOJQ", "  ").is_err());
    }
}
