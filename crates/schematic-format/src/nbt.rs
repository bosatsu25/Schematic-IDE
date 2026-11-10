use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io::{Cursor, Read, Write};

pub use schematic_core::NbtTag;

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
    let payload = read_payload(reader, tag_id, 0)?;
    Ok((name, payload))
}

/// Maximum nesting of compounds/lists (the vanilla limit).
const MAX_DEPTH: usize = 512;

/// Largest decompressed NBT accepted. Far above any real schematic (a dense
/// 10M-block litematic is tens of MiB) and below the wasm32 address space,
/// so a gzip bomb becomes an error instead of an out-of-memory abort.
pub const MAX_DECOMPRESSED_BYTES: u64 = 1 << 30;

/// Reads exactly `len` bytes without trusting `len` for the allocation: a
/// corrupt length on a short stream fails with `UnexpectedEnd` instead of
/// trying to allocate gigabytes.
fn read_bytes<R: Read>(reader: &mut R, len: usize) -> Result<Vec<u8>, NbtError> {
    let mut bytes = Vec::with_capacity(len.min(1 << 20));
    let read = reader.take(len as u64).read_to_end(&mut bytes)?;
    if read != len {
        return Err(NbtError::UnexpectedEnd);
    }
    Ok(bytes)
}

fn read_len<R: Read>(reader: &mut R, what: &str) -> Result<usize, NbtError> {
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf)?;
    let len = i32::from_be_bytes(len_buf);
    usize::try_from(len).map_err(|_| NbtError::Io(format!("Negative {what} length")))
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
    let bytes = read_bytes(reader, len)?;
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

fn read_payload<R: Read>(reader: &mut R, tag_id: u8, depth: usize) -> Result<NbtTag, NbtError> {
    if depth > MAX_DEPTH {
        return Err(NbtError::Io("NBT is nested too deeply".to_string()));
    }
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
            let len = read_len(reader, "byte array")?;
            Ok(NbtTag::ByteArray(read_bytes(reader, len)?))
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
            if item_type == 0 {
                // Zero-size items would let a 4-byte length allocate gigabytes.
                return Err(NbtError::Io("Non-empty list of TAG_End".to_string()));
            }
            let mut items = Vec::with_capacity((len as usize).min(1 << 16));
            for _ in 0..len {
                items.push(read_payload(reader, item_type, depth + 1)?);
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
                let value = read_payload(reader, next_id, depth + 1)?;
                compound.insert(key, value);
            }
            Ok(NbtTag::Compound(compound))
        }
        11 => {
            let len = read_len(reader, "int array")?;
            let byte_len = len
                .checked_mul(4)
                .ok_or_else(|| NbtError::Io("Int array too long".to_string()))?;
            let bytes = read_bytes(reader, byte_len)?;
            let (words, _) = bytes.as_chunks::<4>();
            Ok(NbtTag::IntArray(
                words.iter().map(|b| i32::from_be_bytes(*b)).collect(),
            ))
        }
        12 => {
            let len = read_len(reader, "long array")?;
            let byte_len = len
                .checked_mul(8)
                .ok_or_else(|| NbtError::Io("Long array too long".to_string()))?;
            let bytes = read_bytes(reader, byte_len)?;
            let (words, _) = bytes.as_chunks::<8>();
            Ok(NbtTag::LongArray(
                words.iter().map(|b| i64::from_be_bytes(*b)).collect(),
            ))
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
    decode_with_limit(bytes, MAX_DECOMPRESSED_BYTES)
}

/// Like [`decode_gzip_or_raw`], rejecting gzip streams that inflate to more
/// than `max_decompressed` bytes.
pub fn decode_with_limit(
    bytes: &[u8],
    max_decompressed: u64,
) -> Result<(String, NbtTag), NbtError> {
    if bytes.len() >= 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        let mut decoder = GzDecoder::new(bytes).take(max_decompressed.saturating_add(1));
        let mut decompressed = Vec::new();
        decoder.read_to_end(&mut decompressed)?;
        if decompressed.len() as u64 > max_decompressed {
            return Err(NbtError::Io(format!(
                "Decompressed data exceeds {} MiB",
                max_decompressed >> 20
            )));
        }
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
