use crate::{
    litematic::FormatError,
    nbt::{decode_gzip_or_raw, encode_gzip, NbtTag},
};
use schematic_core::{
    BlockEntityRef, BlockPosition, BlockState, Document, DocumentMetadata, EntityRef, Position,
    Region, RegionId, Size,
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

        // In Sponge schematic, root can either be "Schematic" compound, or root itself
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

        let width = schem
            .get("Width")
            .and_then(|t| t.as_i32().map(|i| i as u32))
            .ok_or_else(|| FormatError::MissingField("Width".to_string()))?;

        let height = schem
            .get("Height")
            .and_then(|t| t.as_i32().map(|i| i as u32))
            .ok_or_else(|| FormatError::MissingField("Height".to_string()))?;

        let length = schem
            .get("Length")
            .and_then(|t| t.as_i32().map(|i| i as u32))
            .ok_or_else(|| FormatError::MissingField("Length".to_string()))?;

        let origin = if let Some(offset) = schem.get("Offset").and_then(|t| t.as_int_array()) {
            if offset.len() >= 3 {
                Position::new(offset[0], offset[1], offset[2])
            } else {
                Position::new(0, 0, 0)
            }
        } else {
            Position::new(0, 0, 0)
        };

        let mut doc = Document::new(DocumentMetadata {
            name: Some("SpongeSchematic".to_string()),
            ..Default::default()
        });

        let region_id = RegionId::new("Main");
        let size = Size::new(width, height, length);
        let mut region = Region::new(region_id.clone(), origin, size);

        // Parse Palette
        let palette_compound = schem
            .get("Palette")
            .and_then(|t| t.as_compound())
            .ok_or_else(|| FormatError::MissingField("Palette".to_string()))?;

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

        let max_id = palette_map.keys().copied().max().unwrap_or(0);
        let mut palette_states: Vec<Option<BlockState>> = vec![None; (max_id + 1) as usize];
        for (id, state) in palette_map {
            if id >= 0 {
                palette_states[id as usize] = Some(state);
            }
        }

        // Intern states into region palette
        let mut region_pal_indices = Vec::with_capacity(palette_states.len());
        for st_opt in &palette_states {
            if let Some(st) = st_opt {
                let idx = region
                    .palette_mut()
                    .intern(st.clone())
                    .map_err(|e| FormatError::InvalidStructure(format!("Palette error: {e}")))?;
                region_pal_indices.push(Some(idx));
            } else {
                region_pal_indices.push(None);
            }
        }

        // Decode BlockData
        let block_data = schem
            .get("BlockData")
            .and_then(|t| t.as_byte_array())
            .ok_or_else(|| FormatError::MissingField("BlockData".to_string()))?;

        let total_blocks = (width as usize) * (height as usize) * (length as usize);
        let mut offset = 0;
        let mut idx = 0;

        while offset < block_data.len() && idx < total_blocks {
            let pal_id = read_varint(block_data, &mut offset)? as usize;
            if pal_id < palette_states.len() {
                if let Some(state) = &palette_states[pal_id] {
                    if state.id() != "minecraft:air"
                        && state.id() != "air"
                        && state.id() != "minecraft:cave_air"
                        && state.id() != "minecraft:void_air"
                    {
                        let x = (idx % (width as usize)) as i64;
                        let z = ((idx / (width as usize)) % (length as usize)) as i64;
                        let y = (idx / ((width as usize) * (length as usize))) as i64;
                        let pal_idx = region_pal_indices[pal_id];
                        let _ = region.set_block_index(BlockPosition::new(x, y, z), pal_idx);
                    }
                }
            }
            idx += 1;
        }

        // Parse BlockEntities
        if let Some(be_list) = schem.get("BlockEntities").and_then(|t| t.as_list()) {
            for be_tag in be_list {
                if let Some(be_compound) = be_tag.as_compound() {
                    let id_str = be_compound
                        .get("Id")
                        .or_else(|| be_compound.get("id"))
                        .and_then(|t| t.as_string())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "minecraft:chest".to_string());

                    let pos = if let Some(pos_arr) =
                        be_compound.get("Pos").and_then(|t| t.as_int_array())
                    {
                        if pos_arr.len() >= 3 {
                            Position::new(pos_arr[0], pos_arr[1], pos_arr[2])
                        } else {
                            Position::new(0, 0, 0)
                        }
                    } else {
                        let x = be_compound.get("x").and_then(|t| t.as_i32()).unwrap_or(0);
                        let y = be_compound.get("y").and_then(|t| t.as_i32()).unwrap_or(0);
                        let z = be_compound.get("z").and_then(|t| t.as_i32()).unwrap_or(0);
                        Position::new(x, y, z)
                    };

                    doc.add_block_entity(BlockEntityRef::new(id_str, pos));
                }
            }
        }

        // Parse Entities
        if let Some(ent_list) = schem.get("Entities").and_then(|t| t.as_list()) {
            for ent_tag in ent_list {
                if let Some(ent_compound) = ent_tag.as_compound() {
                    let id_str = ent_compound
                        .get("Id")
                        .or_else(|| ent_compound.get("id"))
                        .and_then(|t| t.as_string())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "minecraft:pig".to_string());

                    let pos =
                        if let Some(pos_list) = ent_compound.get("Pos").and_then(|t| t.as_list()) {
                            let mut coords = [0.0f64; 3];
                            for (i, p) in pos_list.iter().take(3).enumerate() {
                                coords[i] = p.as_f64().unwrap_or(0.0);
                            }
                            coords
                        } else {
                            [0.0, 0.0, 0.0]
                        };

                    doc.add_entity(EntityRef::new(id_str, pos));
                }
            }
        }

        doc.insert_region(region);

        // Unknown tags
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
                    | "BlockData"
                    | "BlockEntities"
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

    pub fn export(&self, doc: &Document) -> Result<Vec<u8>, FormatError> {
        let region = doc.regions().next().ok_or_else(|| {
            FormatError::InvalidStructure("Document contains no regions".to_string())
        })?;

        let width = region.size().x;
        let height = region.size().y;
        let length = region.size().z;

        let mut schem = BTreeMap::new();
        schem.insert("Version".to_string(), NbtTag::Int(self.version));
        schem.insert("DataVersion".to_string(), NbtTag::Int(self.data_version));
        schem.insert("Width".to_string(), NbtTag::Short(width as i16));
        schem.insert("Height".to_string(), NbtTag::Short(height as i16));
        schem.insert("Length".to_string(), NbtTag::Short(length as i16));
        schem.insert(
            "Offset".to_string(),
            NbtTag::IntArray(vec![
                region.origin().x,
                region.origin().y,
                region.origin().z,
            ]),
        );

        // Build palette
        let mut palette_compound = BTreeMap::new();
        let mut state_to_sponge_id: BTreeMap<String, i32> = BTreeMap::new();
        let mut next_id = 0i32;

        // Air is id 0
        state_to_sponge_id.insert("minecraft:air".to_string(), 0);
        palette_compound.insert("minecraft:air".to_string(), NbtTag::Int(0));
        next_id += 1;

        for (_pal_idx, state) in region.palette().iter() {
            let state_str = state.to_string();
            if !state_to_sponge_id.contains_key(&state_str) {
                state_to_sponge_id.insert(state_str.clone(), next_id);
                palette_compound.insert(state_str, NbtTag::Int(next_id));
                next_id += 1;
            }
        }
        schem.insert("Palette".to_string(), NbtTag::Compound(palette_compound));

        // Encode BlockData
        let total_blocks = (width as usize) * (height as usize) * (length as usize);
        let mut block_data_bytes = Vec::with_capacity(total_blocks);

        for y in 0..(height as usize) {
            for z in 0..(length as usize) {
                for x in 0..(width as usize) {
                    let local_pos = BlockPosition::new(x as i64, y as i64, z as i64);
                    let state = region
                        .block_index_at(local_pos)
                        .ok()
                        .flatten()
                        .and_then(|idx| region.palette().get(idx));

                    let sponge_id = match state {
                        Some(st) => {
                            let s = st.to_string();
                            *state_to_sponge_id.get(&s).unwrap_or(&0)
                        }
                        None => 0,
                    };
                    write_varint(sponge_id, &mut block_data_bytes);
                }
            }
        }
        schem.insert("BlockData".to_string(), NbtTag::ByteArray(block_data_bytes));

        // BlockEntities
        let mut be_list = Vec::new();
        for be in doc.block_entities() {
            let mut be_compound = BTreeMap::new();
            be_compound.insert("Id".to_string(), NbtTag::String(be.kind.clone()));
            be_compound.insert(
                "Pos".to_string(),
                NbtTag::IntArray(vec![be.position.x, be.position.y, be.position.z]),
            );
            be_list.push(NbtTag::Compound(be_compound));
        }
        schem.insert("BlockEntities".to_string(), NbtTag::List(10, be_list));

        // Entities
        let mut ent_list = Vec::new();
        for ent in doc.entities() {
            let mut ent_compound = BTreeMap::new();
            ent_compound.insert("Id".to_string(), NbtTag::String(ent.kind.clone()));
            ent_compound.insert(
                "Pos".to_string(),
                NbtTag::List(
                    6, // Double
                    vec![
                        NbtTag::Double(ent.position[0]),
                        NbtTag::Double(ent.position[1]),
                        NbtTag::Double(ent.position[2]),
                    ],
                ),
            );
            ent_list.push(NbtTag::Compound(ent_compound));
        }
        schem.insert("Entities".to_string(), NbtTag::List(10, ent_list));

        // Reattach unknown root tags
        for (k, v) in &self.raw_root_unknown {
            if !schem.contains_key(k) {
                schem.insert(k.clone(), v.clone());
            }
        }

        // Root compound wrapped in "Schematic"
        let mut root = BTreeMap::new();
        root.insert("Schematic".to_string(), NbtTag::Compound(schem));

        encode_gzip("Schematic", &NbtTag::Compound(root)).map_err(FormatError::Nbt)
    }
}
