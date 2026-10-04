use crate::{EditCommand, EditError, PatchError, PatchSet};
use schematic_core::{BlockPosition, Document, PaletteIndex, RegionId};

#[derive(Clone, Debug, PartialEq)]
pub struct EditWorkspace {
    source: Document,
    committed: Document,
    pending_preview: Option<PatchSet>,
    undo_stack: Vec<PatchSet>,
    redo_stack: Vec<PatchSet>,
}

impl EditWorkspace {
    pub fn start(source: Document) -> Self {
        Self {
            committed: source.clone(),
            source,
            pending_preview: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn source(&self) -> &Document {
        &self.source
    }

    pub fn committed(&self) -> &Document {
        &self.committed
    }

    pub fn pending_preview(&self) -> Option<&PatchSet> {
        self.pending_preview.as_ref()
    }

    pub fn has_preview(&self) -> bool {
        self.pending_preview.is_some()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn is_dirty(&self) -> bool {
        if self.undo_stack.is_empty() {
            false
        } else {
            !self.committed.is_same_content(&self.source)
        }
    }

    pub fn preview_patch(&mut self, patch: PatchSet) {
        self.pending_preview = Some(patch);
    }

    pub fn preview_command(&mut self, command: &impl EditCommand) -> Result<(), EditError> {
        let patch = command.create_patch(&self.committed)?;
        self.preview_patch(patch);
        Ok(())
    }

    pub fn cancel_preview(&mut self) {
        self.pending_preview = None;
    }

    pub fn commit_preview(&mut self) -> Result<(), EditError> {
        let Some(patch) = self.pending_preview.take() else {
            return Err(EditError::Patch(PatchError::InvalidChunkGrouping));
        };
        if patch.is_empty() {
            return Ok(());
        }
        patch.apply(&mut self.committed)?;
        self.undo_stack.push(patch);
        self.redo_stack.clear();
        Ok(())
    }

    pub fn undo(&mut self) -> Result<bool, EditError> {
        if self.has_preview() {
            return Err(EditError::InvalidCoordinate);
        }
        let Some(patch) = self.undo_stack.last() else {
            return Ok(false);
        };
        patch.revert(&mut self.committed)?;
        let patch = self.undo_stack.pop().expect("undo stack had an entry");
        self.redo_stack.push(patch);
        Ok(true)
    }

    pub fn redo(&mut self) -> Result<bool, EditError> {
        if self.has_preview() {
            return Err(EditError::InvalidCoordinate);
        }
        let Some(patch) = self.redo_stack.last() else {
            return Ok(false);
        };
        patch.apply(&mut self.committed)?;
        let patch = self.redo_stack.pop().expect("redo stack had an entry");
        self.undo_stack.push(patch);
        Ok(true)
    }

    pub fn preview_block_at(
        &self,
        region_id: &RegionId,
        position: BlockPosition,
    ) -> Result<Option<PaletteIndex>, EditError> {
        if let Some(preview) = &self.pending_preview {
            if let Some(change) = preview.change_at(region_id, position) {
                return Ok(change.after());
            }
        }
        let region = self
            .committed
            .region(region_id)
            .ok_or_else(|| EditError::MissingRegion(region_id.clone()))?;
        region
            .block_index_at(position)
            .map_err(EditError::RegionBlock)
    }
}
