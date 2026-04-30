// this is a pb data file and need to skip linter snakecase warnings.
#![allow(non_snake_case)]

use prost::alloc::borrow::Cow;
use quick_protobuf::{
    sizeofs::*, BytesReader, MessageRead, MessageWrite, Result, Writer, WriterBackend,
};

#[derive(Debug, Default, PartialEq, Clone)]
pub(crate) struct PbData<'a> {
    pub data_type: mod_Data::DataType,
    pub data: Option<Cow<'a, [u8]>>,
    pub file_size: Option<u64>,
    pub block_sizes: Vec<u64>,
    pub hash_type: Option<u64>,
    pub fan_out: Option<u64>,
    pub mode: Option<u32>,
    pub mtime: Option<UnixTime>,
}
impl<'a> MessageRead<'a> for PbData<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.data_type = r.read_enum(bytes)?,
                Ok(18) => msg.data = Some(r.read_bytes(bytes).map(Cow::Borrowed)?),
                Ok(24) => msg.file_size = Some(r.read_uint64(bytes)?),
                Ok(32) => msg.block_sizes.push(r.read_uint64(bytes)?),
                Ok(40) => msg.hash_type = Some(r.read_uint64(bytes)?),
                Ok(48) => msg.fan_out = Some(r.read_uint64(bytes)?),
                Ok(56) => msg.mode = Some(r.read_uint32(bytes)?),
                Ok(66) => msg.mtime = Some(r.read_message::<UnixTime>(bytes)?),
                Ok(t) => {
                    r.read_unknown(bytes, t)?;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}
impl<'a> MessageWrite for PbData<'a> {
    fn get_size(&self) -> usize {
        1 + sizeof_varint(self.data_type as u64)
            + self.data.as_ref().map_or(0, |m| 1 + sizeof_len((m).len()))
            + self.file_size.as_ref().map_or(0, |m| 1 + sizeof_varint(*m))
            + self
                .block_sizes
                .iter()
                .map(|s| 1 + sizeof_varint(*s))
                .sum::<usize>()
            + self.hash_type.as_ref().map_or(0, |m| 1 + sizeof_varint(*m))
            + self.fan_out.as_ref().map_or(0, |m| 1 + sizeof_varint(*m))
            + self
                .mode
                .as_ref()
                .map_or(0, |m| 1 + sizeof_varint(*m as u64))
            + self
                .mtime
                .as_ref()
                .map_or(0, |m| 1 + sizeof_len((m).get_size()))
    }
    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(8, |w| w.write_enum(self.data_type as i32))?;
        if let Some(ref s) = self.data {
            w.write_with_tag(18, |w| w.write_bytes(s))?;
        }
        if let Some(ref s) = self.file_size {
            w.write_with_tag(24, |w| w.write_uint64(*s))?;
        }
        for s in &self.block_sizes {
            w.write_with_tag(32, |w| w.write_uint64(*s))?;
        }
        if let Some(ref s) = self.hash_type {
            w.write_with_tag(40, |w| w.write_uint64(*s))?;
        }
        if let Some(ref s) = self.fan_out {
            w.write_with_tag(48, |w| w.write_uint64(*s))?;
        }
        if let Some(ref s) = self.mode {
            w.write_with_tag(56, |w| w.write_uint32(*s))?;
        }
        if let Some(ref s) = self.mtime {
            w.write_with_tag(66, |w| w.write_message(s))?;
        }
        Ok(())
    }
}

pub mod mod_Data {
    #[derive(Debug, Default, PartialEq, Eq, Clone, Copy)]
    pub enum DataType {
        #[default]
        Raw = 0,
        Directory = 1,
        File = 2,
        Metadata = 3,
        Symlink = 4,
        HAMTShard = 5,
    }
    impl From<i32> for DataType {
        fn from(i: i32) -> Self {
            match i {
                0 => DataType::Raw,
                1 => DataType::Directory,
                2 => DataType::File,
                3 => DataType::Metadata,
                4 => DataType::Symlink,
                5 => DataType::HAMTShard,
                _ => Self::default(),
            }
        }
    }
    impl<'a> From<&'a str> for DataType {
        fn from(s: &'a str) -> Self {
            match s {
                "Raw" => DataType::Raw,
                "Directory" => DataType::Directory,
                "File" => DataType::File,
                "Metadata" => DataType::Metadata,
                "Symlink" => DataType::Symlink,
                "HAMTShard" => DataType::HAMTShard,
                _ => Self::default(),
            }
        }
    }
    impl From<DataType> for i32 {
        fn from(dt: DataType) -> Self {
            match dt {
                DataType::Raw => 0,
                DataType::Directory => 1,
                DataType::File => 2,
                DataType::Metadata => 3,
                DataType::Symlink => 4,
                DataType::HAMTShard => 5,
            }
        }
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub(crate) struct UnixTime {
    pub Seconds: i64,
    pub FractionalNanoseconds: Option<u32>,
}
impl<'a> MessageRead<'a> for UnixTime {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.Seconds = r.read_int64(bytes)?,
                Ok(21) => msg.FractionalNanoseconds = Some(r.read_fixed32(bytes)?),
                Ok(t) => {
                    r.read_unknown(bytes, t)?;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}
impl MessageWrite for UnixTime {
    fn get_size(&self) -> usize {
        1 + sizeof_varint(self.Seconds as u64)
            + self.FractionalNanoseconds.as_ref().map_or(0, |_| 1 + 4)
    }
    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        w.write_with_tag(8, |w| w.write_int64(self.Seconds))?;
        if let Some(ref s) = self.FractionalNanoseconds {
            w.write_with_tag(21, |w| w.write_fixed32(*s))?;
        }
        Ok(())
    }
}
#[derive(Debug, Default, PartialEq, Clone)]
pub(crate) struct Metadata<'a> {
    pub MimeType: Option<Cow<'a, str>>,
}
impl<'a> MessageRead<'a> for Metadata<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.MimeType = Some(r.read_string(bytes).map(Cow::Borrowed)?),
                Ok(t) => {
                    r.read_unknown(bytes, t)?;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}
impl<'a> MessageWrite for Metadata<'a> {
    fn get_size(&self) -> usize {
        self.MimeType
            .as_ref()
            .map_or(0, |m| 1 + sizeof_len((m).len()))
    }
    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.MimeType {
            w.write_with_tag(10, |w| w.write_string(s))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{mod_Data, PbData};
    use quick_protobuf::{BytesReader, MessageRead, MessageWrite, Writer};
    use std::borrow::Cow;

    fn encode(pb: &PbData) -> Vec<u8> {
        let mut out = Vec::new();
        let mut writer = Writer::new(&mut out);
        pb.write_message(&mut writer).unwrap();
        out
    }

    fn decode(bytes: &[u8]) -> PbData<'_> {
        let mut reader = BytesReader::from_bytes(bytes);
        PbData::from_reader(&mut reader, bytes).unwrap()
    }

    #[test]
    fn test_pb_data_roundtrip_minimal() {
        let original = PbData {
            data_type: mod_Data::DataType::File,
            ..Default::default()
        };
        let encoded = encode(&original);
        let decoded = decode(&encoded);
        assert_eq!(decoded.data_type, mod_Data::DataType::File);
        assert_eq!(decoded.data, None);
        assert_eq!(decoded.file_size, None);
        assert!(decoded.block_sizes.is_empty());
    }

    #[test]
    fn test_pb_data_roundtrip_with_fields() {
        let payload: Vec<u8> = b"test chunk data".to_vec();
        let original = PbData {
            data_type: mod_Data::DataType::File,
            data: Some(Cow::Owned(payload.clone())),
            file_size: Some(15),
            block_sizes: vec![5, 5, 5],
            ..Default::default()
        };
        let encoded = encode(&original);
        let decoded = decode(&encoded);
        assert_eq!(decoded.data_type, mod_Data::DataType::File);
        assert_eq!(decoded.data.as_deref(), Some(payload.as_slice()));
        assert_eq!(decoded.file_size, Some(15));
        assert_eq!(decoded.block_sizes, vec![5u64, 5, 5]);
    }

    #[test]
    fn test_pb_data_get_size_matches_encoded_length() {
        let original = PbData {
            data_type: mod_Data::DataType::Raw,
            file_size: Some(100),
            block_sizes: vec![50, 50],
            ..Default::default()
        };
        let encoded = encode(&original);
        assert_eq!(encoded.len(), original.get_size());
    }

    #[test]
    fn test_pb_data_all_data_types_roundtrip() {
        use mod_Data::DataType;
        for dt in [
            DataType::Raw,
            DataType::Directory,
            DataType::File,
            DataType::Metadata,
            DataType::Symlink,
            DataType::HAMTShard,
        ] {
            let original = PbData { data_type: dt, ..Default::default() };
            let encoded = encode(&original);
            let decoded = decode(&encoded);
            assert_eq!(decoded.data_type, dt, "DataType {:?} roundtrip failed", dt);
        }
    }
}
