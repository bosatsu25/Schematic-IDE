use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{Cursor, Read, Write};

#[derive(Clone, Debug, PartialEq)]
pub enum NbtTag {
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<u8>),
    String(String),
    List(u8, Vec<NbtTag>),
    Compound(BTreeMap<String, NbtTag>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl NbtTag {
    pub fn tag_id(&self) -> u8 {
        match self {
            Self::End => 0,
            Self::Byte(_) => 1,
            Self::Short(_) => 2,
            Self::Int(_) => 3,
            Self::Long(_) => 4,
            Self::Float(_) => 5,
            Self::Double(_) => 6,
            Self::ByteArray(_) => 7,
            Self::String(_) => 8,
            Self::List(_, _) => 9,
            Self::Compound(_) => 10,
            Self::IntArray(_) => 11,
            Self::LongArray(_) => 12,
        }
    }

    pub fn as_compound(&self) -> Option<&BTreeMap<String, NbtTag>> {
        match self {
            Self::Compound(map) => Some(map),
            _ => None,
        }
    }

    pub fn as_compound_mut(&mut self) -> Option<&mut BTreeMap<String, NbtTag>> {
        match self {
            Self::Compound(map) => Some(map),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i32(&self) -> Option<i32> {
        match self {
            Self::Int(i) => Some(*i),
            Self::Short(s) => Some(*s as i32),
            Self::Byte(b) => Some(*b as i32),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Long(l) => Some(*l),
            Self::Int(i) => Some(*i as i64),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[NbtTag]> {
        match self {
            Self::List(_, list) => Some(list),
            _ => None,
        }
    }

    pub fn as_long_array(&self) -> Option<&[i64]> {
        match self {
            Self::LongArray(arr) => Some(arr),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NbtError {
    Io(String),
    InvalidTagId(u8),
    InvalidUtf8,
    UnexpectedEnd,
}

impl Display for NbtError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(msg) => write!(formatter, "NBT I/O error: {msg}"),
            Self::InvalidTagId(id) => write!(formatter, "Invalid NBT tag ID: {id}"),
            Self::InvalidUtf8 => formatter.write_str("Invalid UTF-8 string in NBT"),
            Self::UnexpectedEnd => formatter.write_str("Unexpected end of NBT stream"),
        }
    }
}

impl Error for NbtError {}

impl From<std::io::Error> for NbtError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err.to_string())
    }
}

pub fn read_nbt<R: Read>(reader: &mut R) -> Result<(String, NbtTag), NbtError> {
    let mut tag_id_buf = [0u8; 1];
    reader.read_exact(&mut tag_id_buf)?;
    let tag_id = tag_id_buf[0];
    if tag_id == 0 {
        return Ok((String::new(), NbtTag::End));
    }
    let name = read_string_payload(reader)?;
    let payload = read_payload(reader, tag_id)?;
    Ok((name, payload))
}

pub fn write_nbt<W: Write>(writer: &mut W, name: &str, tag: &NbtTag) -> Result<(), NbtError> {
    let tag_id = tag.tag_id();
    writer.write_all(&[tag_id])?;
    if tag_id == 0 {
        return Ok(());
    }
    write_string_payload(writer, name)?;
    write_payload(writer, tag)?;
    Ok(())
}

fn read_string_payload<R: Read>(reader: &mut R) -> Result<String, NbtError> {
    let mut len_buf = [0u8; 2];
    reader.read_exact(&mut len_buf)?;
    let len = u16::from_be_bytes(len_buf) as usize;
    let mut bytes = vec![0u8; len];
    reader.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(|_| NbtError::InvalidUtf8)
}

fn write_string_payload<W: Write>(writer: &mut W, value: &str) -> Result<(), NbtError> {
    let bytes = value.as_bytes();
    let len =
        u16::try_from(bytes.len()).map_err(|_| NbtError::Io("String too long".to_string()))?;
    writer.write_all(&len.to_be_bytes())?;
    writer.write_all(bytes)?;
    Ok(())
}

fn read_payload<R: Read>(reader: &mut R, tag_id: u8) -> Result<NbtTag, NbtError> {
    match tag_id {
        0 => Ok(NbtTag::End),
        1 => {
            let mut buf = [0u8; 1];
            reader.read_exact(&mut buf)?;
            Ok(NbtTag::Byte(buf[0] as i8))
        }
        2 => {
            let mut buf = [0u8; 2];
            reader.read_exact(&mut buf)?;
            Ok(NbtTag::Short(i16::from_be_bytes(buf)))
        }
        3 => {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            Ok(NbtTag::Int(i32::from_be_bytes(buf)))
        }
        4 => {
            let mut buf = [0u8; 8];
            reader.read_exact(&mut buf)?;
            Ok(NbtTag::Long(i64::from_be_bytes(buf)))
        }
        5 => {
            let mut buf = [0u8; 4];
            reader.read_exact(&mut buf)?;
            Ok(NbtTag::Float(f32::from_be_bytes(buf)))
        }
        6 => {
            let mut buf = [0u8; 8];
            reader.read_exact(&mut buf)?;
            Ok(NbtTag::Double(f64::from_be_bytes(buf)))
        }
        7 => {
            let mut len_buf = [0u8; 4];
            reader.read_exact(&mut len_buf)?;
            let len = i32::from_be_bytes(len_buf);
            if len < 0 {
                return Err(NbtError::Io("Negative byte array length".to_string()));
            }
            let mut bytes = vec![0u8; len as usize];
            reader.read_exact(&mut bytes)?;
            Ok(NbtTag::ByteArray(bytes))
        }
        8 => {
            let s = read_string_payload(reader)?;
            Ok(NbtTag::String(s))
        }
        9 => {
            let mut item_type_buf = [0u8; 1];
            reader.read_exact(&mut item_type_buf)?;
            let item_type = item_type_buf[0];
            let mut len_buf = [0u8; 4];
            reader.read_exact(&mut len_buf)?;
            let len = i32::from_be_bytes(len_buf);
            if len <= 0 {
                return Ok(NbtTag::List(item_type, Vec::new()));
            }
            let mut items = Vec::with_capacity(len as usize);
            for _ in 0..len {
                items.push(read_payload(reader, item_type)?);
            }
            Ok(NbtTag::List(item_type, items))
        }
        10 => {
            let mut compound = BTreeMap::new();
            loop {
                let mut next_id_buf = [0u8; 1];
                reader.read_exact(&mut next_id_buf)?;
                let next_id = next_id_buf[0];
                if next_id == 0 {
                    break;
                }
                let key = read_string_payload(reader)?;
                let value = read_payload(reader, next_id)?;
                compound.insert(key, value);
            }
            Ok(NbtTag::Compound(compound))
        }
        11 => {
            let mut len_buf = [0u8; 4];
            reader.read_exact(&mut len_buf)?;
            let len = i32::from_be_bytes(len_buf);
            if len < 0 {
                return Err(NbtError::Io("Negative int array length".to_string()));
            }
            let mut items = Vec::with_capacity(len as usize);
            for _ in 0..len {
                let mut buf = [0u8; 4];
                reader.read_exact(&mut buf)?;
                items.push(i32::from_be_bytes(buf));
            }
            Ok(NbtTag::IntArray(items))
        }
        12 => {
            let mut len_buf = [0u8; 4];
            reader.read_exact(&mut len_buf)?;
            let len = i32::from_be_bytes(len_buf);
            if len < 0 {
                return Err(NbtError::Io("Negative long array length".to_string()));
            }
            let mut items = Vec::with_capacity(len as usize);
            for _ in 0..len {
                let mut buf = [0u8; 8];
                reader.read_exact(&mut buf)?;
                items.push(i64::from_be_bytes(buf));
            }
            Ok(NbtTag::LongArray(items))
        }
        other => Err(NbtError::InvalidTagId(other)),
    }
}

fn write_payload<W: Write>(writer: &mut W, tag: &NbtTag) -> Result<(), NbtError> {
    match tag {
        NbtTag::End => Ok(()),
        NbtTag::Byte(b) => {
            writer.write_all(&[*b as u8])?;
            Ok(())
        }
        NbtTag::Short(s) => {
            writer.write_all(&s.to_be_bytes())?;
            Ok(())
        }
        NbtTag::Int(i) => {
            writer.write_all(&i.to_be_bytes())?;
            Ok(())
        }
        NbtTag::Long(l) => {
            writer.write_all(&l.to_be_bytes())?;
            Ok(())
        }
        NbtTag::Float(f) => {
            writer.write_all(&f.to_be_bytes())?;
            Ok(())
        }
        NbtTag::Double(d) => {
            writer.write_all(&d.to_be_bytes())?;
            Ok(())
        }
        NbtTag::ByteArray(arr) => {
            let len = i32::try_from(arr.len())
                .map_err(|_| NbtError::Io("Byte array too long".to_string()))?;
            writer.write_all(&len.to_be_bytes())?;
            writer.write_all(arr)?;
            Ok(())
        }
        NbtTag::String(s) => {
            write_string_payload(writer, s)?;
            Ok(())
        }
        NbtTag::List(item_type, items) => {
            writer.write_all(&[*item_type])?;
            let len = i32::try_from(items.len())
                .map_err(|_| NbtError::Io("List too long".to_string()))?;
            writer.write_all(&len.to_be_bytes())?;
            for item in items {
                write_payload(writer, item)?;
            }
            Ok(())
        }
        NbtTag::Compound(entries) => {
            for (key, val) in entries {
                writer.write_all(&[val.tag_id()])?;
                write_string_payload(writer, key)?;
                write_payload(writer, val)?;
            }
            writer.write_all(&[0u8])?; // TAG_End
            Ok(())
        }
        NbtTag::IntArray(arr) => {
            let len = i32::try_from(arr.len())
                .map_err(|_| NbtError::Io("Int array too long".to_string()))?;
            writer.write_all(&len.to_be_bytes())?;
            for val in arr {
                writer.write_all(&val.to_be_bytes())?;
            }
            Ok(())
        }
        NbtTag::LongArray(arr) => {
            let len = i32::try_from(arr.len())
                .map_err(|_| NbtError::Io("Long array too long".to_string()))?;
            writer.write_all(&len.to_be_bytes())?;
            for val in arr {
                writer.write_all(&val.to_be_bytes())?;
            }
            Ok(())
        }
    }
}

pub fn decode_gzip_or_raw(bytes: &[u8]) -> Result<(String, NbtTag), NbtError> {
    if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        let mut decoder = GzDecoder::new(bytes);
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        let mut cursor = Cursor::new(decompressed);
        read_nbt(&mut cursor)
    } else {
        let mut cursor = Cursor::new(bytes);
        read_nbt(&mut cursor)
    }
}

pub fn encode_gzip(name: &str, tag: &NbtTag) -> Result<Vec<u8>, NbtError> {
    let mut raw_bytes = Vec::new();
    write_nbt(&mut raw_bytes, name, tag)?;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&raw_bytes)?;
    let compressed = encoder.finish()?;
    Ok(compressed)
}
