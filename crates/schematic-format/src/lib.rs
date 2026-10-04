pub mod bit_packing;
pub mod litematic;
pub mod nbt;
pub mod sponge;
pub mod structure;

pub use litematic::{FormatError, LitematicDocument, RawRegionData};
pub use nbt::{decode_gzip_or_raw, encode_gzip, read_nbt, write_nbt, NbtError, NbtTag};
pub use sponge::SpongeSchematic;
pub use structure::StructureDocument;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchematicFormatType {
    Litematic,
    SpongeSchematic,
    StructureNbt,
}

pub fn detect_format(bytes: &[u8]) -> Result<SchematicFormatType, FormatError> {
    let (_root_name, root_tag) = decode_gzip_or_raw(bytes)?;
    let root = root_tag
        .as_compound()
        .ok_or_else(|| FormatError::InvalidStructure("Root tag is not a Compound".to_string()))?;

    if root.contains_key("Regions") && root.contains_key("Metadata") {
        return Ok(SchematicFormatType::Litematic);
    }

    if root.contains_key("Schematic")
        || (root.contains_key("Width")
            && root.contains_key("Height")
            && root.contains_key("Length")
            && root.contains_key("BlockData"))
    {
        return Ok(SchematicFormatType::SpongeSchematic);
    }

    if root.contains_key("size") && (root.contains_key("palette") || root.contains_key("palettes"))
    {
        return Ok(SchematicFormatType::StructureNbt);
    }

    Err(FormatError::InvalidStructure(
        "Unrecognized schematic format: neither Litematic, Sponge Schematic, nor Structure NBT"
            .to_string(),
    ))
}
