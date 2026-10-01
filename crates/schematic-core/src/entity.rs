use crate::Position;

#[derive(Clone, Debug, PartialEq)]
pub struct EntityRef {
    pub kind: String,
    pub position: [f64; 3],
}

impl EntityRef {
    pub fn new(kind: impl Into<String>, position: [f64; 3]) -> Self {
        Self {
            kind: kind.into(),
            position,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BlockEntityRef {
    pub kind: String,
    pub position: Position,
}

impl BlockEntityRef {
    pub fn new(kind: impl Into<String>, position: Position) -> Self {
        Self {
            kind: kind.into(),
            position,
        }
    }
}
