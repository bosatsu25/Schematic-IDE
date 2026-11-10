use crate::{
    convert::merge_regions,
    litematic::{double_list, region_volume, take_double_list, take_string, FormatError},
    nbt::{decode_gzip_or_raw, encode_gzip, NbtTag},
};
use schematic_core::{
    BlockEntityRef, BlockPosition, BlockProperty, BlockState, Document, DocumentMetadata,
    EntityRef, PaletteIndex, Position, Region, RegionId, Size,
};
use std::collections::BTreeMap;

/// Largest volume exported as Structure NBT. The format stores a compound
/// per cell (vanilla structure blocks top out at 48³), so larger volumes
/// would need gigabytes of memory.
pub const STRUCTURE_MAX_EXPORT_VOLUME: u64 = 1 << 24;

#[derive(Clone, Debug, PartialEq)]
pub struct StructureDocument {
    pub data_version: i32,
    pub document: Document,
    pub raw_root_unknown: BTreeMap<String, NbtTag>,
}

impl StructureDocument {
    pub fn from_document(document: Document, data_version: i32) -> Self {
        Self {
            data_version,
            document,
            raw_root_unknown: BTreeMap::new(),
        }
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, FormatError> {
        let (_root_name, root_tag) = decode_gzip_or_raw(bytes)?;
        let root = root_tag.as_compound().ok_or_else(|| {
            FormatError::InvalidStructure("Root tag is not a Compound".to_string())
        })?;

        let data_version = root
            .get("DataVersion")
            .and_then(|t| t.as_i32())
            .unwrap_or(2975);

        // size: List of 3 Ints [x, y, z]
        let size_list = root
            .get("size")
            .and_then(|t| t.as_list())
            .ok_or_else(|| FormatError::MissingField("size".to_string()))?;

        if size_list.len() < 3 {
            return Err(FormatError::InvalidStructure(
                "size list must contain 3 ints".to_string(),
            ));
        }

        let width = size_list[0].as_i32().unwrap_or(0).max(0) as u32;
        let height = size_list[1].as_i32().unwrap_or(0).max(0) as u32;
        let length = size_list[2].as_i32().unwrap_or(0).max(0) as u32;

        let mut doc = Document::new(DocumentMetadata {
            name: Some("StructureNbt".to_string()),
            ..Default::default()
        });

        let region_id = RegionId::new("Main");
        let origin = Position::new(0, 0, 0);
        let size = Size::new(width, height, length);
        if region_volume(width, height, length).is_none() {
            return Err(FormatError::InvalidStructure(format!(
                "Structure size {width}x{height}x{length} is out of range"
            )));
        }
        let mut region = Region::new(region_id.clone(), origin, size);

        // Parse palette: either "palette" (List of Compound) or "palettes" (List of List of Compound)
        let palette_list = if let Some(pal) = root.get("palette").and_then(|t| t.as_list()) {
            pal
        } else if let Some(pals) = root.get("palettes").and_then(|t| t.as_list()) {
            pals.first()
                .and_then(|t| t.as_list())
                .ok_or_else(|| FormatError::MissingField("palette".to_string()))?
        } else {
            return Err(FormatError::MissingField("palette".to_string()));
        };

        let mut palette_states: Vec<BlockState> = Vec::new();
        let mut region_pal_indices: Vec<PaletteIndex> = Vec::new();

        for entry in palette_list {
            let entry_compound = entry.as_compound().ok_or_else(|| {
                FormatError::InvalidStructure("Palette entry is not a Compound".to_string())
            })?;

            let name = entry_compound
                .get("Name")
                .and_then(|t| t.as_string())
                .ok_or_else(|| FormatError::MissingField("Palette entry Name".to_string()))?;

            let mut properties = Vec::new();
            if let Some(props_compound) = entry_compound
                .get("Properties")
                .and_then(|t| t.as_compound())
            {
                for (k, v) in props_compound {
                    if let Some(val_str) = v.as_string() {
                        properties.push(BlockProperty::new(k.clone(), val_str));
                    }
                }
            }

            let state = BlockState::new(name, properties).map_err(|e| {
                FormatError::InvalidStructure(format!("Invalid block state '{name}': {e}"))
            })?;

            let idx = region
                .palette_mut()
                .intern(state.clone())
                .map_err(|e| FormatError::InvalidStructure(format!("Palette error: {e}")))?;

            palette_states.push(state);
            region_pal_indices.push(idx);
        }

        // Parse blocks: List of Compound
        if let Some(blocks_list) = root.get("blocks").and_then(|t| t.as_list()) {
            for block_tag in blocks_list {
                let block_compound = block_tag.as_compound().ok_or_else(|| {
                    FormatError::InvalidStructure("Block entry is not a Compound".to_string())
                })?;

                let pos_list = block_compound
                    .get("pos")
                    .and_then(|t| t.as_list())
                    .ok_or_else(|| FormatError::MissingField("Block pos".to_string()))?;

                if pos_list.len() < 3 {
                    continue;
                }
                let bx = pos_list[0].as_i32().unwrap_or(0) as i64;
                let by = pos_list[1].as_i32().unwrap_or(0) as i64;
                let bz = pos_list[2].as_i32().unwrap_or(0) as i64;

                let state_idx = block_compound
                    .get("state")
                    .and_then(|t| t.as_i32())
                    .ok_or_else(|| FormatError::MissingField("Block state".to_string()))?
                    as usize;

                if state_idx < palette_states.len() {
                    let state = &palette_states[state_idx];
                    if state.id() != "minecraft:air"
                        && state.id() != "air"
                        && state.id() != "minecraft:cave_air"
                        && state.id() != "minecraft:void_air"
                    {
                        let pal_idx = region_pal_indices[state_idx];
                        let _ =
                            region.set_block_index(BlockPosition::new(bx, by, bz), Some(pal_idx));
                    }
                }

                // Block entity NBT if present
                if let Some(nbt_tag) = block_compound.get("nbt").and_then(|t| t.as_compound()) {
                    let mut data = nbt_tag.clone();
                    let kind = take_string(&mut data, &["id", "Id"])
                        .unwrap_or_else(|| "unknown".to_string());
                    data.remove("x");
                    data.remove("y");
                    data.remove("z");
                    doc.add_block_entity(
                        BlockEntityRef::new(
                            region_id.clone(),
                            kind,
                            BlockPosition::new(bx, by, bz),
                        )
                        .with_data(data),
                    );
                }
            }
        }

        // Parse entities: List of Compound {pos, blockPos, nbt}
        if let Some(entities_list) = root.get("entities").and_then(|t| t.as_list()) {
            for ent_tag in entities_list {
                let Some(ent_compound) = ent_tag.as_compound() else {
                    continue;
                };
                let mut outer = ent_compound.clone();
                let pos = take_double_list(&mut outer, "pos").unwrap_or([0.0; 3]);
                let mut data = outer
                    .get("nbt")
                    .and_then(|t| t.as_compound())
                    .cloned()
                    .unwrap_or_default();
                let kind =
                    take_string(&mut data, &["id", "Id"]).unwrap_or_else(|| "unknown".to_string());
                data.remove("Pos");
                doc.add_entity(EntityRef::new(region_id.clone(), kind, pos).with_data(data));
            }
        }

        doc.insert_region(region);

        let mut raw_root_unknown = BTreeMap::new();
        for (k, v) in root {
            if !matches!(
                k.as_str(),
                "DataVersion" | "size" | "palette" | "palettes" | "blocks" | "entities"
            ) {
                raw_root_unknown.insert(k.clone(), v.clone());
            }
        }

        Ok(Self {
            data_version,
            document: doc,
            raw_root_unknown,
        })
    }

    /// Exports vanilla Structure NBT. Multi-region documents are flattened
    /// (see [`crate::convert::merge_regions`]).
    pub fn export(&self, doc: &Document) -> Result<Vec<u8>, FormatError> {
        let merged = merge_regions(doc)?;
        let doc = &merged.document;
        let region = doc.regions().next().ok_or_else(|| {
            FormatError::InvalidStructure("Document contains no regions".to_string())
        })?;

        let width = region.size().x;
        let height = region.size().y;
        let length = region.size().z;
        // Structure NBT stores one compound per cell, air included.
        if !region_volume(width, height, length).is_some_and(|v| v <= STRUCTURE_MAX_EXPORT_VOLUME) {
            return Err(FormatError::InvalidStructure(format!(
                "A {width}x{height}x{length} volume is too large for Structure NBT (limit: {STRUCTURE_MAX_EXPORT_VOLUME} blocks)"
            )));
        }

        let mut root = BTreeMap::new();
        root.insert("DataVersion".to_string(), NbtTag::Int(self.data_version));

        // size: List of 3 Ints
        root.insert(
            "size".to_string(),
            NbtTag::List(
                3, // Int
                vec![
                    NbtTag::Int(width as i32),
                    NbtTag::Int(height as i32),
                    NbtTag::Int(length as i32),
                ],
            ),
        );

        // Build palette
        let mut palette_list = Vec::new();
        let mut state_to_struct_idx: BTreeMap<String, i32> = BTreeMap::new();

        // Ensure air is at palette index 0
        let mut air_compound = BTreeMap::new();
        air_compound.insert(
            "Name".to_string(),
            NbtTag::String("minecraft:air".to_string()),
        );
        palette_list.push(NbtTag::Compound(air_compound));
        state_to_struct_idx.insert("minecraft:air".to_string(), 0);

        for (_pal_idx, state) in region.palette().iter() {
            let key = state.to_string();
            state_to_struct_idx.entry(key).or_insert_with(|| {
                let mut state_compound = BTreeMap::new();
                state_compound.insert("Name".to_string(), NbtTag::String(state.id().to_string()));

                let mut props_compound = BTreeMap::new();
                for prop in state.properties() {
                    props_compound.insert(
                        prop.name().to_string(),
                        NbtTag::String(prop.value().to_string()),
                    );
                }
                if !props_compound.is_empty() {
                    state_compound
                        .insert("Properties".to_string(), NbtTag::Compound(props_compound));
                }

                let idx = palette_list.len() as i32;
                palette_list.push(NbtTag::Compound(state_compound));
                idx
            });
        }
        root.insert("palette".to_string(), NbtTag::List(10, palette_list));

        // Build blocks list
        let mut blocks_list = Vec::new();

        for y in 0..(height as usize) {
            for z in 0..(length as usize) {
                for x in 0..(width as usize) {
                    let local_pos = BlockPosition::new(x as i64, y as i64, z as i64);
                    let state_opt = region
                        .block_index_at(local_pos)
                        .ok()
                        .flatten()
                        .and_then(|idx| region.palette().get(idx));

                    let state_idx = match state_opt {
                        Some(st) => {
                            let key = st.to_string();
                            *state_to_struct_idx.get(&key).unwrap_or(&0)
                        }
                        None => 0,
                    };

                    let mut block_compound = BTreeMap::new();
                    block_compound.insert(
                        "pos".to_string(),
                        NbtTag::List(
                            3,
                            vec![
                                NbtTag::Int(x as i32),
                                NbtTag::Int(y as i32),
                                NbtTag::Int(z as i32),
                            ],
                        ),
                    );
                    block_compound.insert("state".to_string(), NbtTag::Int(state_idx));

                    if let Some(be) = doc.block_entity_at(region.id(), local_pos) {
                        let mut nbt_compound = be.data.clone();
                        nbt_compound.insert("id".to_string(), NbtTag::String(be.kind.clone()));
                        block_compound.insert("nbt".to_string(), NbtTag::Compound(nbt_compound));
                    }

                    blocks_list.push(NbtTag::Compound(block_compound));
                }
            }
        }
        root.insert("blocks".to_string(), NbtTag::List(10, blocks_list));

        // Build entities list
        let mut entities_list = Vec::new();
        for ent in doc.entities_in(region.id()) {
            let mut ent_compound = BTreeMap::new();
            ent_compound.insert("pos".to_string(), double_list(ent.position));
            ent_compound.insert(
                "blockPos".to_string(),
                NbtTag::List(
                    3,
                    ent.position
                        .iter()
                        .map(|v| NbtTag::Int(v.floor() as i32))
                        .collect(),
                ),
            );
            let mut nbt = ent.data.clone();
            nbt.insert("id".to_string(), NbtTag::String(ent.kind.clone()));
            nbt.insert("Pos".to_string(), double_list(ent.position));
            ent_compound.insert("nbt".to_string(), NbtTag::Compound(nbt));
            entities_list.push(NbtTag::Compound(ent_compound));
        }
        root.insert("entities".to_string(), NbtTag::List(10, entities_list));

        // Reattach unknown root tags
        for (k, v) in &self.raw_root_unknown {
            if !root.contains_key(k) {
                root.insert(k.clone(), v.clone());
            }
        }

        encode_gzip("", &NbtTag::Compound(root)).map_err(FormatError::Nbt)
    }
}
