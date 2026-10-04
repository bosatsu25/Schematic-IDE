use crate::{EditCommand, EditError, PatchError, PatchSet};
use schematic_core::Document;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct History {
    undo: Vec<PatchSet>,
    redo: Vec<PatchSet>,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn execute(
        &mut self,
        document: &mut Document,
        command: &impl EditCommand,
    ) -> Result<(), EditError> {
        let patch = command.create_patch(document)?;
        if patch.is_empty() {
            return Ok(());
        }
        patch.apply(document)?;
        self.undo.push(patch);
        self.redo.clear();
        Ok(())
    }

    pub fn undo(&mut self, document: &mut Document) -> Result<bool, PatchError> {
        let Some(patch) = self.undo.last() else {
            return Ok(false);
        };
        patch.revert(document)?;
        let patch = self.undo.pop().expect("last patch was present");
        self.redo.push(patch);
        Ok(true)
    }

    pub fn redo(&mut self, document: &mut Document) -> Result<bool, PatchError> {
        let Some(patch) = self.redo.last() else {
            return Ok(false);
        };
        patch.apply(document)?;
        let patch = self.redo.pop().expect("last patch was present");
        self.undo.push(patch);
        Ok(true)
    }

    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }
}
