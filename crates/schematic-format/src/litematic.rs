use crate::{
    bit_packing::{bits_per_block, pack_block_states, unpack_block_states},
    nbt::{decode_gzip_or_raw, encode_gzip, NbtError, NbtTag},
};
use schematic_core::{
    BlockEntityRef, BlockPosition, BlockProperty, BlockState, Document, DocumentMetadata,
    EntityRef, NbtCompound, PaletteIndex, Position, Region, RegionId, Size,
};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FormatError {
    Nbt(NbtError),
    MissingField(String),
    InvalidStructure(String),
}

impl Display for FormatError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nbt(e) => Display::fmt(e, formatter),
            Self::MissingField(f) => write!(formatter, "Missing required field: {f}"),
            Self::InvalidStructure(s) => write!(formatter, "Invalid Litematic structure: {s}"),
        }
    }
}

impl Error for FormatError {}

impl From<NbtError> for FormatError {
    fn from(err: NbtError) -> Self {
        Self::Nbt(err)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RawRegionData {
    pub original_pos: [i32; 3],
    pub original_size: [i32; 3],
    pub pending_block_ticks: Option<Vec<NbtTag>>,
    pub pending_fluid_ticks: Option<Vec<NbtTag>>,
    pub unknown_tags: BTreeMap<String, NbtTag>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LitematicDocument {
    pub data_version: i32,
    pub version: i32,
    pub document: Document,
    pub raw_metadata: BTreeMap<String, NbtTag>,
    pub raw_root_unknown: BTreeMap<String, NbtTag>,
    pub raw_regions: BTreeMap<RegionId, RawRegionData>,
}

impl LitematicDocument {
    pub fn new(document: Document) -> Self {
        Self::from_document(document, 2975)
    }

    pub fn from_document(document: Document, data_version: i32) -> Self {
        Self {
            data_version,
            version: 6,
            document,
            raw_metadata: BTreeMap::new(),
            raw_root_unknown: BTreeMap::new(),
            raw_regions: BTreeMap::new(),
        }
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, FormatError> {
        let (_root_name, root_tag) = decode_gzip_or_raw(bytes)?;
        let root_compound = root_tag.as_compound().ok_or_else(|| {
            FormatError::InvalidStructure("Root tag is not a Compound".to_string())
        })?;

        let data_version = root_compound
            .get("MinecraftDataVersion")
            .and_then(|t| t.as_i32())
            .unwrap_or(2975);

        let version = root_compound
            .get("Version")
            .and_then(|t| t.as_i32())
            .unwrap_or(5);

        let metadata_tag = root_compound.get("Metadata").and_then(|t| t.as_compound());
        let mut raw_metadata = BTreeMap::new();
        let mut doc_metadata = DocumentMetadata::default();

        if let Some(meta) = metadata_tag {
            raw_metadata = meta.clone();
            if let Some(name) = meta.get("Name").and_then(|t| t.as_string()) {
                doc_metadata.name = Some(name.to_string());
            }
            if let Some(author) = meta.get("Author").and_then(|t| t.as_string()) {
                doc_metadata.author = Some(author.to_string());
            }
            if let Some(desc) = meta.get("Description").and_then(|t| t.as_string()) {
                doc_metadata.description = Some(desc.to_string());
            }
            if let Some(created) = meta.get("TimeCreated").and_then(|t| t.as_i64()) {
                doc_metadata.created_at_unix_ms = Some(created);
            }
            if let Some(modified) = meta.get("TimeModified").and_then(|t| t.as_i64()) {
                doc_metadata.modified_at_unix_ms = Some(modified);
            }
        }

        let mut raw_root_unknown = BTreeMap::new();
        for (key, tag) in root_compound {
            if key != "MinecraftDataVersion"
                && key != "Version"
                && key != "Metadata"
                && key != "Regions"
            {
                raw_root_unknown.insert(key.clone(), tag.clone());
            }
        }

        let mut document = Document::new(doc_metadata);
        let mut raw_regions = BTreeMap::new();

        if let Some(regions_tag) = root_compound.get("Regions").and_then(|t| t.as_compound()) {
            for (region_name, reg_val) in regions_tag {
                let reg_compound = reg_val.as_compound().ok_or_else(|| {
                    FormatError::InvalidStructure(format!("Region '{region_name}' is not Compound"))
                })?;

                let pos_compound = reg_compound
                    .get("Position")
                    .and_then(|t| t.as_compound())
                    .ok_or_else(|| {
                        FormatError::MissingField(format!("Position in region '{region_name}'"))
                    })?;
                let px = pos_compound.get("x").and_then(|t| t.as_i32()).unwrap_or(0);
                let py = pos_compound.get("y").and_then(|t| t.as_i32()).unwrap_or(0);
                let pz = pos_compound.get("z").and_then(|t| t.as_i32()).unwrap_or(0);

                let size_compound = reg_compound
                    .get("Size")
                    .and_then(|t| t.as_compound())
                    .ok_or_else(|| {
                        FormatError::MissingField(format!("Size in region '{region_name}'"))
                    })?;
                let sx = size_compound.get("x").and_then(|t| t.as_i32()).unwrap_or(0);
                let sy = size_compound.get("y").and_then(|t| t.as_i32()).unwrap_or(0);
                let sz = size_compound.get("z").and_then(|t| t.as_i32()).unwrap_or(0);

                let min_x = if sx >= 0 { px } else { px + sx + 1 };
                let min_y = if sy >= 0 { py } else { py + sy + 1 };
                let min_z = if sz >= 0 { pz } else { pz + sz + 1 };
                let len_x = sx.unsigned_abs();
                let len_y = sy.unsigned_abs();
                let len_z = sz.unsigned_abs();

                let origin = Position::new(min_x, min_y, min_z);
                let size = Size::new(len_x, len_y, len_z);
                let region_id = RegionId::new(region_name.clone());
                let mut region = Region::new(region_id.clone(), origin, size);

                // Palette
                let palette_list = reg_compound
                    .get("BlockStatePalette")
                    .and_then(|t| t.as_list())
                    .ok_or_else(|| {
                        FormatError::MissingField(format!(
                            "BlockStatePalette in region '{region_name}'"
                        ))
                    })?;

                let mut palette_states = Vec::with_capacity(palette_list.len());
                for (entry_idx, entry_tag) in palette_list.iter().enumerate() {
                    let entry = entry_tag.as_compound().ok_or_else(|| {
                        FormatError::InvalidStructure(format!(
                            "Palette entry {entry_idx} in '{region_name}' is not Compound"
                        ))
                    })?;
                    let state_name =
                        entry
                            .get("Name")
                            .and_then(|t| t.as_string())
                            .ok_or_else(|| {
                                FormatError::MissingField(format!(
                                    "Name in palette entry {entry_idx} in '{region_name}'"
                                ))
                            })?;

                    let mut props = Vec::new();
                    if let Some(props_comp) = entry.get("Properties").and_then(|t| t.as_compound())
                    {
                        for (pk, pv) in props_comp {
                            if let Some(pval) = pv.as_string() {
                                props.push(BlockProperty::new(pk.clone(), pval));
                            }
                        }
                    }

                    let block_state = BlockState::new(state_name, props).map_err(|e| {
                        FormatError::InvalidStructure(format!(
                            "Invalid BlockState '{state_name}': {e}"
                        ))
                    })?;

                    region
                        .palette_mut()
                        .intern(block_state.clone())
                        .map_err(|e| {
                            FormatError::InvalidStructure(format!("Palette error: {e}"))
                        })?;
                    palette_states.push(block_state);
                }

                // BlockStates LongArray
                if let Some(longs) = reg_compound
                    .get("BlockStates")
                    .and_then(|t| t.as_long_array())
                {
                    let total_blocks = (len_x as usize) * (len_y as usize) * (len_z as usize);
                    let bits = bits_per_block(palette_states.len().max(1));
                    let indices = unpack_block_states(longs, total_blocks, bits);

                    for dy in 0..len_y {
                        for dz in 0..len_z {
                            for dx in 0..len_x {
                                let k = ((dy as usize * len_z as usize) + dz as usize)
                                    * len_x as usize
                                    + dx as usize;
                                if k >= indices.len() {
                                    continue;
                                }
                                let palette_idx = indices[k];
                                if palette_idx >= palette_states.len() {
                                    continue;
                                }
                                let state = &palette_states[palette_idx];
                                // Skip air in sparse chunk representation
                                if state.id() == "minecraft:air"
                                    || state.id() == "air"
                                    || state.id() == "minecraft:cave_air"
                                    || state.id() == "minecraft:void_air"
                                {
                                    continue;
                                }

                                // The container is always indexed from the
                                // region's minimum corner, whatever the sign
                                // of Size (see `EntityFrame`).
                                let local_pos = BlockPosition::new(dx as i64, dy as i64, dz as i64);
                                region
                                    .set_block_index(
                                        local_pos,
                                        Some(PaletteIndex::new(palette_idx as u32)),
                                    )
                                    .map_err(|e| {
                                        FormatError::InvalidStructure(format!(
                                            "Set block error: {e}"
                                        ))
                                    })?;
                            }
                        }
                    }
                }

                // Block Entities: x/y/z are container coordinates (minimum corner).
                let frame = EntityFrame::new([sx, sy, sz]);
                if let Some(be_list) = reg_compound.get("TileEntities").and_then(|t| t.as_list()) {
                    for be_tag in be_list {
                        let Some(be_comp) = be_tag.as_compound() else {
                            continue;
                        };
                        let mut data = be_comp.clone();
                        let id = take_string(&mut data, &["id", "Id"])
                            .unwrap_or_else(|| "unknown".to_string());
                        let bx = take_int(&mut data, "x").unwrap_or(0);
                        let by = take_int(&mut data, "y").unwrap_or(0);
                        let bz = take_int(&mut data, "z").unwrap_or(0);
                        document.add_block_entity(
                            BlockEntityRef::new(
                                region_id.clone(),
                                id,
                                BlockPosition::new(bx as i64, by as i64, bz as i64),
                            )
                            .with_data(data),
                        );
                    }
                }

                // Entities
                if let Some(ent_list) = reg_compound.get("Entities").and_then(|t| t.as_list()) {
                    for ent_tag in ent_list {
                        let Some(ent_comp) = ent_tag.as_compound() else {
                            continue;
                        };
                        let mut data = ent_comp.clone();
                        let id = take_string(&mut data, &["id", "Id"])
                            .unwrap_or_else(|| "unknown".to_string());
                        let pos = take_double_list(&mut data, "Pos").unwrap_or([0.0; 3]);
                        document.add_entity(
                            EntityRef::new(region_id.clone(), id, frame.to_local(pos))
                                .with_data(data),
                        );
                    }
                }

                let pending_block_ticks = reg_compound
                    .get("PendingBlockTicks")
                    .and_then(|t| t.as_list())
                    .map(|l| l.to_vec());
                let pending_fluid_ticks = reg_compound
                    .get("PendingFluidTicks")
                    .and_then(|t| t.as_list())
                    .map(|l| l.to_vec());

                let mut unknown_tags = BTreeMap::new();
                for (k, v) in reg_compound {
                    if k != "Position"
                        && k != "Size"
                        && k != "BlockStatePalette"
                        && k != "BlockStates"
                        && k != "TileEntities"
                        && k != "Entities"
                        && k != "PendingBlockTicks"
                        && k != "PendingFluidTicks"
                    {
                        unknown_tags.insert(k.clone(), v.clone());
                    }
                }

                raw_regions.insert(
                    region_id.clone(),
                    RawRegionData {
                        original_pos: [px, py, pz],
                        original_size: [sx, sy, sz],
                        pending_block_ticks,
                        pending_fluid_ticks,
                        unknown_tags,
                    },
                );

                document.insert_region(region);
            }
        }

        Ok(Self {
            data_version,
            version,
            document,
            raw_metadata,
            raw_root_unknown,
            raw_regions,
        })
    }

    pub fn export(&self, doc: &Document) -> Result<Vec<u8>, FormatError> {
        let mut root_map = self.raw_root_unknown.clone();
        root_map.insert(
            "MinecraftDataVersion".to_string(),
            NbtTag::Int(self.data_version),
        );
        root_map.insert("Version".to_string(), NbtTag::Int(self.version));

        // Build Metadata
        let mut meta_map = self.raw_metadata.clone();
        if let Some(name) = &doc.metadata().name {
            meta_map.insert("Name".to_string(), NbtTag::String(name.clone()));
        }
        if let Some(author) = &doc.metadata().author {
            meta_map.insert("Author".to_string(), NbtTag::String(author.clone()));
        }
        if let Some(desc) = &doc.metadata().description {
            meta_map.insert("Description".to_string(), NbtTag::String(desc.clone()));
        }
        if let Some(created) = doc.metadata().created_at_unix_ms {
            meta_map.insert("TimeCreated".to_string(), NbtTag::Long(created));
        }
        if let Some(modified) = doc.metadata().modified_at_unix_ms {
            meta_map.insert("TimeModified".to_string(), NbtTag::Long(modified));
        }

        let mut total_volume = 0i32;
        let mut total_blocks = 0i32;
        let mut min_bounds: Option<[i32; 3]> = None;
        let mut max_bounds: Option<[i32; 3]> = None;

        for region in doc.regions() {
            let s = region.size();
            total_volume += (s.x as i32) * (s.y as i32) * (s.z as i32);
            for (_, chunk) in region.chunks() {
                total_blocks += chunk.occupied_blocks().count() as i32;
            }
            let o = region.origin();
            let ox = o.x;
            let oy = o.y;
            let oz = o.z;
            let ex = ox + s.x as i32;
            let ey = oy + s.y as i32;
            let ez = oz + s.z as i32;

            match &mut min_bounds {
                None => min_bounds = Some([ox, oy, oz]),
                Some(min) => {
                    min[0] = min[0].min(ox);
                    min[1] = min[1].min(oy);
                    min[2] = min[2].min(oz);
                }
            }
            match &mut max_bounds {
                None => max_bounds = Some([ex, ey, ez]),
                Some(max) => {
                    max[0] = max[0].max(ex);
                    max[1] = max[1].max(ey);
                    max[2] = max[2].max(ez);
                }
            }
        }

        meta_map.insert("TotalVolume".to_string(), NbtTag::Int(total_volume));
        meta_map.insert("TotalBlocks".to_string(), NbtTag::Int(total_blocks));

        let enclosing_size = if let (Some(min), Some(max)) = (min_bounds, max_bounds) {
            let mut enc = BTreeMap::new();
            enc.insert("x".to_string(), NbtTag::Int(max[0] - min[0]));
            enc.insert("y".to_string(), NbtTag::Int(max[1] - min[1]));
            enc.insert("z".to_string(), NbtTag::Int(max[2] - min[2]));
            NbtTag::Compound(enc)
        } else {
            let mut enc = BTreeMap::new();
            enc.insert("x".to_string(), NbtTag::Int(0));
            enc.insert("y".to_string(), NbtTag::Int(0));
            enc.insert("z".to_string(), NbtTag::Int(0));
            NbtTag::Compound(enc)
        };
        meta_map.insert("EnclosingSize".to_string(), enclosing_size);

        root_map.insert("Metadata".to_string(), NbtTag::Compound(meta_map));

        // Build Regions
        let mut regions_map = BTreeMap::new();

        for region in doc.regions() {
            let mut reg_compound = BTreeMap::new();
            let raw_info = self.raw_regions.get(region.id());

            // Position & Size (preserve negative sizes if original region dimensions match)
            let (px, py, pz, sx, sy, sz) = if let Some(raw) = raw_info {
                if raw.original_size[0].unsigned_abs() == region.size().x
                    && raw.original_size[1].unsigned_abs() == region.size().y
                    && raw.original_size[2].unsigned_abs() == region.size().z
                {
                    (
                        raw.original_pos[0],
                        raw.original_pos[1],
                        raw.original_pos[2],
                        raw.original_size[0],
                        raw.original_size[1],
                        raw.original_size[2],
                    )
                } else {
                    (
                        region.origin().x,
                        region.origin().y,
                        region.origin().z,
                        region.size().x as i32,
                        region.size().y as i32,
                        region.size().z as i32,
                    )
                }
            } else {
                (
                    region.origin().x,
                    region.origin().y,
                    region.origin().z,
                    region.size().x as i32,
                    region.size().y as i32,
                    region.size().z as i32,
                )
            };

            let mut pos_map = BTreeMap::new();
            pos_map.insert("x".to_string(), NbtTag::Int(px));
            pos_map.insert("y".to_string(), NbtTag::Int(py));
            pos_map.insert("z".to_string(), NbtTag::Int(pz));
            reg_compound.insert("Position".to_string(), NbtTag::Compound(pos_map));

            let mut size_map = BTreeMap::new();
            size_map.insert("x".to_string(), NbtTag::Int(sx));
            size_map.insert("y".to_string(), NbtTag::Int(sy));
            size_map.insert("z".to_string(), NbtTag::Int(sz));
            reg_compound.insert("Size".to_string(), NbtTag::Compound(size_map));

            // Palette
            // Litematica requires at least 1 entry (air)
            let air_state = BlockState::new("minecraft:air", []).unwrap();
            let mut out_palette: Vec<BlockState> = Vec::new();
            let mut air_idx: Option<usize> = None;

            for (_idx, state) in region.palette().iter() {
                if state.id() == "minecraft:air" || state.id() == "air" {
                    air_idx = Some(out_palette.len());
                }
                out_palette.push(state.clone());
            }

            if air_idx.is_none() {
                air_idx = Some(out_palette.len());
                out_palette.push(air_state);
            }

            let mut palette_tags = Vec::with_capacity(out_palette.len());
            for state in &out_palette {
                let mut state_comp = BTreeMap::new();
                state_comp.insert("Name".to_string(), NbtTag::String(state.id().to_string()));
                let mut prop_comp = BTreeMap::new();
                for prop in state.properties() {
                    prop_comp.insert(
                        prop.name().to_string(),
                        NbtTag::String(prop.value().to_string()),
                    );
                }
                if !prop_comp.is_empty() {
                    state_comp.insert("Properties".to_string(), NbtTag::Compound(prop_comp));
                }
                palette_tags.push(NbtTag::Compound(state_comp));
            }
            reg_compound.insert(
                "BlockStatePalette".to_string(),
                NbtTag::List(10, palette_tags),
            );

            // BlockStates LongArray
            let len_x = sx.unsigned_abs();
            let len_y = sy.unsigned_abs();
            let len_z = sz.unsigned_abs();
            let total_blocks = (len_x as usize) * (len_y as usize) * (len_z as usize);
            let bits = bits_per_block(out_palette.len());

            let mut indices = Vec::with_capacity(total_blocks);
            for dy in 0..len_y {
                for dz in 0..len_z {
                    for dx in 0..len_x {
                        let local_pos = BlockPosition::new(dx as i64, dy as i64, dz as i64);

                        let p_idx = match region.block_index_at(local_pos).ok().flatten() {
                            Some(idx) => {
                                if let Some(state) = region.palette().get(idx) {
                                    out_palette
                                        .iter()
                                        .position(|s| s == state)
                                        .unwrap_or(air_idx.unwrap())
                                } else {
                                    air_idx.unwrap()
                                }
                            }
                            None => air_idx.unwrap(),
                        };
                        indices.push(p_idx);
                    }
                }
            }

            let packed = pack_block_states(&indices, bits);
            reg_compound.insert("BlockStates".to_string(), NbtTag::LongArray(packed));

            // Block Entities & Entities (from the edited document)
            let frame = EntityFrame::new([sx, sy, sz]);
            let mut tile_entities = Vec::new();
            for be in doc.block_entities_in(region.id()) {
                let mut compound = be.data.clone();
                let [x, y, z] = be.position.coordinates().map(|v| v as i32);
                compound.insert("id".to_string(), NbtTag::String(be.kind.clone()));
                compound.insert("x".to_string(), NbtTag::Int(x));
                compound.insert("y".to_string(), NbtTag::Int(y));
                compound.insert("z".to_string(), NbtTag::Int(z));
                tile_entities.push(NbtTag::Compound(compound));
            }
            reg_compound.insert("TileEntities".to_string(), NbtTag::List(10, tile_entities));

            let mut entities = Vec::new();
            for entity in doc.entities_in(region.id()) {
                let mut compound = entity.data.clone();
                compound.insert("id".to_string(), NbtTag::String(entity.kind.clone()));
                compound.insert(
                    "Pos".to_string(),
                    double_list(frame.from_local(entity.position)),
                );
                entities.push(NbtTag::Compound(compound));
            }
            reg_compound.insert("Entities".to_string(), NbtTag::List(10, entities));

            // Preserved Pending Ticks & Unknown Tags
            if let Some(raw) = raw_info {
                if let Some(ticks) = &raw.pending_block_ticks {
                    reg_compound.insert(
                        "PendingBlockTicks".to_string(),
                        NbtTag::List(10, ticks.clone()),
                    );
                }
                if let Some(ticks) = &raw.pending_fluid_ticks {
                    reg_compound.insert(
                        "PendingFluidTicks".to_string(),
                        NbtTag::List(10, ticks.clone()),
                    );
                }
                for (uk, uv) in &raw.unknown_tags {
                    reg_compound.insert(uk.clone(), uv.clone());
                }
            }

            regions_map.insert(
                region.id().as_str().to_string(),
                NbtTag::Compound(reg_compound),
            );
        }

        root_map.insert("Regions".to_string(), NbtTag::Compound(regions_map));

        let encoded = encode_gzip("Litematic", &NbtTag::Compound(root_map))?;
        Ok(encoded)
    }
}

/// Litematica coordinate conventions for a region whose `Size` may be negative.
///
/// - `Position` is the region's primary corner. When a `Size` component is
///   negative the region extends towards negative coordinates, so the minimum
///   corner is `Position + Size + 1` on that axis.
/// - `BlockStates` and `TileEntities` coordinates are always relative to the
///   **minimum corner** (container coordinates); they are never mirrored.
/// - Entity `Pos` is relative to `Position` (the primary corner), so on a
///   negative axis it lies in `(Size, 0]` and is shifted by `|Size| - 1` to
///   reach region-local (minimum-corner) coordinates.
///
/// This matches independent readers of the format; verifying it against a
/// file written by Litematica itself is tracked in issue #37.
struct EntityFrame {
    shift: [f64; 3],
}

impl EntityFrame {
    fn new(size: [i32; 3]) -> Self {
        Self {
            shift: size.map(|s| {
                if s < 0 {
                    (s.unsigned_abs() as f64) - 1.0
                } else {
                    0.0
                }
            }),
        }
    }

    fn to_local(&self, p: [f64; 3]) -> [f64; 3] {
        [0, 1, 2].map(|i| p[i] + self.shift[i])
    }

    fn from_local(&self, p: [f64; 3]) -> [f64; 3] {
        [0, 1, 2].map(|i| p[i] - self.shift[i])
    }
}

pub(crate) fn take_string(data: &mut NbtCompound, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(NbtTag::String(value)) = data.get(*key) {
            let value = value.clone();
            data.remove(*key);
            return Some(value);
        }
    }
    None
}

pub(crate) fn take_int(data: &mut NbtCompound, key: &str) -> Option<i32> {
    let value = data.get(key).and_then(NbtTag::as_i32)?;
    data.remove(key);
    Some(value)
}

pub(crate) fn take_double_list(data: &mut NbtCompound, key: &str) -> Option<[f64; 3]> {
    let list = data.get(key).and_then(NbtTag::as_list)?;
    let mut out = [0.0; 3];
    for (i, v) in list.iter().take(3).enumerate() {
        out[i] = v.as_f64().unwrap_or(0.0);
    }
    data.remove(key);
    Some(out)
}

pub(crate) fn double_list(p: [f64; 3]) -> NbtTag {
    NbtTag::List(6, p.iter().map(|v| NbtTag::Double(*v)).collect())
}
