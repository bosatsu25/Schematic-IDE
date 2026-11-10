//! Face-culled, chunked mesh extraction for the renderer.
//!
//! Instead of one cube per non-air block, only faces that border air, a
//! non-occluding block, or the region boundary are emitted, grouped by
//! 16³ storage chunk so the renderer can rebuild just the chunks an edit
//! touched. The output is a compact little-endian `i32` stream that the
//! worker transfers to the main thread without JSON.
//!
//! Layout (all `u32`/`i32` little-endian): `[chunk_count]`, then per chunk
//! `[cx, cy, cz, face_count]` followed by `face_count` packed faces, one
//! `u32` each: bits 0–3 x, 4–7 y, 8–11 z (block offset inside the chunk),
//! 12–14 direction, 15–31 palette index. Region-local block position is
//! `chunk * 16 + offset`. Directions: 0 +X, 1 −X, 2 +Y, 3 −Y, 4 +Z, 5 −Z.

use schematic_core::{
    BlockPosition, ChunkPosition, Document, PaletteIndex, Region, RegionId, CHUNK_VOLUME,
};
use schematic_edit::PatchSet;
use std::collections::{BTreeSet, HashMap};

pub const DIRECTIONS: [[i64; 3]; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];

/// Block ids that render but do not hide the faces behind them.
fn occludes(id: &str) -> bool {
    let name = id.trim_start_matches("minecraft:");
    if crate::session::is_air(id) {
        return false;
    }
    const NON_FULL: [&str; 30] = [
        "glass",
        "leaves",
        "_pane",
        "fence",
        "iron_bars",
        "slab",
        "stairs",
        "torch",
        "door",
        "sign",
        "carpet",
        "rail",
        "flower",
        "sapling",
        "button",
        "lever",
        "pressure_plate",
        "water",
        "lava",
        "ice",
        "wall",
        "chain",
        "lantern",
        "ladder",
        "vine",
        "banner",
        "head",
        "skull",
        "candle",
        "bed",
    ];
    !NON_FULL.iter().any(|needle| name.contains(needle))
        && !matches!(
            name,
            "grass" | "short_grass" | "tall_grass" | "fern" | "snow" | "cobweb" | "barrier"
        )
}

/// Largest palette index a packed face can carry (17 bits).
pub const MAX_PACKED_PALETTE: u32 = (1 << 17) - 1;

/// Packs one face as described in the module docs.
pub fn pack_face(pos: BlockPosition, dir: u32, palette_index: u32) -> u32 {
    let local = |v: i64| (v.rem_euclid(16)) as u32;
    local(pos.x)
        | (local(pos.y) << 4)
        | (local(pos.z) << 8)
        | (dir << 12)
        | (palette_index.min(MAX_PACKED_PALETTE) << 15)
}

/// The block state shown for each region of `doc` with `preview` applied.
pub struct RegionView<'a> {
    region: &'a Region,
    overrides: HashMap<BlockPosition, Option<PaletteIndex>>,
    /// Palette names including entries the preview adds.
    pub palette: Vec<String>,
    /// Per palette index: is the block visible / does it hide neighbours.
    visible: Vec<bool>,
    occluding: Vec<bool>,
}

impl<'a> RegionView<'a> {
    pub fn new(
        doc: &'a Document,
        preview: Option<&'a PatchSet>,
        region_id: &RegionId,
    ) -> Option<Self> {
        let region = doc.region(region_id)?;
        let mut states: Vec<Option<&schematic_core::BlockState>> =
            region.palette().iter().map(|(_, s)| Some(s)).collect();
        let mut overrides = HashMap::new();
        if let Some(patch) = preview {
            for (rid, index, state) in patch.palette_additions() {
                if rid == region_id {
                    let i = index.get() as usize;
                    if states.len() <= i {
                        states.resize(i + 1, None);
                    }
                    states[i] = Some(state);
                }
            }
            for p in patch.patches().filter(|p| p.region_id() == region_id) {
                for change in p.changes() {
                    overrides.insert(change.position(), change.after());
                }
            }
        }
        let palette = states
            .iter()
            .map(|s| {
                s.map(crate::session::block_state_to_string)
                    .unwrap_or_default()
            })
            .collect();
        let visible = states
            .iter()
            .map(|s| s.is_some_and(|s| !crate::session::is_air(s.id())))
            .collect();
        let occluding = states
            .iter()
            .map(|s| s.is_some_and(|s| occludes(s.id())))
            .collect();
        Some(Self {
            region,
            overrides,
            palette,
            visible,
            occluding,
        })
    }

    pub fn region(&self) -> &Region {
        self.region
    }

    /// Positions the preview changes (for highlight overlays).
    pub fn preview_positions(&self) -> Vec<[i64; 3]> {
        let mut out: Vec<[i64; 3]> = self.overrides.keys().map(|p| p.coordinates()).collect();
        out.sort_unstable();
        out
    }

    fn index_at(&self, pos: BlockPosition) -> Option<PaletteIndex> {
        if let Some(over) = self.overrides.get(&pos) {
            return *over;
        }
        self.region.block_index_at(pos).ok().flatten()
    }

    fn in_bounds(&self, p: BlockPosition) -> bool {
        let s = self.region.size();
        p.x >= 0 && p.y >= 0 && p.z >= 0 && p.x < s.x as i64 && p.y < s.y as i64 && p.z < s.z as i64
    }

    /// Every chunk holding committed or previewed blocks.
    pub fn all_chunks(&self) -> Vec<ChunkPosition> {
        let mut set: BTreeSet<ChunkPosition> = self.region.chunks().map(|(pos, _)| pos).collect();
        for (pos, after) in &self.overrides {
            if after.is_some() {
                set.insert(ChunkPosition::from_block_position(*pos));
            }
        }
        set.into_iter().collect()
    }

    /// Appends the visible faces of one chunk; returns the face count.
    fn chunk_faces(&self, chunk: ChunkPosition, out: &mut Vec<i32>) -> usize {
        let mut count = 0;
        for idx in 0..CHUNK_VOLUME {
            let Some(pos) = chunk.block_position_for_index(idx) else {
                continue;
            };
            let Some(pal) = self.index_at(pos) else {
                continue;
            };
            let pal_i = pal.get() as usize;
            if !self.visible.get(pal_i).copied().unwrap_or(false) {
                continue;
            }
            for (dir, d) in DIRECTIONS.iter().enumerate() {
                let n = BlockPosition::new(pos.x + d[0], pos.y + d[1], pos.z + d[2]);
                let hidden = self.in_bounds(n)
                    && self.index_at(n).is_some_and(|ni| {
                        self.occluding
                            .get(ni.get() as usize)
                            .copied()
                            .unwrap_or(false)
                    });
                if !hidden {
                    out.push(pack_face(pos, dir as u32, pal_i as u32) as i32);
                    count += 1;
                }
            }
        }
        count
    }

    /// Encodes the given chunks (all chunks when `None`) as described in the
    /// module docs. Requested chunks are always emitted, even when empty, so
    /// the renderer can clear them.
    pub fn encode_faces(&self, chunks: Option<&[ChunkPosition]>) -> Vec<u8> {
        let owned;
        let chunks = match chunks {
            Some(c) => c,
            None => {
                owned = self.all_chunks();
                &owned
            }
        };
        let mut out: Vec<i32> = Vec::with_capacity(1 + chunks.len() * 64);
        out.push(chunks.len() as i32);
        for chunk in chunks {
            out.extend_from_slice(&[chunk.x as i32, chunk.y as i32, chunk.z as i32, 0]);
            let count_slot = out.len() - 1;
            let count = self.chunk_faces(*chunk, &mut out);
            out[count_slot] = count as i32;
        }
        let mut bytes = Vec::with_capacity(out.len() * 4);
        for v in out {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        bytes
    }
}

/// Chunks whose visible faces a patch can change: every touched chunk and
/// its six neighbours (faces on chunk borders depend on the neighbour).
pub fn affected_chunks(patch: &PatchSet) -> Vec<(RegionId, ChunkPosition)> {
    let mut set = BTreeSet::new();
    for p in patch.patches() {
        let c = p.chunk_position();
        set.insert((p.region_id().clone(), c));
        for d in DIRECTIONS {
            set.insert((
                p.region_id().clone(),
                ChunkPosition::new(c.x + d[0], c.y + d[1], c.z + d[2]),
            ));
        }
    }
    set.into_iter().collect()
}
