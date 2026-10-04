use crate::clipboard::Clipboard;
use crate::transform::{MirrorAxis, RotationAngle};
use crate::{BlockChange, PatchSet};
use schematic_core::{
    BlockPosition, BlockState, Document, PaletteError, PaletteIndex, Position, Region, RegionId,
    Selection,
};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{Display, Formatter};

pub trait EditCommand {
    fn create_patch(&self, document: &Document) -> Result<PatchSet, EditError>;
}

#[derive(Clone, Debug)]
pub struct FillCommand {
    region_id: RegionId,
    selection: Selection,
    state: BlockState,
}

impl FillCommand {
    pub fn new(region_id: RegionId, selection: Selection, state: BlockState) -> Self {
        Self {
            region_id,
            selection,
            state,
        }
    }
}

impl EditCommand for FillCommand {
    fn create_patch(&self, document: &Document) -> Result<PatchSet, EditError> {
        let region = get_region(document, &self.region_id)?;
        let mut changes = Vec::new();
        visit_selected_blocks_dense(region, &self.selection, |local_position, old_index| {
            let current_state = old_index.and_then(|index| region.palette().get(index));
            if current_state == Some(&self.state) {
                return Ok(());
            }
            changes.push((local_position, old_index));
            Ok(())
        })?;
        if changes.is_empty() {
            return Ok(PatchSet::empty());
        }

        let (new_index, addition) = planned_palette_index(region, &self.state)?;
        let changes = changes
            .into_iter()
            .map(|(position, before)| BlockChange::new(position, before, Some(new_index)));
        Ok(PatchSet::from_changes(
            self.region_id.clone(),
            changes,
            addition,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct ReplaceCommand {
    region_id: RegionId,
    selection: Selection,
    from: BlockState,
    to: BlockState,
}

impl ReplaceCommand {
    pub fn new(
        region_id: RegionId,
        selection: Selection,
        from: BlockState,
        to: BlockState,
    ) -> Self {
        Self {
            region_id,
            selection,
            from,
            to,
        }
    }
}

impl EditCommand for ReplaceCommand {
    fn create_patch(&self, document: &Document) -> Result<PatchSet, EditError> {
        if self.from == self.to {
            return Ok(PatchSet::empty());
        }
        let region = get_region(document, &self.region_id)?;
        let mut changes = Vec::new();
        visit_occupied_selected_blocks(region, &self.selection, |local_position, old_index| {
            if region.palette().get(old_index) == Some(&self.from) {
                changes.push((local_position, old_index));
            }
            Ok(())
        })?;
        if changes.is_empty() {
            return Ok(PatchSet::empty());
        }

        let (new_index, addition) = planned_palette_index(region, &self.to)?;
        let changes = changes
            .into_iter()
            .map(|(position, before)| BlockChange::new(position, Some(before), Some(new_index)));
        Ok(PatchSet::from_changes(
            self.region_id.clone(),
            changes,
            addition,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct DeleteCommand {
    region_id: RegionId,
    selection: Selection,
}

impl DeleteCommand {
    pub fn new(region_id: RegionId, selection: Selection) -> Self {
        Self {
            region_id,
            selection,
        }
    }
}

impl EditCommand for DeleteCommand {
    fn create_patch(&self, document: &Document) -> Result<PatchSet, EditError> {
        let region = get_region(document, &self.region_id)?;
        let mut changes = Vec::new();
        visit_occupied_selected_blocks(region, &self.selection, |position, before| {
            changes.push(BlockChange::new(position, Some(before), None));
            Ok(())
        })?;
        Ok(PatchSet::from_changes(
            self.region_id.clone(),
            changes,
            None,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct PasteCommand {
    region_id: RegionId,
    target_pos: Position,
    clipboard: Clipboard,
}

impl PasteCommand {
    pub fn new(region_id: RegionId, target_pos: Position, clipboard: Clipboard) -> Self {
        Self {
            region_id,
            target_pos,
            clipboard,
        }
    }
}

impl EditCommand for PasteCommand {
    fn create_patch(&self, document: &Document) -> Result<PatchSet, EditError> {
        let region = get_region(document, &self.region_id)?;
        if self.clipboard.is_empty() {
            return Ok(PatchSet::empty());
        }

        let mut blocks_to_set = Vec::new();
        for block in &self.clipboard.blocks {
            let world_pos = Position::new(
                self.target_pos.x + block.offset[0],
                self.target_pos.y + block.offset[1],
                self.target_pos.z + block.offset[2],
            );
            if let Some(local_pos) = region.world_to_local(world_pos) {
                blocks_to_set.push((local_pos, block.state.clone()));
            }
        }

        apply_block_updates(&self.region_id, region, Vec::new(), blocks_to_set)
    }
}

#[derive(Clone, Debug)]
pub struct MoveCommand {
    region_id: RegionId,
    selection: Selection,
    delta: [i32; 3],
}

impl MoveCommand {
    pub fn new(region_id: RegionId, selection: Selection, delta: [i32; 3]) -> Self {
        Self {
            region_id,
            selection,
            delta,
        }
    }
}

impl EditCommand for MoveCommand {
    fn create_patch(&self, document: &Document) -> Result<PatchSet, EditError> {
        if self.selection.is_empty()
            || (self.delta[0] == 0 && self.delta[1] == 0 && self.delta[2] == 0)
        {
            return Ok(PatchSet::empty());
        }
        let region = get_region(document, &self.region_id)?;
        let clipboard = Clipboard::from_region_selection(region, &self.selection)?;
        if clipboard.is_empty() {
            return Ok(PatchSet::empty());
        }

        let min = self.selection.bounds().min();

        let mut blocks_to_clear = Vec::new();
        visit_occupied_selected_blocks(region, &self.selection, |pos, _| {
            blocks_to_clear.push(pos);
            Ok(())
        })?;

        let mut blocks_to_set = Vec::new();
        for block in &clipboard.blocks {
            let world_pos = Position::new(
                min.x + block.offset[0] + self.delta[0],
                min.y + block.offset[1] + self.delta[1],
                min.z + block.offset[2] + self.delta[2],
            );
            if let Some(local_pos) = region.world_to_local(world_pos) {
                blocks_to_set.push((local_pos, block.state.clone()));
            }
        }

        apply_block_updates(&self.region_id, region, blocks_to_clear, blocks_to_set)
    }
}

#[derive(Clone, Debug)]
pub struct RotateCommand {
    region_id: RegionId,
    selection: Selection,
    angle: RotationAngle,
}

impl RotateCommand {
    pub fn new(region_id: RegionId, selection: Selection, angle: RotationAngle) -> Self {
        Self {
            region_id,
            selection,
            angle,
        }
    }
}

impl EditCommand for RotateCommand {
    fn create_patch(&self, document: &Document) -> Result<PatchSet, EditError> {
        if self.selection.is_empty() {
            return Ok(PatchSet::empty());
        }
        let region = get_region(document, &self.region_id)?;
        let clipboard = Clipboard::from_region_selection(region, &self.selection)?;
        if clipboard.is_empty() {
            return Ok(PatchSet::empty());
        }

        let min = self.selection.bounds().min();

        let rotated = clipboard.rotated(self.angle);

        let mut blocks_to_clear = Vec::new();
        visit_occupied_selected_blocks(region, &self.selection, |pos, _| {
            blocks_to_clear.push(pos);
            Ok(())
        })?;

        let mut blocks_to_set = Vec::new();
        for block in &rotated.blocks {
            let world_pos = Position::new(
                min.x + block.offset[0],
                min.y + block.offset[1],
                min.z + block.offset[2],
            );
            if let Some(local_pos) = region.world_to_local(world_pos) {
                blocks_to_set.push((local_pos, block.state.clone()));
            }
        }

        apply_block_updates(&self.region_id, region, blocks_to_clear, blocks_to_set)
    }
}

#[derive(Clone, Debug)]
pub struct MirrorCommand {
    region_id: RegionId,
    selection: Selection,
    axis: MirrorAxis,
}

impl MirrorCommand {
    pub fn new(region_id: RegionId, selection: Selection, axis: MirrorAxis) -> Self {
        Self {
            region_id,
            selection,
            axis,
        }
    }
}

impl EditCommand for MirrorCommand {
    fn create_patch(&self, document: &Document) -> Result<PatchSet, EditError> {
        if self.selection.is_empty() {
            return Ok(PatchSet::empty());
        }
        let region = get_region(document, &self.region_id)?;
        let clipboard = Clipboard::from_region_selection(region, &self.selection)?;
        if clipboard.is_empty() {
            return Ok(PatchSet::empty());
        }

        let min = self.selection.bounds().min();

        let mirrored = clipboard.mirrored(self.axis);

        let mut blocks_to_clear = Vec::new();
        visit_occupied_selected_blocks(region, &self.selection, |pos, _| {
            blocks_to_clear.push(pos);
            Ok(())
        })?;

        let mut blocks_to_set = Vec::new();
        for block in &mirrored.blocks {
            let world_pos = Position::new(
                min.x + block.offset[0],
                min.y + block.offset[1],
                min.z + block.offset[2],
            );
            if let Some(local_pos) = region.world_to_local(world_pos) {
                blocks_to_set.push((local_pos, block.state.clone()));
            }
        }

        apply_block_updates(&self.region_id, region, blocks_to_clear, blocks_to_set)
    }
}

fn apply_block_updates(
    region_id: &RegionId,
    region: &Region,
    blocks_to_clear: impl IntoIterator<Item = BlockPosition>,
    blocks_to_set: impl IntoIterator<Item = (BlockPosition, BlockState)>,
) -> Result<PatchSet, EditError> {
    let mut targets: HashMap<BlockPosition, Option<BlockState>> = HashMap::new();
    for pos in blocks_to_clear {
        targets.insert(pos, None);
    }
    for (pos, state) in blocks_to_set {
        targets.insert(pos, Some(state));
    }

    let mut next_palette_index = region.palette().len();
    let mut state_to_index: HashMap<BlockState, PaletteIndex> = HashMap::new();
    let mut additions: Vec<(PaletteIndex, BlockState)> = Vec::new();

    let mut changes = Vec::new();

    for (pos, target_state) in targets {
        let current_index = region.block_index_at(pos).map_err(EditError::RegionBlock)?;
        match target_state {
            None => {
                if current_index.is_some() {
                    changes.push(BlockChange::new(pos, current_index, None));
                }
            }
            Some(state) => {
                let target_index = if let Some(idx) = region.palette().index_of(&state) {
                    idx
                } else if let Some(&idx) = state_to_index.get(&state) {
                    idx
                } else {
                    let idx = PaletteIndex::new(
                        u32::try_from(next_palette_index)
                            .map_err(|_| EditError::PaletteCapacity)?,
                    );
                    next_palette_index += 1;
                    state_to_index.insert(state.clone(), idx);
                    additions.push((idx, state.clone()));
                    idx
                };

                if current_index != Some(target_index) {
                    changes.push(BlockChange::new(pos, current_index, Some(target_index)));
                }
            }
        }
    }

    if changes.is_empty() {
        return Ok(PatchSet::empty());
    }

    Ok(PatchSet::from_changes_with_palette_additions(
        region_id.clone(),
        changes,
        additions,
    ))
}

fn get_region<'a>(document: &'a Document, id: &RegionId) -> Result<&'a Region, EditError> {
    document
        .region(id)
        .ok_or_else(|| EditError::MissingRegion(id.clone()))
}

pub(crate) fn visit_occupied_selected_blocks(
    region: &Region,
    selection: &Selection,
    mut visit: impl FnMut(BlockPosition, PaletteIndex) -> Result<(), EditError>,
) -> Result<(), EditError> {
    if selection.is_empty() || region.is_empty() {
        return Ok(());
    }
    for (chunk_pos, chunk) in region.chunks() {
        if chunk.is_empty() {
            continue;
        }
        for (index, palette_index) in chunk.occupied_blocks() {
            let Some(local_pos) = chunk_pos.block_position_for_index(index) else {
                continue;
            };
            let Some(world_pos) = region.local_to_world(local_pos) else {
                continue;
            };
            if selection.contains(world_pos) {
                visit(local_pos, palette_index)?;
            }
        }
    }
    Ok(())
}

fn visit_selected_blocks_dense(
    region: &Region,
    selection: &Selection,
    mut visit: impl FnMut(BlockPosition, Option<PaletteIndex>) -> Result<(), EditError>,
) -> Result<(), EditError> {
    if selection.is_empty() || region.is_empty() {
        return Ok(());
    }
    let mut visited = HashSet::new();
    for box_item in selection.boxes() {
        let Some(bounds) = region.bounds().intersection(box_item.bounds()) else {
            continue;
        };
        let min = bounds.min();
        let max = bounds.max_exclusive();

        for y in min.y as i64..max[1] {
            for z in min.z as i64..max[2] {
                for x in min.x as i64..max[0] {
                    let world = Position::new(
                        i32::try_from(x).map_err(|_| EditError::InvalidCoordinate)?,
                        i32::try_from(y).map_err(|_| EditError::InvalidCoordinate)?,
                        i32::try_from(z).map_err(|_| EditError::InvalidCoordinate)?,
                    );
                    if !visited.insert(world) {
                        continue;
                    }
                    let local = region
                        .world_to_local(world)
                        .ok_or(EditError::InvalidCoordinate)?;
                    let index = region
                        .block_index_at(local)
                        .map_err(EditError::RegionBlock)?;
                    visit(local, index)?;
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn planned_palette_index(
    region: &Region,
    state: &BlockState,
) -> Result<(PaletteIndex, Option<(PaletteIndex, BlockState)>), EditError> {
    if let Some(index) = region.palette().index_of(state) {
        return Ok((index, None));
    }
    let index = PaletteIndex::new(
        u32::try_from(region.palette().len()).map_err(|_| EditError::PaletteCapacity)?,
    );
    Ok((index, Some((index, state.clone()))))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EditError {
    MissingRegion(RegionId),
    InvalidCoordinate,
    PaletteCapacity,
    RegionBlock(schematic_core::RegionBlockError),
    Patch(crate::PatchError),
}

impl Display for EditError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRegion(id) => write!(formatter, "region '{}' does not exist", id.as_str()),
            Self::InvalidCoordinate => {
                formatter.write_str("coordinate is outside supported bounds")
            }
            Self::PaletteCapacity => PaletteError::CapacityExceeded.fmt(formatter),
            Self::RegionBlock(error) => Display::fmt(error, formatter),
            Self::Patch(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for EditError {}

impl From<crate::PatchError> for EditError {
    fn from(error: crate::PatchError) -> Self {
        Self::Patch(error)
    }
}
