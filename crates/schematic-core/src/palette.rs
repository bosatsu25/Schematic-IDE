use crate::BlockState;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PaletteIndex(u32);

impl PaletteIndex {
    pub const fn new(index: u32) -> Self {
        Self(index)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Palette {
    states: Vec<BlockState>,
    indices: HashMap<BlockState, PaletteIndex>,
}

impl Palette {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, state: BlockState) -> Result<PaletteIndex, PaletteError> {
        if let Some(index) = self.indices.get(&state) {
            return Ok(*index);
        }
        let index = PaletteIndex(
            u32::try_from(self.states.len()).map_err(|_| PaletteError::CapacityExceeded)?,
        );
        self.states.push(state.clone());
        self.indices.insert(state, index);
        Ok(index)
    }

    pub fn get(&self, index: PaletteIndex) -> Option<&BlockState> {
        self.states.get(index.0 as usize)
    }

    pub fn len(&self) -> usize {
        self.states.len()
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (PaletteIndex, &BlockState)> {
        self.states
            .iter()
            .enumerate()
            .map(|(index, state)| (PaletteIndex(index as u32), state))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaletteError {
    CapacityExceeded,
}

impl Display for PaletteError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("palette cannot assign an index larger than u32::MAX")
    }
}

impl Error for PaletteError {}
