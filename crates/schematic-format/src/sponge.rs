use crate::{
    convert::merge_regions,
    litematic::{double_list, take_double_list, take_string, FormatError},
    nbt::{decode_gzip_or_raw, encode_gzip, NbtTag},
};
use schematic_core::{
    BlockEntityRef, BlockPosition, BlockState, Document, DocumentMetadata, EntityRef, NbtCompound,
    Position, Region, RegionId, Size,
};
use std::collections::BTreeMap;
use std::str::FromStr;

pub fn read_varint(slice: &[u8], offset: &mut usize) -> Result<i32, FormatError> {
    let mut value = 0i32;
    let mut position = 0;
    while *offset < slice.len() {
        let current_byte = slice[*offset];
        *offset += 1;
        value |= ((current_byte & 0x7F) as i32) << position;
        if (current_byte & 0x80) == 0 {
            break;
        }
        position += 7;
        if position >= 35 {
            return Err(FormatError::InvalidStructure(
                "VarInt is too large".to_string(),
            ));
        }
    }
    Ok(value)
}

pub fn write_varint(mut value: i32, out: &mut Vec<u8>) {
    loop {
        if (value & !0x7Fi32) == 0 {
            out.push(value as u8);
            return;
        }
        out.push(((value & 0x7F) | 0x80) as u8);
        value = (value as u32 >> 7) as i32;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpongeSchematic {
    pub data_version: i32,
    pub version: i32,
    pub document: Document,
    pub raw_root_unknown: BTreeMap<String, NbtTag>,
}

const AIR_IDS: [&str; 4] = [
    "minecraft:air",
    "air",
    "minecraft:cave_air",
    "minecraft:void_air",
];

/// Sponge stores dimensions as unsigned shorts.
pub const SPONGE_MAX_DIMENSION: u32 = u16::MAX as u32;

impl SpongeSchematic {
    pub fn from_document(document: Document, data_version: i32) -> Self {
        Self {
            data_version,
            version: 2,
            document,
            raw_root_unknown: BTreeMap::new(),
        }
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, FormatError> {
        let (_root_name, root_tag) = decode_gzip_or_raw(bytes)?;
        let root_compound = root_tag.as_compound().ok_or_else(|| {
            FormatError::InvalidStructure("Root tag is not a Compound".to_string())
        })?;

        // v3 wraps everything in a "Schematic" compound; v1/v2 use the root.
        let schem =
            if let Some(inner) = root_compound.get("Schematic").and_then(|t| t.as_compound()) {
                inner
            } else {
                root_compound
            };

        let version = schem.get("Version").and_then(|t| t.as_i32()).unwrap_or(2);
        let data_version = schem
            .get("DataVersion")
            .and_then(|t| t.as_i32())
            .unwrap_or(2975);

        let dim = |key: &str| {
            schem
                .get(key)
                .and_then(|t| match t {
                    // Unsigned short stored in a signed NBT short.
                    NbtTag::Short(v) => Some(*v as u16 as u32),
                    other => other.as_i32().map(|i| i.max(0) as u32),
                })
                .ok_or_else(|| FormatError::MissingField(key.to_string()))
        };
        let width = dim("Width")?;
        let height = dim("Height")?;
        let length = dim("Length")?;

        let origin = match schem.get("Offset").and_then(|t| t.as_int_array()) {
            Some(offset) if offset.len() >= 3 => Position::new(offset[0], offset[1], offset[2]),
            _ => Position::new(0, 0, 0),
        };

        let mut metadata = DocumentMetadata {
            name: Some("SpongeSchematic".to_string()),
            ..Default::default()
        };
        if let Some(meta) = schem.get("Metadata").and_then(|t| t.as_compound()) {
            if let Some(name) = meta.get("Name").and_then(|t| t.as_string()) {
                metadata.name = Some(name.to_string());
            }
            if let Some(author) = meta.get("Author").and_then(|t| t.as_string()) {
                metadata.author = Some(author.to_string());
            }
            if let Some(date) = meta.get("Date").and_then(|t| t.as_i64()) {
                metadata.created_at_unix_ms = Some(date);
            }
        }
        let mut doc = Document::new(metadata);

        let region_id = RegionId::new("Main");
        let size = Size::new(width, height, length);
        let mut region = Region::new(region_id.clone(), origin, size);

        // v3 nests block data in a "Blocks" container.
        let blocks_container = schem.get("Blocks").and_then(|t| t.as_compound());
        let (palette_compound, block_data, be_list) = if let Some(blocks) = blocks_container {
            (
                blocks.get("Palette").and_then(|t| t.as_compound()),
                blocks.get("Data").and_then(|t| t.as_byte_array()),
                blocks.get("BlockEntities").and_then(|t| t.as_list()),
            )
        } else {
            (
                schem.get("Palette").and_then(|t| t.as_compound()),
                schem.get("BlockData").and_then(|t| t.as_byte_array()),
                schem
                    .get("BlockEntities")
                    .or_else(|| schem.get("TileEntities"))
                    .and_then(|t| t.as_list()),
            )
        };
        let palette_compound =
            palette_compound.ok_or_else(|| FormatError::MissingField("Palette".to_string()))?;
        let block_data =
            block_data.ok_or_else(|| FormatError::MissingField("BlockData".to_string()))?;

        let mut palette_map: BTreeMap<i32, BlockState> = BTreeMap::new();
        for (state_str, id_tag) in palette_compound {
            let id = id_tag.as_i32().ok_or_else(|| {
                FormatError::InvalidStructure(format!("Invalid palette id for {state_str}"))
            })?;
            let state = BlockState::from_str(state_str).map_err(|e| {
                FormatError::InvalidStructure(format!("Invalid blockstate '{state_str}': {e}"))
            })?;
            palette_map.insert(id, state);
        }

        let max_id = palette_map.keys().copied().max().unwrap_or(0).max(0);
        let mut palette_states: Vec<Option<BlockState>> = vec![None; (max_id + 1) as usize];
        for (id, state) in palette_map {
            if id >= 0 {
                palette_states[id as usize] = Some(state);
            }
        }

        let mut region_pal_indices = Vec::with_capacity(palette_states.len());
        for st_opt in &palette_states {
            match st_opt {
                Some(st) if !AIR_IDS.contains(&st.id()) => {
                    let idx = region.palette_mut().intern(st.clone()).map_err(|e| {
                        FormatError::InvalidStructure(format!("Palette error: {e}"))
                    })?;
                    region_pal_indices.push(Some(idx));
                }
                _ => region_pal_indices.push(None),
            }
        }

        let total_blocks = (width as usize) * (height as usize) * (length as usize);
        let mut offset = 0;
        let mut idx = 0;
        while offset < block_data.len() && idx < total_blocks {
            let pal_id = read_varint(block_data, &mut offset)? as usize;
            if let Some(Some(pal_idx)) = region_pal_indices.get(pal_id) {
                let x = (idx % (width as usize)) as i64;
                let z = ((idx / (width as usize)) % (length as usize)) as i64;
                let y = (idx / ((width as usize) * (length as usize))) as i64;
                let _ = region.set_block_index(BlockPosition::new(x, y, z), Some(*pal_idx));
            }
            idx += 1;
        }

        let nested = version >= 3;
        for be_tag in be_list.into_iter().flatten() {
            let Some(be_compound) = be_tag.as_compound() else {
                continue;
            };
            let (kind, pos, data) = split_block_entity(be_compound, nested);
            doc.add_block_entity(BlockEntityRef::new(region_id.clone(), kind, pos).with_data(data));
        }

        for ent_tag in schem
            .get("Entities")
            .and_then(|t| t.as_list())
            .into_iter()
            .flatten()
        {
            let Some(ent_compound) = ent_tag.as_compound() else {
                continue;
            };
            let mut data = if nested {
                ent_compound
                    .get("Data")
                    .and_then(|t| t.as_compound())
                    .cloned()
                    .unwrap_or_default()
            } else {
                ent_compound.clone()
            };
            let mut outer = ent_compound.clone();
            let kind = take_string(&mut outer, &["Id", "id"])
                .or_else(|| take_string(&mut data, &["Id", "id"]))
                .unwrap_or_else(|| "unknown".to_string());
            let pos = take_double_list(&mut outer, "Pos").unwrap_or([0.0; 3]);
            if !nested {
                data = outer;
            }
            data.remove("Pos");
            data.remove("Id");
            data.remove("id");
            doc.add_entity(EntityRef::new(region_id.clone(), kind, pos).with_data(data));
        }

        doc.insert_region(region);

        let mut raw_root_unknown = BTreeMap::new();
        for (k, v) in schem {
            if !matches!(
                k.as_str(),
                "Version"
                    | "DataVersion"
                    | "Width"
                    | "Height"
                    | "Length"
                    | "Offset"
                    | "Palette"
                    | "PaletteMax"
                    | "BlockData"
                    | "Blocks"
                    | "BlockEntities"
                    | "TileEntities"
                    | "Entities"
            ) {
                raw_root_unknown.insert(k.clone(), v.clone());
            }
        }

        Ok(Self {
            data_version,
            version,
            document: doc,
            raw_root_unknown,
        })
    }

    /// Exports as Sponge v3 when the source was v3, otherwise v2. Multi-region
    /// documents are flattened (see [`crate::convert::merge_regions`]).
    pub fn export(&self, doc: &Document) -> Result<Vec<u8>, FormatError> {
        let merged = merge_regions(doc)?;
        let doc = &merged.document;
        let region = doc.regions().next().ok_or_else(|| {
            FormatError::InvalidStructure("Document contains no regions".to_string())
        })?;

        let width = region.size().x;
        let height = region.size().y;
        let length = region.size().z;
        if width > SPONGE_MAX_DIMENSION
            || height > SPONGE_MAX_DIMENSION
            || length > SPONGE_MAX_DIMENSION
        {
            return Err(FormatError::InvalidStructure(format!(
                "Sponge schematics are limited to {SPONGE_MAX_DIMENSION} blocks per axis; got {width}x{height}x{length}"
            )));
        }
        let v3 = self.version >= 3;

        let mut schem = BTreeMap::new();
        schem.insert("Version".to_string(), NbtTag::Int(if v3 { 3 } else { 2 }));
        schem.insert("DataVersion".to_string(), NbtTag::Int(self.data_version));
        schem.insert("Width".to_string(), NbtTag::Short(width as u16 as i16));
        schem.insert("Height".to_string(), NbtTag::Short(height as u16 as i16));
        schem.insert("Length".to_string(), NbtTag::Short(length as u16 as i16));
        schem.insert(
            "Offset".to_string(),
            NbtTag::IntArray(vec![
                region.origin().x,
                region.origin().y,
                region.origin().z,
            ]),
        );

        // Palette: air is always id 0.
        let mut palette_compound = BTreeMap::new();
        palette_compound.insert("minecraft:air".to_string(), NbtTag::Int(0));
        let mut index_to_sponge: Vec<i32> = Vec::with_capacity(region.palette().len());
        let mut by_name: BTreeMap<String, i32> = BTreeMap::new();
        by_name.insert("minecraft:air".to_string(), 0);
        for (_pal_idx, state) in region.palette().iter() {
            let key = state.to_string();
            let next = by_name.len() as i32;
            let id = *by_name.entry(key.clone()).or_insert(next);
            palette_compound.insert(key, NbtTag::Int(id));
            index_to_sponge.push(id);
        }

        let total_blocks = (width as usize) * (height as usize) * (length as usize);
        let mut block_data_bytes = Vec::with_capacity(total_blocks);
        for y in 0..(height as i64) {
            for z in 0..(length as i64) {
                for x in 0..(width as i64) {
                    let sponge_id = region
                        .block_index_at(BlockPosition::new(x, y, z))
                        .ok()
                        .flatten()
                        .and_then(|idx| index_to_sponge.get(idx.get() as usize).copied())
                        .unwrap_or(0);
                    write_varint(sponge_id, &mut block_data_bytes);
                }
            }
        }

        let mut be_list = Vec::new();
        for be in doc.block_entities_in(region.id()) {
            let mut compound = if v3 {
                let mut c = BTreeMap::new();
                if !be.data.is_empty() {
                    c.insert("Data".to_string(), NbtTag::Compound(be.data.clone()));
                }
                c
            } else {
                be.data.clone()
            };
            compound.insert("Id".to_string(), NbtTag::String(be.kind.clone()));
            compound.insert(
                "Pos".to_string(),
                NbtTag::IntArray(vec![
                    be.position.x as i32,
                    be.position.y as i32,
                    be.position.z as i32,
                ]),
            );
            be_list.push(NbtTag::Compound(compound));
        }

        let mut ent_list = Vec::new();
        for ent in doc.entities_in(region.id()) {
            let mut compound = if v3 {
                let mut c = BTreeMap::new();
                let mut data = ent.data.clone();
                data.insert("id".to_string(), NbtTag::String(ent.kind.clone()));
                c.insert("Data".to_string(), NbtTag::Compound(data));
                c
            } else {
                ent.data.clone()
            };
            compound.insert("Id".to_string(), NbtTag::String(ent.kind.clone()));
            compound.insert("Pos".to_string(), double_list(ent.position));
            ent_list.push(NbtTag::Compound(compound));
        }

        if v3 {
            let mut blocks = BTreeMap::new();
            blocks.insert("Palette".to_string(), NbtTag::Compound(palette_compound));
            blocks.insert("Data".to_string(), NbtTag::ByteArray(block_data_bytes));
            blocks.insert("BlockEntities".to_string(), NbtTag::List(10, be_list));
            schem.insert("Blocks".to_string(), NbtTag::Compound(blocks));
        } else {
            schem.insert(
                "PaletteMax".to_string(),
                NbtTag::Int(palette_compound.len() as i32),
            );
            schem.insert("Palette".to_string(), NbtTag::Compound(palette_compound));
            schem.insert("BlockData".to_string(), NbtTag::ByteArray(block_data_bytes));
            schem.insert("BlockEntities".to_string(), NbtTag::List(10, be_list));
        }
        schem.insert("Entities".to_string(), NbtTag::List(10, ent_list));

        // Metadata: keep unknown keys from the source, refresh Name/Author.
        let mut meta = self
            .raw_root_unknown
            .get("Metadata")
            .and_then(|t| t.as_compound())
            .cloned()
            .unwrap_or_default();
        if let Some(name) = &doc.metadata().name {
            meta.insert("Name".to_string(), NbtTag::String(name.clone()));
        }
        if let Some(author) = &doc.metadata().author {
            meta.insert("Author".to_string(), NbtTag::String(author.clone()));
        }
        if let Some(date) = doc.metadata().created_at_unix_ms {
            meta.insert("Date".to_string(), NbtTag::Long(date));
        }
        if !meta.is_empty() {
            schem.insert("Metadata".to_string(), NbtTag::Compound(meta));
        }

        for (k, v) in &self.raw_root_unknown {
            if !schem.contains_key(k) {
                schem.insert(k.clone(), v.clone());
            }
        }

        if v3 {
            let mut root = BTreeMap::new();
            root.insert("Schematic".to_string(), NbtTag::Compound(schem));
            encode_gzip("", &NbtTag::Compound(root)).map_err(FormatError::Nbt)
        } else {
            encode_gzip("Schematic", &NbtTag::Compound(schem)).map_err(FormatError::Nbt)
        }
    }
}

/// Splits a Sponge block entity compound into `(kind, position, payload)`.
fn split_block_entity(
    compound: &NbtCompound,
    nested: bool,
) -> (String, BlockPosition, NbtCompound) {
    let mut outer = compound.clone();
    let kind = take_string(&mut outer, &["Id", "id"]).unwrap_or_else(|| "unknown".to_string());
    let pos = match outer.remove("Pos") {
        Some(NbtTag::IntArray(p)) if p.len() >= 3 => {
            BlockPosition::new(p[0] as i64, p[1] as i64, p[2] as i64)
        }
        _ => {
            let get = |k: &str| outer.get(k).and_then(|t| t.as_i32()).unwrap_or(0) as i64;
            let p = BlockPosition::new(get("x"), get("y"), get("z"));
            outer.remove("x");
            outer.remove("y");
            outer.remove("z");
            p
        }
    };
    let data = if nested {
        outer
            .get("Data")
            .and_then(|t| t.as_compound())
            .cloned()
            .unwrap_or_default()
    } else {
        outer
    };
    (kind, pos, data)
}
