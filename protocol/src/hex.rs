use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};

pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
    let hex_string = hex::encode(v);
    String::serialize(&hex_string, s)
}

pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    let hex_string = String::deserialize(d)?;
    hex::decode(hex_string.as_bytes()).map_err(|e| serde::de::Error::custom(e))
}
