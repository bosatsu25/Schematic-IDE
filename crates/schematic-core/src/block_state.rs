use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockProperty {
    name: String,
    value: String,
}

impl BlockProperty {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BlockState {
    id: String,
    properties: BTreeMap<String, BlockProperty>,
}

impl BlockState {
    pub fn new(
        id: impl Into<String>,
        properties: impl IntoIterator<Item = BlockProperty>,
    ) -> Result<Self, BlockStateError> {
        let id = id.into();
        if id.is_empty() {
            return Err(BlockStateError::EmptyId);
        }

        let mut canonical = BTreeMap::new();
        for property in properties {
            if property.name.is_empty() {
                return Err(BlockStateError::EmptyPropertyName);
            }
            let name = property.name.clone();
            if canonical.insert(name.clone(), property).is_some() {
                return Err(BlockStateError::DuplicateProperty(name));
            }
        }

        Ok(Self {
            id,
            properties: canonical,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn properties(&self) -> impl Iterator<Item = &BlockProperty> {
        self.properties.values()
    }

    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties.get(name).map(BlockProperty::value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BlockStateError {
    EmptyId,
    EmptyPropertyName,
    DuplicateProperty(String),
}

impl Display for BlockState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.properties.is_empty() {
            write!(f, "{}", self.id)
        } else {
            let mut props: Vec<String> = self
                .properties
                .values()
                .map(|p| format!("{}={}", p.name(), p.value()))
                .collect();
            props.sort();
            write!(f, "{}[{}]", self.id, props.join(","))
        }
    }
}

impl std::str::FromStr for BlockState {
    type Err = BlockStateError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if let Some(open) = s.find('[') {
            if !s.ends_with(']') {
                return Err(BlockStateError::EmptyPropertyName);
            }
            let id = &s[..open];
            let inside = &s[open + 1..s.len() - 1];
            let mut props = Vec::new();
            for pair in inside.split(',') {
                let pair = pair.trim();
                if pair.is_empty() {
                    continue;
                }
                let mut parts = pair.splitn(2, '=');
                let name = parts.next().unwrap_or("").trim();
                let val = parts.next().unwrap_or("").trim();
                if name.is_empty() {
                    return Err(BlockStateError::EmptyPropertyName);
                }
                props.push(BlockProperty::new(name, val));
            }
            BlockState::new(id, props)
        } else {
            BlockState::new(s, [])
        }
    }
}

impl Display for BlockStateError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyId => formatter.write_str("block state identifier cannot be empty"),
            Self::EmptyPropertyName => formatter.write_str("block property name cannot be empty"),
            Self::DuplicateProperty(name) => {
                write!(formatter, "duplicate block property: {name}")
            }
        }
    }
}

impl Error for BlockStateError {}
