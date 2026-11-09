//! Deterministic dense `.litematic` fixture generator shared by the scale
//! benchmarks (`tests/scale_benchmarks.rs`) and the browser harness fixture
//! writer (`examples/gen_dense_fixture.rs`).
//!
//! The generated structure is intentionally dense so that benchmarks measure
//! the cost of real block data rather than empty chunks:
//!
//! * ~55% of all voxels are occupied (per-voxel hash, so interior faces are
//!   *not* hidden in a way a naive renderer could exploit).
//! * Materials are chosen per 4x2x4 cell (spatially coherent, like real builds)
//!   with ~1/8 per-voxel variation, drawn from a 34-state palette that
//!   includes property-bearing states (logs, stairs, slabs, containers).
//! * ~1 in 512 occupied voxels is a container (chest / furnace / barrel) with
//!   a matching `TileEntities` NBT compound.
#![allow(dead_code)]

use schematic_core::{
    BlockPosition, BlockProperty, BlockState, Document, DocumentMetadata, Position, Region,
    RegionId, Size,
};
use schematic_format::{LitematicDocument, NbtTag, RawRegionData};
use std::collections::BTreeMap;
use std::time::Instant;

/// Name of the single region in every generated fixture.
pub const REGION_NAME: &str = "Main";

/// Target occupancy in per-mille (550 = 55%).
const FILL_PER_MILLE: u64 = 550;

/// One in `BLOCK_ENTITY_EVERY` occupied voxels becomes a container block entity.
const BLOCK_ENTITY_EVERY: u64 = 512;

/// A named benchmark scale.
#[derive(Clone, Copy, Debug)]
pub struct Scale {
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
    pub length: u32,
}

impl Scale {
    pub const fn total_voxels(&self) -> usize {
        self.width as usize * self.height as usize * self.length as usize
    }
}

pub const SMOKE_32: Scale = Scale {
    name: "32^3",
    width: 32,
    height: 32,
    length: 32,
};
pub const SCALE_1M: Scale = Scale {
    name: "1M",
    width: 100,
    height: 100,
    length: 100,
};
pub const SCALE_5M: Scale = Scale {
    name: "5M",
    width: 200,
    height: 125,
    length: 200,
};
pub const SCALE_10M: Scale = Scale {
    name: "10M",
    width: 250,
    height: 160,
    length: 250,
};

/// Look up a scale by its CLI name (`32`, `1m`, `5m`, `10m`).
pub fn scale_by_name(name: &str) -> Option<Scale> {
    match name.to_ascii_lowercase().as_str() {
        "32" | "smoke" | "32^3" => Some(SMOKE_32),
        "1m" => Some(SCALE_1M),
        "5m" => Some(SCALE_5M),
        "10m" => Some(SCALE_10M),
        _ => None,
    }
}

/// A generated fixture plus the facts a benchmark report needs.
pub struct DenseFixture {
    pub scale: Scale,
    pub total_voxels: usize,
    pub occupied_blocks: usize,
    pub palette_states: usize,
    pub block_entities: usize,
    pub litematic: Vec<u8>,
    pub build_ms: f64,
    pub export_ms: f64,
}

impl DenseFixture {
    pub fn fill_percent(&self) -> f64 {
        self.occupied_blocks as f64 * 100.0 / self.total_voxels as f64
    }
}

fn state(id: &str, props: &[(&str, &str)]) -> BlockState {
    BlockState::new(id, props.iter().map(|(k, v)| BlockProperty::new(*k, *v))).unwrap()
}

/// Plain (non-container) block states, in palette order.
fn structural_states() -> Vec<BlockState> {
    let plain = [
        "minecraft:stone",
        "minecraft:granite",
        "minecraft:polished_granite",
        "minecraft:diorite",
        "minecraft:andesite",
        "minecraft:cobblestone",
        "minecraft:mossy_cobblestone",
        "minecraft:stone_bricks",
        "minecraft:cracked_stone_bricks",
        "minecraft:deepslate_bricks",
        "minecraft:oak_planks",
        "minecraft:spruce_planks",
        "minecraft:birch_planks",
        "minecraft:dark_oak_planks",
        "minecraft:glass",
        "minecraft:white_concrete",
        "minecraft:gray_concrete",
        "minecraft:black_concrete",
        "minecraft:bricks",
        "minecraft:sandstone",
        "minecraft:smooth_quartz",
    ];
    let mut states: Vec<BlockState> = plain.iter().map(|id| state(id, &[])).collect();
    for axis in ["x", "y", "z"] {
        states.push(state("minecraft:oak_log", &[("axis", axis)]));
    }
    for facing in ["north", "south", "east", "west"] {
        states.push(state(
            "minecraft:oak_stairs",
            &[
                ("facing", facing),
                ("half", "bottom"),
                ("shape", "straight"),
                ("waterlogged", "false"),
            ],
        ));
    }
    for ty in ["bottom", "top"] {
        states.push(state(
            "minecraft:stone_brick_slab",
            &[("type", ty), ("waterlogged", "false")],
        ));
    }
    states.push(state(
        "minecraft:lantern",
        &[("hanging", "false"), ("waterlogged", "false")],
    ));
    states
}

/// Container states that carry a block entity: (state, block entity id).
fn container_states() -> Vec<(BlockState, &'static str)> {
    vec![
        (
            state(
                "minecraft:chest",
                &[
                    ("facing", "north"),
                    ("type", "single"),
                    ("waterlogged", "false"),
                ],
            ),
            "minecraft:chest",
        ),
        (
            state(
                "minecraft:furnace",
                &[("facing", "north"), ("lit", "false")],
            ),
            "minecraft:furnace",
        ),
        (
            state("minecraft:barrel", &[("facing", "up"), ("open", "false")]),
            "minecraft:barrel",
        ),
    ]
}

fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn hash3(x: u32, y: u32, z: u32, salt: u64) -> u64 {
    splitmix64(((x as u64) | ((y as u64) << 21) | ((z as u64) << 42)) ^ salt)
}

fn block_entity_tag(id: &str, x: u32, y: u32, z: u32) -> NbtTag {
    let mut map = BTreeMap::new();
    map.insert("id".to_string(), NbtTag::String(id.to_string()));
    map.insert("x".to_string(), NbtTag::Int(x as i32));
    map.insert("y".to_string(), NbtTag::Int(y as i32));
    map.insert("z".to_string(), NbtTag::Int(z as i32));
    map.insert("Items".to_string(), NbtTag::List(10, Vec::new()));
    NbtTag::Compound(map)
}

/// Build a dense document for `scale` and export it to `.litematic` bytes.
pub fn generate(scale: Scale) -> DenseFixture {
    let Scale {
        width,
        height,
        length,
        ..
    } = scale;
    let build_start = Instant::now();

    let meta = DocumentMetadata {
        name: Some(format!("DenseBench_{}x{}x{}", width, height, length)),
        author: Some("schematic-ide scale benchmark".to_string()),
        description: Some("Dense deterministic benchmark fixture (~55% fill)".to_string()),
        ..Default::default()
    };
    let mut doc = Document::new(meta);
    let region_id = RegionId::new(REGION_NAME);
    let mut region = Region::new(
        region_id.clone(),
        Position::new(0, 0, 0),
        Size::new(width, height, length),
    );

    let structural: Vec<_> = structural_states()
        .into_iter()
        .map(|s| region.palette_mut().intern(s).unwrap())
        .collect();
    let containers: Vec<_> = container_states()
        .into_iter()
        .map(|(s, id)| (region.palette_mut().intern(s).unwrap(), id))
        .collect();
    let palette_states = region.palette().len();

    let mut occupied = 0usize;
    let mut block_entities = Vec::new();
    for y in 0..height {
        for z in 0..length {
            for x in 0..width {
                let h = hash3(x, y, z, 0x5EED);
                if h % 1000 >= FILL_PER_MILLE {
                    continue;
                }
                let variant = h >> 32;
                let idx = if variant.is_multiple_of(BLOCK_ENTITY_EVERY) {
                    let (idx, id) = containers[(variant / BLOCK_ENTITY_EVERY) as usize % 3];
                    block_entities.push(block_entity_tag(id, x, y, z));
                    idx
                } else if variant % 8 == 1 {
                    structural[(variant >> 8) as usize % structural.len()]
                } else {
                    let cell = hash3(x >> 2, y >> 1, z >> 2, 0xCE11);
                    structural[cell as usize % structural.len()]
                };
                region
                    .set_block_index(BlockPosition::new(x as i64, y as i64, z as i64), Some(idx))
                    .unwrap();
                occupied += 1;
            }
        }
    }
    doc.insert_region(region);
    let build_ms = build_start.elapsed().as_secs_f64() * 1000.0;

    let block_entity_count = block_entities.len();
    let mut litematic = LitematicDocument::from_document(doc, 2975);
    litematic.raw_regions.insert(
        region_id,
        RawRegionData {
            original_pos: [0, 0, 0],
            original_size: [width as i32, height as i32, length as i32],
            block_entities,
            entities: Vec::new(),
            pending_block_ticks: None,
            pending_fluid_ticks: None,
            unknown_tags: BTreeMap::new(),
        },
    );

    let export_start = Instant::now();
    let bytes = litematic.export(&litematic.document).unwrap();
    let export_ms = export_start.elapsed().as_secs_f64() * 1000.0;

    DenseFixture {
        scale,
        total_voxels: scale.total_voxels(),
        occupied_blocks: occupied,
        palette_states,
        block_entities: block_entity_count,
        litematic: bytes,
        build_ms,
        export_ms,
    }
}
