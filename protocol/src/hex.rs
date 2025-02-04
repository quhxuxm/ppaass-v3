use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};

pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
    let hex_string = hex::encode(v);
    String::serialize(&hex_string, s)
}

pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Bytes, D::Error> {
    let hex_string = String::deserialize(d)?;
    hex::decode(hex_string)
        .map(Into::into)
        .map_err(|e| serde::de::Error::custom(e))
}
