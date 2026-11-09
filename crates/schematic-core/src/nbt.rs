use std::collections::BTreeMap;

/// Minecraft NBT value. Lives in the core so block entity and entity payloads
/// can be carried through edits independently of any file format.
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

    pub fn as_int_array(&self) -> Option<&[i32]> {
        match self {
            Self::IntArray(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_byte_array(&self) -> Option<&[u8]> {
        match self {
            Self::ByteArray(arr) => Some(arr),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Double(d) => Some(*d),
            Self::Float(f) => Some(*f as f64),
            Self::Int(i) => Some(*i as f64),
            _ => None,
        }
    }
}
