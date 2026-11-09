use crate::command::{visit_occupied_selected_blocks, EditError};
use crate::transform::{
    mirror_block_state, mirror_entity_data, mirror_relative_coords, mirror_relative_point,
    rotate_block_state, rotate_entity_data, rotate_relative_coords, rotate_relative_point,
    MirrorAxis, RotationAngle,
};
use schematic_core::{BlockState, Document, NbtCompound, Position, Region, RegionId, Selection};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClipboardBlock {
    pub offset: [i32; 3],
    pub state: BlockState,
}

/// A block entity captured relative to the clipboard's minimum corner.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipboardBlockEntity {
    pub offset: [i32; 3],
    pub kind: String,
    pub data: NbtCompound,
}

/// An entity captured relative to the clipboard's minimum corner.
///
/// `source_block` is the world block cell the entity occupied when captured;
/// it is used to shift absolute hanging-entity anchors (`TileX/Y/Z`) by the
/// same displacement as the entity itself.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipboardEntity {
    pub offset: [f64; 3],
    pub kind: String,
    pub data: NbtCompound,
    pub source_block: [i32; 3],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Clipboard {
    pub size: [u32; 3],
    pub blocks: Vec<ClipboardBlock>,
    pub block_entities: Vec<ClipboardBlockEntity>,
    pub entities: Vec<ClipboardEntity>,
}

impl Clipboard {
    pub fn new(size: [u32; 3], blocks: Vec<ClipboardBlock>) -> Self {
        Self {
            size,
            blocks,
            block_entities: Vec::new(),
            entities: Vec::new(),
        }
    }

    pub fn empty() -> Self {
        Self::new([0, 0, 0], Vec::new())
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty() && self.entities.is_empty()
    }

    /// Captures blocks, block entities and entities inside `selection` from
    /// one region of `document`.
    pub fn from_document_selection(
        document: &Document,
        region_id: &RegionId,
        selection: &Selection,
    ) -> Result<Self, EditError> {
        let region = document
            .region(region_id)
            .ok_or_else(|| EditError::MissingRegion(region_id.clone()))?;
        let mut clipboard = Self::from_region_selection(region, selection)?;
        if selection.is_empty() {
            return Ok(clipboard);
        }
        let min = selection.bounds().min();
        for be in document.block_entities_in(region_id) {
            let Some(world) = region.local_to_world(be.position) else {
                continue;
            };
            if selection.contains(world)
                && region.block_index_at(be.position).ok().flatten().is_some()
            {
                clipboard.block_entities.push(ClipboardBlockEntity {
                    offset: [world.x - min.x, world.y - min.y, world.z - min.z],
                    kind: be.kind.clone(),
                    data: be.data.clone(),
                });
            }
        }
        for entity in selected_entities(document, region, selection) {
            let origin = region.origin();
            let world = [
                origin.x as f64 + entity.position[0],
                origin.y as f64 + entity.position[1],
                origin.z as f64 + entity.position[2],
            ];
            clipboard.entities.push(ClipboardEntity {
                offset: [
                    world[0] - min.x as f64,
                    world[1] - min.y as f64,
                    world[2] - min.z as f64,
                ],
                kind: entity.kind.clone(),
                data: entity.data.clone(),
                source_block: [
                    world[0].floor() as i32,
                    world[1].floor() as i32,
                    world[2].floor() as i32,
                ],
            });
        }
        Ok(clipboard)
    }

    pub fn from_region_selection(
        region: &Region,
        selection: &Selection,
    ) -> Result<Self, EditError> {
        if selection.is_empty() {
            return Ok(Self::empty());
        }
        let bounds = selection.bounds();
        let min = bounds.min();
        let size = bounds.size().dimensions();
        let mut blocks = Vec::new();

        visit_occupied_selected_blocks(region, selection, |local_pos, palette_idx| {
            if let Some(world_pos) = region.local_to_world(local_pos) {
                if let Some(state) = region.palette().get(palette_idx) {
                    let offset = [
                        world_pos.x - min.x,
                        world_pos.y - min.y,
                        world_pos.z - min.z,
                    ];
                    blocks.push(ClipboardBlock {
                        offset,
                        state: state.clone(),
                    });
                }
            }
            Ok(())
        })?;

        Ok(Self::new(size, blocks))
    }

    pub fn rotated(&self, angle: RotationAngle) -> Self {
        if self.is_empty() {
            return self.clone();
        }
        let size_x = self.size[0] as i64;
        let size_z = self.size[2] as i64;
        let (new_size_x, new_size_z) = match angle {
            RotationAngle::Deg90 | RotationAngle::Deg270 => (self.size[2], self.size[0]),
            RotationAngle::Deg180 => (self.size[0], self.size[2]),
        };
        let new_size = [new_size_x, self.size[1], new_size_z];

        let blocks = self
            .blocks
            .iter()
            .map(|b| {
                let (rx, rz) = rotate_relative_coords(
                    b.offset[0] as i64,
                    b.offset[2] as i64,
                    size_x,
                    size_z,
                    angle,
                );
                ClipboardBlock {
                    offset: [rx as i32, b.offset[1], rz as i32],
                    state: rotate_block_state(&b.state, angle),
                }
            })
            .collect();

        let block_entities = self
            .block_entities
            .iter()
            .map(|be| {
                let (rx, rz) = rotate_relative_coords(
                    be.offset[0] as i64,
                    be.offset[2] as i64,
                    size_x,
                    size_z,
                    angle,
                );
                ClipboardBlockEntity {
                    offset: [rx as i32, be.offset[1], rz as i32],
                    ..be.clone()
                }
            })
            .collect();

        let entities = self
            .entities
            .iter()
            .map(|e| {
                let (rx, rz) = rotate_relative_point(
                    e.offset[0],
                    e.offset[2],
                    size_x as f64,
                    size_z as f64,
                    angle,
                );
                let mut data = e.data.clone();
                rotate_entity_data(&e.kind, &mut data, angle);
                ClipboardEntity {
                    offset: [rx, e.offset[1], rz],
                    data,
                    ..e.clone()
                }
            })
            .collect();

        Self {
            size: new_size,
            blocks,
            block_entities,
            entities,
        }
    }

    pub fn mirrored(&self, axis: MirrorAxis) -> Self {
        if self.is_empty() {
            return self.clone();
        }
        let size_x = self.size[0] as i64;
        let size_z = self.size[2] as i64;

        let blocks = self
            .blocks
            .iter()
            .map(|b| {
                let (rx, rz) = mirror_relative_coords(
                    b.offset[0] as i64,
                    b.offset[2] as i64,
                    size_x,
                    size_z,
                    axis,
                );
                ClipboardBlock {
                    offset: [rx as i32, b.offset[1], rz as i32],
                    state: mirror_block_state(&b.state, axis),
                }
            })
            .collect();

        let block_entities = self
            .block_entities
            .iter()
            .map(|be| {
                let (rx, rz) = mirror_relative_coords(
                    be.offset[0] as i64,
                    be.offset[2] as i64,
                    size_x,
                    size_z,
                    axis,
                );
                ClipboardBlockEntity {
                    offset: [rx as i32, be.offset[1], rz as i32],
                    ..be.clone()
                }
            })
            .collect();

        let entities = self
            .entities
            .iter()
            .map(|e| {
                let (rx, rz) = mirror_relative_point(
                    e.offset[0],
                    e.offset[2],
                    size_x as f64,
                    size_z as f64,
                    axis,
                );
                let mut data = e.data.clone();
                mirror_entity_data(&e.kind, &mut data, axis);
                ClipboardEntity {
                    offset: [rx, e.offset[1], rz],
                    data,
                    ..e.clone()
                }
            })
            .collect();

        Self {
            size: self.size,
            blocks,
            block_entities,
            entities,
        }
    }
}

/// Entities of `region` whose containing block cell lies inside `selection`.
pub(crate) fn selected_entities<'a>(
    document: &'a Document,
    region: &'a Region,
    selection: &'a Selection,
) -> impl Iterator<Item = &'a schematic_core::EntityRef> + 'a {
    let origin = region.origin();
    document.entities_in(region.id()).filter(move |entity| {
        let cell = entity.block_position();
        let world = Position::new(
            origin.x.saturating_add(cell.x as i32),
            origin.y.saturating_add(cell.y as i32),
            origin.z.saturating_add(cell.z as i32),
        );
        selection.contains(world)
    })
}
