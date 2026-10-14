use serde::Deserialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(from = "OidRepr")]
pub struct ObjectId(pub String);

const OID_SECONDS_HEX: usize = 8;

pub fn oid_seconds(oid: &str) -> Option<u32> {
    oid.get(..OID_SECONDS_HEX)
        .and_then(|hex| u32::from_str_radix(hex, 16).ok())
}

impl ObjectId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn seconds(&self) -> Option<u32> {
        oid_seconds(&self.0)
    }
}

#[derive(Deserialize)]
struct OidRepr {
    #[serde(rename = "$oid")]
    oid: String,
}

impl From<OidRepr> for ObjectId {
    fn from(value: OidRepr) -> Self {
        Self(value.oid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oid_from_extended_json() {
        let oid: ObjectId = serde_json::from_str(r#"{"$oid":"6a9eeb1f000000000000c001"}"#).unwrap();
        assert_eq!(oid.as_str(), "6a9eeb1f000000000000c001");
    }

    #[test]
    fn oid_seconds_from_hex_prefix() {
        assert_eq!(oid_seconds("6a9fb57d000000000000c101"), Some(0x6a9f_b57d));
        assert_eq!(oid_seconds("6a9fb109000000000000c102"), Some(0x6a9f_b109));
        assert!(oid_seconds("6a9fb57d000000000000c101") > oid_seconds("6a9fb109000000000000c102"));
        assert_eq!(oid_seconds("short"), None);
        assert_eq!(oid_seconds("zzzzzzzz14857beef3052fe1"), None);
    }
}
