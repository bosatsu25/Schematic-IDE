use schematic_core::{BlockPosition, Document, Position, Region, CHUNK_EDGE};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Fixability {
    None,
    Manual,
    Automatic,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub region: Option<String>,
    pub position: Option<[i32; 3]>,
    pub fixability: Fixability,
}

impl Diagnostic {
    pub fn new(
        severity: DiagnosticSeverity,
        code: impl Into<String>,
        message: impl Into<String>,
        region: Option<impl Into<String>>,
        position: Option<[i32; 3]>,
        fixability: Fixability,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
            region: region.map(Into::into),
            position,
            fixability,
        }
    }
}

pub struct DocumentValidator;

impl DocumentValidator {
    pub fn validate(document: &Document) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        Self::validate_metadata(document, &mut diagnostics);

        for region in document.regions() {
            Self::validate_region(region, &mut diagnostics);
        }

        Self::validate_block_entities(document, &mut diagnostics);
        Self::validate_entities(document, &mut diagnostics);

        diagnostics
    }

    fn validate_metadata(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
        let meta = document.metadata();
        if meta.name.as_deref().unwrap_or("").trim().is_empty() {
            diagnostics.push(Diagnostic::new(
                DiagnosticSeverity::Warning,
                "WARN_EMPTY_DOC_NAME",
                "Document metadata name is empty or missing",
                None::<String>,
                None,
                Fixability::Manual,
            ));
        }

        let mut total_volume: u128 = 0;
        for region in document.regions() {
            let size = region.size();
            let vol = (size.x as u128) * (size.y as u128) * (size.z as u128);
            total_volume = total_volume.saturating_add(vol);
        }
        if total_volume > 1_000_000_000 {
            diagnostics.push(Diagnostic::new(
                DiagnosticSeverity::Warning,
                "WARN_SERIALIZATION_SAFETY",
                format!("Total structure volume ({total_volume} blocks) exceeds 1B blocks; operations may be memory intensive"),
                None::<String>,
                None,
                Fixability::None,
            ));
        }
    }

    fn validate_region(region: &Region, diagnostics: &mut Vec<Diagnostic>) {
        let region_name = region.id().as_str();
        let size = region.size();

        if size.x == 0 || size.y == 0 || size.z == 0 {
            diagnostics.push(Diagnostic::new(
                DiagnosticSeverity::Warning,
                "WARN_ZERO_SIZE_REGION",
                format!(
                    "Region '{region_name}' has zero dimension: {}x{}x{}",
                    size.x, size.y, size.z
                ),
                Some(region_name),
                None,
                Fixability::Manual,
            ));
        }

        let palette = region.palette();
        let pal_len = palette.len();
        if pal_len == 0 {
            diagnostics.push(Diagnostic::new(
                DiagnosticSeverity::Error,
                "ERR_EMPTY_PALETTE",
                format!("Region '{region_name}' has an empty palette"),
                Some(region_name),
                None,
                Fixability::Automatic,
            ));
        } else {
            for (idx, state) in palette.iter() {
                let id = state.id();
                if !id.contains(':') || id.chars().any(|c| c.is_whitespace()) {
                    diagnostics.push(Diagnostic::new(
                        DiagnosticSeverity::Warning,
                        "WARN_SUSPICIOUS_BLOCK_ID",
                        format!("Region '{region_name}' palette index {} has non-standard block identifier '{id}'", idx.get()),
                        Some(region_name),
                        None,
                        Fixability::Manual,
                    ));
                }
            }
        }

        let size_x = size.x as i64;
        let size_y = size.y as i64;
        let size_z = size.z as i64;
        let world_pos = region.origin();

        for (chunk_pos, chunk) in region.chunks() {
            let min_local_x = chunk_pos.x * (CHUNK_EDGE as i64);
            let min_local_y = chunk_pos.y * (CHUNK_EDGE as i64);
            let min_local_z = chunk_pos.z * (CHUNK_EDGE as i64);

            let max_local_x = min_local_x + (CHUNK_EDGE as i64);
            let max_local_y = min_local_y + (CHUNK_EDGE as i64);
            let max_local_z = min_local_z + (CHUNK_EDGE as i64);

            if max_local_x <= 0
                || max_local_y <= 0
                || max_local_z <= 0
                || min_local_x >= size_x
                || min_local_y >= size_y
                || min_local_z >= size_z
            {
                let wx = (world_pos.x as i64 + min_local_x) as i32;
                let wy = (world_pos.y as i64 + min_local_y) as i32;
                let wz = (world_pos.z as i64 + min_local_z) as i32;
                diagnostics.push(Diagnostic::new(
                    DiagnosticSeverity::Error,
                    "ERR_OUT_OF_BOUNDS_CHUNK",
                    format!(
                        "Region '{region_name}' contains storage chunk at ({}, {}, {}) entirely outside bounds ({}x{}x{})",
                        chunk_pos.x, chunk_pos.y, chunk_pos.z, size.x, size.y, size.z
                    ),
                    Some(region_name),
                    Some([wx, wy, wz]),
                    Fixability::Automatic,
                ));
            }

            for local_x in 0..CHUNK_EDGE {
                for local_y in 0..CHUNK_EDGE {
                    for local_z in 0..CHUNK_EDGE {
                        let block_x = min_local_x + local_x as i64;
                        let block_y = min_local_y + local_y as i64;
                        let block_z = min_local_z + local_z as i64;

                        let index_opt = chunk_pos
                            .index_for_block(BlockPosition::new(block_x, block_y, block_z));

                        if let Some(idx) = index_opt {
                            if let Ok(Some(pal_idx)) = chunk.get(idx) {
                                let wx = (world_pos.x as i64 + block_x) as i32;
                                let wy = (world_pos.y as i64 + block_y) as i32;
                                let wz = (world_pos.z as i64 + block_z) as i32;

                                if (pal_idx.get() as usize) >= pal_len {
                                    diagnostics.push(Diagnostic::new(
                                        DiagnosticSeverity::Error,
                                        "ERR_INVALID_PALETTE_INDEX",
                                        format!(
                                            "Region '{region_name}' block at local ({block_x}, {block_y}, {block_z}) references missing palette index {}",
                                            pal_idx.get()
                                        ),
                                        Some(region_name),
                                        Some([wx, wy, wz]),
                                        Fixability::Automatic,
                                    ));
                                }

                                if block_x < 0
                                    || block_y < 0
                                    || block_z < 0
                                    || block_x >= size_x
                                    || block_y >= size_y
                                    || block_z >= size_z
                                {
                                    diagnostics.push(Diagnostic::new(
                                        DiagnosticSeverity::Error,
                                        "ERR_OUT_OF_BOUNDS_BLOCK",
                                        format!(
                                            "Region '{region_name}' block at local ({block_x}, {block_y}, {block_z}) is outside region size {}x{}x{}",
                                            size.x, size.y, size.z
                                        ),
                                        Some(region_name),
                                        Some([wx, wy, wz]),
                                        Fixability::Automatic,
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn validate_block_entities(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
        for be in document.block_entities() {
            let pos = be.position;
            let contained = document.regions().any(|r| r.contains(pos));
            if !contained {
                diagnostics.push(Diagnostic::new(
                    DiagnosticSeverity::Warning,
                    "WARN_BLOCK_ENTITY_OUTSIDE_REGIONS",
                    format!(
                        "Block entity '{}' at ({}, {}, {}) is not contained within any region",
                        be.kind, pos.x, pos.y, pos.z
                    ),
                    None::<String>,
                    Some([pos.x, pos.y, pos.z]),
                    Fixability::Manual,
                ));
            }
        }
    }

    fn validate_entities(document: &Document, diagnostics: &mut Vec<Diagnostic>) {
        for ent in document.entities() {
            let px = ent.position[0].floor() as i32;
            let py = ent.position[1].floor() as i32;
            let pz = ent.position[2].floor() as i32;
            let pos = Position::new(px, py, pz);
            let contained = document.regions().any(|r| r.contains(pos));
            if !contained {
                diagnostics.push(Diagnostic::new(
                    DiagnosticSeverity::Info,
                    "INFO_ENTITY_OUTSIDE_REGIONS",
                    format!(
                        "Entity '{}' at ({:.1}, {:.1}, {:.1}) is outside all region bounding boxes",
                        ent.kind, ent.position[0], ent.position[1], ent.position[2]
                    ),
                    None::<String>,
                    Some([px, py, pz]),
                    Fixability::None,
                ));
            }
        }
    }
}

pub fn validate_document(document: &Document) -> Vec<Diagnostic> {
    DocumentValidator::validate(document)
}
