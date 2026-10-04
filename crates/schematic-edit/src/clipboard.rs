use crate::command::{visit_occupied_selected_blocks, EditError};
use crate::transform::{
    mirror_block_state, mirror_relative_coords, rotate_block_state, rotate_relative_coords,
    MirrorAxis, RotationAngle,
};
use schematic_core::{BlockState, Region, Selection};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClipboardBlock {
    pub offset: [i32; 3],
    pub state: BlockState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Clipboard {
    pub size: [u32; 3],
    pub blocks: Vec<ClipboardBlock>,
}

impl Clipboard {
    pub fn new(size: [u32; 3], blocks: Vec<ClipboardBlock>) -> Self {
        Self { size, blocks }
    }

    pub fn empty() -> Self {
        Self {
            size: [0, 0, 0],
            blocks: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
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

        Ok(Self { size, blocks })
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

        Self {
            size: new_size,
            blocks,
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

        Self {
            size: self.size,
            blocks,
        }
    }
}
