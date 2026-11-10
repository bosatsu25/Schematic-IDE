//! Cross-format conversion helpers: flattening multi-region documents for
//! single-volume formats and reporting what a conversion cannot represent.

use crate::litematic::FormatError;
use schematic_core::{BlockEntityRef, Document, EntityRef, Position, Region, RegionId, Size};

/// Result of flattening a document into a single region.
#[derive(Clone, Debug)]
pub struct MergedDocument {
    pub document: Document,
    /// Names of the source regions when more than one was merged.
    pub merged_region_names: Vec<String>,
    /// Non-air blocks of an earlier region overwritten by a later region
    /// holding a *different* block at the same world position.
    pub overlap_conflicts: usize,
}

/// Flattens every region of `doc` into one region spanning their union
/// bounding box. Regions are applied in id order; on overlap the later region
/// wins and the conflict is counted. Block entities and entities are re-based
/// to the merged region. Single-region documents are returned unchanged.
pub fn merge_regions(doc: &Document) -> Result<MergedDocument, FormatError> {
    let regions: Vec<&Region> = doc.regions().collect();
    if regions.len() <= 1 {
        return Ok(MergedDocument {
            document: doc.clone(),
            merged_region_names: Vec::new(),
            overlap_conflicts: 0,
        });
    }

    let mut min = [i64::MAX; 3];
    let mut max = [i64::MIN; 3];
    for region in &regions {
        let o = region.origin();
        let s = region.size();
        let lo = [o.x as i64, o.y as i64, o.z as i64];
        let hi = [lo[0] + s.x as i64, lo[1] + s.y as i64, lo[2] + s.z as i64];
        for i in 0..3 {
            min[i] = min[i].min(lo[i]);
            max[i] = max[i].max(hi[i]);
        }
    }
    let to_u32 = |v: i64| {
        u32::try_from(v)
            .map_err(|_| FormatError::InvalidStructure("merged region is too large".to_string()))
    };
    let to_i32 = |v: i64| {
        i32::try_from(v).map_err(|_| {
            FormatError::InvalidStructure("merged region origin out of range".to_string())
        })
    };
    let origin = Position::new(to_i32(min[0])?, to_i32(min[1])?, to_i32(min[2])?);
    let size = Size::new(
        to_u32(max[0] - min[0])?,
        to_u32(max[1] - min[1])?,
        to_u32(max[2] - min[2])?,
    );
    if crate::litematic::region_volume(size.x, size.y, size.z).is_none() {
        return Err(FormatError::InvalidStructure(format!(
            "regions are too far apart to merge into one volume ({}x{}x{})",
            size.x, size.y, size.z
        )));
    }
    let merged_id = RegionId::new("Main");
    let mut merged = Region::new(merged_id.clone(), origin, size);
    let mut overlap_conflicts = 0usize;

    for region in &regions {
        for (chunk_pos, chunk) in region.chunks() {
            for (index, palette_index) in chunk.occupied_blocks() {
                let Some(local) = chunk_pos.block_position_for_index(index) else {
                    continue;
                };
                let Some(world) = region.local_to_world(local) else {
                    continue;
                };
                let Some(state) = region.palette().get(palette_index) else {
                    continue;
                };
                let Some(target) = merged.world_to_local(world) else {
                    continue;
                };
                let new_index = merged
                    .palette_mut()
                    .intern(state.clone())
                    .map_err(|e| FormatError::InvalidStructure(format!("palette error: {e}")))?;
                if let Ok(Some(existing)) = merged.block_index_at(target) {
                    if existing != new_index {
                        overlap_conflicts += 1;
                    }
                }
                merged
                    .set_block_index(target, Some(new_index))
                    .map_err(|e| FormatError::InvalidStructure(format!("merge error: {e}")))?;
            }
        }
    }

    let mut document = Document::new(doc.metadata().clone());
    for be in doc.block_entities() {
        let Some(region) = doc.region(&be.region) else {
            continue;
        };
        let Some(world) = region.local_to_world(be.position) else {
            continue;
        };
        if let Some(local) = merged.world_to_local(world) {
            document.add_block_entity(
                BlockEntityRef::new(merged_id.clone(), be.kind.clone(), local)
                    .with_data(be.data.clone()),
            );
        }
    }
    for entity in doc.entities() {
        let Some(region) = doc.region(&entity.region) else {
            continue;
        };
        let ro = region.origin();
        let shift = [
            (ro.x - origin.x) as f64,
            (ro.y - origin.y) as f64,
            (ro.z - origin.z) as f64,
        ];
        document.add_entity(
            EntityRef::new(
                merged_id.clone(),
                entity.kind.clone(),
                [
                    entity.position[0] + shift[0],
                    entity.position[1] + shift[1],
                    entity.position[2] + shift[2],
                ],
            )
            .with_data(entity.data.clone()),
        );
    }
    document.insert_region(merged);

    Ok(MergedDocument {
        document,
        merged_region_names: regions
            .iter()
            .map(|r| r.id().as_str().to_string())
            .collect(),
        overlap_conflicts,
    })
}

/// Export targets understood by [`conversion_report`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExportTarget {
    Litematic,
    Sponge,
    Structure,
}

impl ExportTarget {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "litematic" => Some(Self::Litematic),
            "sponge" | "schem" => Some(Self::Sponge),
            "structure" | "nbt" => Some(Self::Structure),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Litematic => ".litematic",
            Self::Sponge => "Sponge .schem",
            Self::Structure => "Structure .nbt",
        }
    }

    fn matches(self, source: crate::SchematicFormatType) -> bool {
        matches!(
            (self, source),
            (Self::Litematic, crate::SchematicFormatType::Litematic)
                | (Self::Sponge, crate::SchematicFormatType::SpongeSchematic)
                | (Self::Structure, crate::SchematicFormatType::StructureNbt)
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LossSeverity {
    /// Export is refused.
    Error,
    /// Data visible in the game or the editor is lost or altered.
    Warning,
    /// Format bookkeeping is lost; the build itself is intact.
    Info,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversionLoss {
    pub code: String,
    pub severity: LossSeverity,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversionReport {
    pub target: ExportTarget,
    pub losses: Vec<ConversionLoss>,
}

impl ConversionReport {
    pub fn is_lossless(&self) -> bool {
        self.losses.is_empty()
    }

    pub fn is_blocked(&self) -> bool {
        self.losses
            .iter()
            .any(|l| l.severity == LossSeverity::Error)
    }
}

/// What the loaded source file carried beyond the core document.
#[derive(Clone, Debug, Default)]
pub struct SourceExtras {
    pub format: Option<crate::SchematicFormatType>,
    /// Format-specific tags only the source format can store (unknown root
    /// keys, litematic metadata extras, per-region unknown tags, ...).
    pub format_specific_keys: Vec<String>,
    /// Litematic pending block/fluid ticks.
    pub pending_tick_count: usize,
}

/// Lists everything exporting `doc` to `target` cannot represent, so the UI
/// can ask before any data is dropped (no silent data loss).
pub fn conversion_report(
    doc: &Document,
    source: &SourceExtras,
    target: ExportTarget,
) -> Result<ConversionReport, FormatError> {
    let mut losses = Vec::new();
    let mut push = |code: &str, severity: LossSeverity, message: String| {
        losses.push(ConversionLoss {
            code: code.to_string(),
            severity,
            message,
        });
    };
    let single_volume = matches!(target, ExportTarget::Sponge | ExportTarget::Structure);

    if single_volume && doc.regions().count() > 1 {
        let merged = match merge_regions(doc) {
            Ok(merged) => merged,
            Err(e) => {
                push("VOLUME_LIMIT", LossSeverity::Error, e.to_string());
                return Ok(ConversionReport { target, losses });
            }
        };
        push(
            "REGIONS_MERGED",
            LossSeverity::Warning,
            format!(
                "{} cannot hold multiple regions: {} will be merged into one volume and their names lost",
                target.label(),
                merged.merged_region_names.join(", ")
            ),
        );
        if merged.overlap_conflicts > 0 {
            push(
                "REGION_OVERLAP",
                LossSeverity::Warning,
                format!(
                    "{} block(s) where regions overlap with different blocks will be overwritten by the later region",
                    merged.overlap_conflicts
                ),
            );
        }
        let region = merged.document.regions().next();
        if let Some(region) = region {
            check_dimensions(target, region.size(), &mut push);
        }
    } else if let Some(region) = doc.regions().next() {
        if single_volume {
            check_dimensions(target, region.size(), &mut push);
        }
    }

    if target == ExportTarget::Structure {
        if let Some(region) = doc.regions().next() {
            let o = region.origin();
            if doc.regions().count() == 1 && (o.x, o.y, o.z) != (0, 0, 0) {
                push(
                    "ORIGIN_DROPPED",
                    LossSeverity::Info,
                    format!(
                        "Structure NBT has no origin; the placement offset ({}, {}, {}) is dropped",
                        o.x, o.y, o.z
                    ),
                );
            }
        }
    }

    let meta = doc.metadata();
    let has = |v: &Option<String>| v.as_deref().is_some_and(|s| !s.is_empty());
    let mut dropped_meta = Vec::new();
    match target {
        ExportTarget::Litematic => {}
        ExportTarget::Sponge => {
            if has(&meta.description) {
                dropped_meta.push("description");
            }
        }
        ExportTarget::Structure => {
            if has(&meta.name) {
                dropped_meta.push("name");
            }
            if has(&meta.author) {
                dropped_meta.push("author");
            }
            if has(&meta.description) {
                dropped_meta.push("description");
            }
        }
    }
    if !dropped_meta.is_empty() {
        push(
            "METADATA_DROPPED",
            LossSeverity::Info,
            format!(
                "{} has no field for: {}",
                target.label(),
                dropped_meta.join(", ")
            ),
        );
    }

    if target != ExportTarget::Litematic && source.pending_tick_count > 0 {
        push(
            "PENDING_TICKS_DROPPED",
            LossSeverity::Warning,
            format!(
                "{} scheduled block/fluid tick(s) cannot be stored in {} and will be dropped",
                source.pending_tick_count,
                target.label()
            ),
        );
    }

    if let Some(format) = source.format {
        if !target.matches(format) && !source.format_specific_keys.is_empty() {
            push(
                "FORMAT_SPECIFIC_TAGS_DROPPED",
                LossSeverity::Info,
                format!(
                    "Source-format-only tags will not be carried over: {}",
                    source.format_specific_keys.join(", ")
                ),
            );
        }
    }

    Ok(ConversionReport { target, losses })
}

fn check_dimensions(
    target: ExportTarget,
    size: Size,
    push: &mut impl FnMut(&str, LossSeverity, String),
) {
    let volume = (size.x as u64)
        .saturating_mul(size.y as u64)
        .saturating_mul(size.z as u64);
    if target == ExportTarget::Structure && volume > crate::STRUCTURE_MAX_EXPORT_VOLUME {
        push(
            "VOLUME_LIMIT",
            LossSeverity::Error,
            format!(
                "Structure NBT export is limited to {} blocks; this volume is {}x{}x{} ({volume} blocks)",
                crate::STRUCTURE_MAX_EXPORT_VOLUME,
                size.x,
                size.y,
                size.z
            ),
        );
    }
    let limit = crate::sponge::SPONGE_MAX_DIMENSION;
    if target == ExportTarget::Sponge && (size.x > limit || size.y > limit || size.z > limit) {
        push(
            "DIMENSION_LIMIT",
            LossSeverity::Error,
            format!(
                "Sponge schematics are limited to {limit} blocks per axis; this volume is {}x{}x{}",
                size.x, size.y, size.z
            ),
        );
    }
}
