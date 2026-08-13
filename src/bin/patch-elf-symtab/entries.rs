//! Symbol → hex-encoded payload mapping for interchange formats.
//!
//! Public types are used from JSON and forthcoming CLI wiring; the binary does not reference them yet.

use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Raw bytes that serialize to a **lowercase hex string** and deserialize from hex where **A–F
/// or a–f** are accepted (optional **`0x` / `0X`** prefix).
#[derive(Clone, PartialEq, Eq)]
pub struct HexBytes(pub Vec<u8>);

impl fmt::Debug for HexBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("HexBytes")
            .field(&hex::encode(&self.0))
            .finish()
    }
}

impl Serialize for HexBytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(&self.0))
    }
}

/// Deserialize from a **single JSON string** whose characters are interpreted as hex (two digits per
/// output byte).
impl<'de> Deserialize<'de> for HexBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        /// Parses the inner contents of a JSON string after serde has validated the outer type.
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = HexBytes;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a hex-encoded string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let v = strip_optional_hex_prefix(v);
                let bytes = hex::decode(v).map_err(E::custom)?;
                Ok(HexBytes(bytes))
            }

            // Borrowed JSON strings usually hit `visit_str`; owned strings (or some formats)
            // delegate here—reuse one implementation so parsing stays identical.
            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_str(&v)
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

/// Returns `s` without leading ASCII whitespace, then strips **`0x` / `0X`** only when those two
/// characters appear at position zero—any later `x` is treated as part of the hex payload (so we do
/// not interpret `"10xf"` as prefixed hex).
fn strip_optional_hex_prefix(s: &str) -> &str {
    let s = s.trim();
    let b = s.as_bytes();
    if b.len() >= 2 && b[0] == b'0' && (b[1] == b'x' || b[1] == b'X') {
        &s[2..]
    } else {
        s
    }
}

/// Map of logical name → hex-encoded bytes (JSON values are hex strings).
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Entries(pub HashMap<String, HexBytes>);

impl Deref for Entries {
    type Target = HashMap<String, HexBytes>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_bytes_roundtrip_json() {
        let original = HexBytes(vec![0xde, 0xad, 0xbe, 0xef]);
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(json, "\"deadbeef\"");
        let back: HexBytes = serde_json::from_str(&json).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn entries_roundtrip_json() {
        let mut m = HashMap::new();
        m.insert("sym_a".to_string(), HexBytes(vec![1, 2, 3]));
        let entries = Entries(m);

        let json = serde_json::to_string(&entries).unwrap();
        assert!(json.contains("\"sym_a\":\"010203\""));

        let back: Entries = serde_json::from_str(&json).unwrap();
        assert_eq!(entries, back);
    }

    #[test]
    fn accepts_optional_0x_prefix() {
        let v: HexBytes = serde_json::from_str("\"0x00ff\"").unwrap();
        assert_eq!(v.0, vec![0x00, 0xff]);
    }

    #[test]
    fn accepts_optional_0x_uppercase_prefix() {
        let v: HexBytes = serde_json::from_str("\"0X00ff\"").unwrap();
        assert_eq!(v.0, vec![0x00, 0xff]);
    }

    #[test]
    fn accepts_uppercase_hex() {
        let v: HexBytes = serde_json::from_str("\"DEADBEEF\"").unwrap();
        assert_eq!(v.0, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn accepts_mixed_case_hex() {
        let v: HexBytes = serde_json::from_str("\"DeAdBeEf\"").unwrap();
        assert_eq!(v.0, vec![0xde, 0xad, 0xbe, 0xef]);
    }
}
