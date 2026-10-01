#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DocumentMetadata {
    pub name: Option<String>,
    pub description: Option<String>,
    pub author: Option<String>,
    pub created_at_unix_ms: Option<i64>,
    pub modified_at_unix_ms: Option<i64>,
}
