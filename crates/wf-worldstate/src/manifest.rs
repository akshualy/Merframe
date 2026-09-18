use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct RawManifestItem {
    pub item_type: String,
    #[serde(default)]
    pub prime_price: Option<u32>,
    #[serde(default)]
    pub regular_price: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManifestItem {
    pub item_type: String,
    pub ducats: Option<u32>,
    pub credits: Option<u32>,
}

impl From<&RawManifestItem> for ManifestItem {
    fn from(raw: &RawManifestItem) -> Self {
        Self {
            item_type: raw.item_type.clone(),
            ducats: raw.prime_price,
            credits: raw.regular_price,
        }
    }
}
