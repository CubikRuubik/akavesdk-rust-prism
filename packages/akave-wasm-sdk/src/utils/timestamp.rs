use prost_types::Timestamp;
use serde::{Deserialize, Deserializer, Serializer};
use std::time::{SystemTime, UNIX_EPOCH};

/// Custom serialization module for prost_types::Timestamp
pub mod timestamp_serde {
    use super::*;

    /// Serialize a prost_types::Timestamp to RFC3339 string format
    pub fn serialize<S>(timestamp: &Option<Timestamp>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match timestamp {
            Some(ts) => {
                // Use the time crate for WASM-friendly time operations
                use time::OffsetDateTime;
                
                let datetime = OffsetDateTime::from_unix_timestamp(ts.seconds)
                    .map_err(|e| serde::ser::Error::custom(format!("Invalid timestamp: {}", e)))?
                    .replace_nanosecond(ts.nanos as u32)
                    .map_err(|e| serde::ser::Error::custom(format!("Invalid nanoseconds: {}", e)))?;
                
                let formatted = datetime.format(&time::format_description::well_known::Rfc3339)
                    .map_err(|e| serde::ser::Error::custom(format!("Formatting error: {}", e)))?;
                
                serializer.serialize_str(&formatted)
            }
            None => serializer.serialize_none(),
        }
    }

    /// Deserialize from RFC3339 string to prost_types::Timestamp
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Timestamp>, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Use Option<String> to handle null values
        let opt_str = Option::<String>::deserialize(deserializer)?;
        
        match opt_str {
            Some(s) => {
                use time::OffsetDateTime;
                
                let datetime = OffsetDateTime::parse(&s, &time::format_description::well_known::Rfc3339)
                    .map_err(|e| serde::de::Error::custom(format!("Parse error: {}", e)))?;
                
                let timestamp = Timestamp {
                    seconds: datetime.unix_timestamp(),
                    nanos: datetime.nanosecond() as i32,
                };
                
                Ok(Some(timestamp))
            }
            None => Ok(None),
        }
    }
}

/// Serialization module for Timestamp that doesn't wrap in an Option
pub mod timestamp_serde_direct {
    use super::*;

    /// Serialize a prost_types::Timestamp to RFC3339 string format
    pub fn serialize<S>(timestamp: &Timestamp, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use time::OffsetDateTime;
        
        let datetime = OffsetDateTime::from_unix_timestamp(timestamp.seconds)
            .map_err(|e| serde::ser::Error::custom(format!("Invalid timestamp: {}", e)))?
            .replace_nanosecond(timestamp.nanos as u32)
            .map_err(|e| serde::ser::Error::custom(format!("Invalid nanoseconds: {}", e)))?;
        
        let formatted = datetime.format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| serde::ser::Error::custom(format!("Formatting error: {}", e)))?;
        
        serializer.serialize_str(&formatted)
    }

    /// Deserialize from RFC3339 string to prost_types::Timestamp
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Timestamp, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        
        use time::OffsetDateTime;
        
        let datetime = OffsetDateTime::parse(&s, &time::format_description::well_known::Rfc3339)
            .map_err(|e| serde::de::Error::custom(format!("Parse error: {}", e)))?;
        
        let timestamp = Timestamp {
            seconds: datetime.unix_timestamp(),
            nanos: datetime.nanosecond() as i32,
        };
        
        Ok(timestamp)
    }
}

/// Helper function to create a current timestamp
pub fn now() -> Timestamp {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    
    Timestamp {
        seconds: now.as_secs() as i64,
        nanos: now.subsec_nanos() as i32,
    }
}

/// Helper function to convert Timestamp to RFC3339 string
pub fn to_rfc3339(timestamp: &Timestamp) -> Result<String, String> {
    use time::OffsetDateTime;
    
    let datetime = OffsetDateTime::from_unix_timestamp(timestamp.seconds)
        .map_err(|e| format!("Invalid timestamp: {}", e))?
        .replace_nanosecond(timestamp.nanos as u32)
        .map_err(|e| format!("Invalid nanoseconds: {}", e))?;
    
    datetime.format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| format!("Formatting error: {}", e))
}

/// Helper function to convert RFC3339 string to Timestamp
pub fn from_rfc3339(s: &str) -> Result<Timestamp, String> {
    use time::OffsetDateTime;
    
    let datetime = OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
        .map_err(|e| format!("Parse error: {}", e))?;
    
    Ok(Timestamp {
        seconds: datetime.unix_timestamp(),
        nanos: datetime.nanosecond() as i32,
    })
}