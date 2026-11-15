use std::collections::HashMap;

use serde::Deserialize;

use crate::error::{DataError, Result};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(transparent)]
pub struct Dictionary {
    texts: HashMap<String, String>,
}

impl Dictionary {
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|source| DataError::Parse("dictionary", source))
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.texts.get(key).map(String::as_str)
    }
}
