use schematic_analysis::{
    NonAirPolicy, SurfaceAnalyzer, SurfaceFeatureAnalyzer, SurfaceFeatureKind,
};
use schematic_core::{
    BlockPosition, BlockProperty, BlockState, OperationTarget, PaletteIndex, Position, RegionId,
    Selection,
};
use schematic_edit::{
    Clipboard, EditWorkspace, FillCommand, IslandCleanupRequest, MirrorAxis, MirrorCommand,
    MoveCommand, PasteCommand, ReplaceCommand, RotateCommand, RotationAngle,
};
use schematic_format::{LitematicDocument, NbtTag};
use schematic_validate::validate_document as core_validate_document;
pub use schematic_validate::{Diagnostic, DiagnosticSeverity, Fixability};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashSet};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SessionStatus {
    pub loaded: bool,
    pub has_preview: bool,
    pub can_undo: bool,
    pub can_redo: bool,
    pub is_dirty: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentSummary {
    pub name: String,
    pub author: String,
    pub description: String,
    pub minecraft_data_version: i32,
    pub version: i32,
    pub regions: Vec<RegionSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionSummary {
    pub name: String,
    pub origin: [i32; 3],
    pub size: [u32; 3],
    pub non_air_blocks: usize,
    pub palette: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionMeshData {
    pub region_id: String,
    pub origin: [i32; 3],
    pub size: [u32; 3],
    pub palette: Vec<String>,
    pub blocks: Vec<i32>, // flat [x, y, z, pal_idx, ...]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_diff: Option<PreviewDiff>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreviewDiff {
    pub modified_positions: Vec<[i32; 3]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplaceRequest {
    pub region_id: String,
    pub selection: Option<SelectionBounds>,
    pub from_block: String,
    pub to_block: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SelectionBounds {
    pub min: [i32; 3],
    pub max: [i32; 3],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FillRequest {
    pub region_id: String,
    pub selection: SelectionBounds,
    pub block: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CopyRequest {
    pub region_id: String,
    pub selection: SelectionBounds,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PasteRequest {
    pub region_id: String,
    pub target: [i32; 3],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MoveRequest {
    pub region_id: String,
    pub selection: SelectionBounds,
    pub delta: [i32; 3],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RotateRequest {
    pub region_id: String,
    pub selection: SelectionBounds,
    pub angle_deg: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MirrorRequest {
    pub region_id: String,
    pub selection: SelectionBounds,
    pub axis: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CleanupRequest {
    pub region_id: String,
    pub max_size: usize,
    pub replacement_block: Option<String>,
    pub protected_kinds: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreviewSummary {
    pub changed_count: usize,
    pub can_commit: bool,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistorySummary {
    pub can_undo: bool,
    pub can_redo: bool,
    pub has_preview: bool,
    pub is_dirty: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockInspectionRequest {
    pub region_id: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockInspection {
    pub region_id: String,
    pub local_position: [i32; 3],
    pub world_position: [i32; 3],
    pub block_id: String,
    pub properties: BTreeMap<String, String>,
    pub palette_index: u32,
    pub block_entity: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentInspection {
    pub metadata: DocumentMetadataInspection,
    pub regions: Vec<RegionInspectionSummary>,
    pub total_entities: usize,
    pub total_block_entities: usize,
    pub raw_nbt_keys: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentMetadataInspection {
    pub name: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub minecraft_data_version: i32,
    pub version: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionInspectionSummary {
    pub name: String,
    pub origin: [i32; 3],
    pub size: [u32; 3],
    pub palette_size: usize,
    pub non_air_blocks: usize,
    pub block_entity_count: usize,
    pub entity_count: usize,
}

pub struct Session {
    litematic: Option<LitematicDocument>,
    workspace: Option<EditWorkspace>,
    clipboard: Option<Clipboard>,
    response_buffer: Vec<u8>,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

impl Session {
    pub fn new() -> Self {
        Self {
            litematic: None,
            workspace: None,
            clipboard: None,
            response_buffer: Vec::new(),
        }
    }

    pub fn response_buffer(&self) -> &[u8] {
        &self.response_buffer
    }

    pub fn set_json_response<T: Serialize>(&mut self, value: &T) {
        self.response_buffer = serde_json::to_vec(value).unwrap_or_default();
    }

    pub fn set_bytes_response(&mut self, bytes: Vec<u8>) {
        self.response_buffer = bytes;
    }

    pub fn set_error_response(&mut self, err: &str) {
        #[derive(Serialize)]
        struct ErrorResp<'a> {
            error: &'a str,
        }
        self.response_buffer = serde_json::to_vec(&ErrorResp { error: err })
            .unwrap_or_else(|_| err.as_bytes().to_vec());
    }

    pub fn get_status(&self) -> SessionStatus {
        let Some(ws) = &self.workspace else {
            return SessionStatus::default();
        };
        SessionStatus {
            loaded: true,
            has_preview: ws.has_preview(),
            can_undo: ws.can_undo(),
            can_redo: ws.can_redo(),
            is_dirty: ws.is_dirty(),
        }
    }

    pub fn load_litematic(&mut self, bytes: &[u8]) -> Result<DocumentSummary, String> {
        let litematic = LitematicDocument::parse(bytes).map_err(|e| e.to_string())?;
        let workspace = EditWorkspace::start(litematic.document.clone());

        let doc = workspace.committed();
        let meta = doc.metadata();
        let mut regions = Vec::new();

        for region in doc.regions() {
            let mut non_air = 0;
            let mut pal_strings = Vec::new();
            for idx in 0..region.palette().len() {
                if let Some(state) = region.palette().get(PaletteIndex::new(idx as u32)) {
                    pal_strings.push(block_state_to_string(state));
                }
            }

            for (_, chunk) in region.chunks() {
                for (_, pal_idx) in chunk.occupied_blocks() {
                    if let Some(st) = region.palette().get(pal_idx) {
                        if !is_air(st.id()) {
                            non_air += 1;
                        }
                    }
                }
            }

            regions.push(RegionSummary {
                name: region.id().as_str().to_string(),
                origin: [region.origin().x, region.origin().y, region.origin().z],
                size: [region.size().x, region.size().y, region.size().z],
                non_air_blocks: non_air,
                palette: pal_strings,
            });
        }

        let summary = DocumentSummary {
            name: meta.name.clone().unwrap_or_default(),
            author: meta.author.clone().unwrap_or_default(),
            description: meta.description.clone().unwrap_or_default(),
            minecraft_data_version: litematic.data_version,
            version: litematic.version,
            regions,
        };

        self.litematic = Some(litematic);
        self.workspace = Some(workspace);

        Ok(summary)
    }

    pub fn get_region_mesh(&self, region_name: &str) -> Result<RegionMeshData, String> {
        let ws = self.workspace.as_ref().ok_or("No document loaded")?;
        let reg_id = RegionId::new(region_name);
        let region = ws
            .committed()
            .region(&reg_id)
            .ok_or_else(|| format!("Region '{region_name}' not found"))?;

        // Extract palette
        let mut palette = Vec::new();
        for idx in 0..region.palette().len() {
            if let Some(state) = region.palette().get(PaletteIndex::new(idx as u32)) {
                palette.push(block_state_to_string(state));
            }
        }

        // Collect preview changes if preview is active
        let mut preview_overrides = BTreeMap::<BlockPosition, Option<PaletteIndex>>::new();
        let mut modified_positions = Vec::new();

        if let Some(patch) = ws.pending_preview() {
            for p in patch.patches() {
                if p.region_id() == &reg_id {
                    for ch in p.changes() {
                        preview_overrides.insert(ch.position(), ch.after());
                        modified_positions.push([
                            ch.position().x as i32,
                            ch.position().y as i32,
                            ch.position().z as i32,
                        ]);
                    }
                }
            }
        }

        // Build flat blocks list [x, y, z, pal_idx, ...]
        let mut blocks = Vec::new();
        let mut seen = HashSet::<BlockPosition>::new();

        for (chunk_pos, chunk) in region.chunks() {
            for (idx, pal_idx) in chunk.occupied_blocks() {
                if let Some(local_pos) = chunk_pos.block_position_for_index(idx) {
                    seen.insert(local_pos);
                    let final_idx = match preview_overrides.get(&local_pos) {
                        Some(over) => *over,
                        None => Some(pal_idx),
                    };

                    if let Some(pidx) = final_idx {
                        if let Some(state) = region.palette().get(pidx) {
                            if !is_air(state.id()) {
                                blocks.push(local_pos.x as i32);
                                blocks.push(local_pos.y as i32);
                                blocks.push(local_pos.z as i32);
                                blocks.push(pidx.get() as i32);
                            }
                        }
                    }
                }
            }
        }

        // Also check any new blocks introduced in preview at positions that were previously empty
        for (pos, maybe_pidx) in &preview_overrides {
            if !seen.contains(pos) {
                if let Some(pidx) = maybe_pidx {
                    if let Some(state) = region.palette().get(*pidx) {
                        if !is_air(state.id()) {
                            blocks.push(pos.x as i32);
                            blocks.push(pos.y as i32);
                            blocks.push(pos.z as i32);
                            blocks.push(pidx.get() as i32);
                        }
                    }
                }
            }
        }

        let preview_diff = if ws.has_preview() {
            Some(PreviewDiff { modified_positions })
        } else {
            None
        };

        Ok(RegionMeshData {
            region_id: region_name.to_string(),
            origin: [region.origin().x, region.origin().y, region.origin().z],
            size: [region.size().x, region.size().y, region.size().z],
            palette,
            blocks,
            preview_diff,
        })
    }

    pub fn preview_replace(&mut self, req: ReplaceRequest) -> Result<PreviewSummary, String> {
        let ws = self.workspace.as_mut().ok_or("No document loaded")?;
        let reg_id = RegionId::new(&req.region_id);
        let region = ws
            .committed()
            .region(&reg_id)
            .ok_or_else(|| format!("Region '{}' not found", req.region_id))?;

        let selection = match req.selection {
            Some(b) => Selection::from_corners(
                Position::new(b.min[0], b.min[1], b.min[2]),
                Position::new(b.max[0], b.max[1], b.max[2]),
            ),
            None => Selection::from_bounds(region.bounds()),
        };

        let from_state = parse_block_state(&req.from_block)?;
        let to_state = parse_block_state(&req.to_block)?;

        let cmd = ReplaceCommand::new(reg_id, selection, from_state.clone(), to_state.clone());
        ws.preview_command(&cmd).map_err(|e| e.to_string())?;

        let changed_count = ws
            .pending_preview()
            .map(|p| p.changed_block_count())
            .unwrap_or(0);

        Ok(PreviewSummary {
            changed_count,
            can_commit: changed_count > 0,
            message: format!(
                "Replaced {} block(s) from '{}' to '{}'",
                changed_count, req.from_block, req.to_block
            ),
        })
    }

    pub fn preview_fill(&mut self, req: FillRequest) -> Result<PreviewSummary, String> {
        let ws = self.workspace.as_mut().ok_or("No document loaded")?;
        let reg_id = RegionId::new(&req.region_id);
        let _ = ws
            .committed()
            .region(&reg_id)
            .ok_or_else(|| format!("Region '{}' not found", req.region_id))?;

        let selection = Selection::from_corners(
            Position::new(
                req.selection.min[0],
                req.selection.min[1],
                req.selection.min[2],
            ),
            Position::new(
                req.selection.max[0],
                req.selection.max[1],
                req.selection.max[2],
            ),
        );

        let state = parse_block_state(&req.block)?;
        let cmd = FillCommand::new(reg_id, selection, state);
        ws.preview_command(&cmd).map_err(|e| e.to_string())?;

        let changed_count = ws
            .pending_preview()
            .map(|p| p.changed_block_count())
            .unwrap_or(0);

        Ok(PreviewSummary {
            changed_count,
            can_commit: changed_count > 0,
            message: format!("Filled {} block(s) with '{}'", changed_count, req.block),
        })
    }

    pub fn copy_selection(&mut self, req: CopyRequest) -> Result<usize, String> {
        let ws = self.workspace.as_mut().ok_or("No document loaded")?;
        let reg_id = RegionId::new(&req.region_id);
        let region = ws
            .committed()
            .region(&reg_id)
            .ok_or_else(|| format!("Region '{}' not found", req.region_id))?;

        let selection = Selection::from_corners(
            Position::new(
                req.selection.min[0],
                req.selection.min[1],
                req.selection.min[2],
            ),
            Position::new(
                req.selection.max[0],
                req.selection.max[1],
                req.selection.max[2],
            ),
        );

        let clipboard =
            Clipboard::from_region_selection(region, &selection).map_err(|e| e.to_string())?;
        let count = clipboard.blocks.len();
        self.clipboard = Some(clipboard);
        Ok(count)
    }

    pub fn preview_paste(&mut self, req: PasteRequest) -> Result<PreviewSummary, String> {
        let clipboard = self.clipboard.clone().ok_or("Clipboard is empty")?;
        let ws = self.workspace.as_mut().ok_or("No document loaded")?;
        let reg_id = RegionId::new(&req.region_id);
        let _ = ws
            .committed()
            .region(&reg_id)
            .ok_or_else(|| format!("Region '{}' not found", req.region_id))?;

        let target_pos = Position::new(req.target[0], req.target[1], req.target[2]);
        let cmd = PasteCommand::new(reg_id, target_pos, clipboard);
        ws.preview_command(&cmd).map_err(|e| e.to_string())?;

        let changed_count = ws
            .pending_preview()
            .map(|p| p.changed_block_count())
            .unwrap_or(0);

        Ok(PreviewSummary {
            changed_count,
            can_commit: changed_count > 0,
            message: format!("Pasted {} block(s)", changed_count),
        })
    }

    pub fn preview_move(&mut self, req: MoveRequest) -> Result<PreviewSummary, String> {
        let ws = self.workspace.as_mut().ok_or("No document loaded")?;
        let reg_id = RegionId::new(&req.region_id);
        let _ = ws
            .committed()
            .region(&reg_id)
            .ok_or_else(|| format!("Region '{}' not found", req.region_id))?;

        let selection = Selection::from_corners(
            Position::new(
                req.selection.min[0],
                req.selection.min[1],
                req.selection.min[2],
            ),
            Position::new(
                req.selection.max[0],
                req.selection.max[1],
                req.selection.max[2],
            ),
        );

        let cmd = MoveCommand::new(reg_id, selection, req.delta);
        ws.preview_command(&cmd).map_err(|e| e.to_string())?;

        let changed_count = ws
            .pending_preview()
            .map(|p| p.changed_block_count())
            .unwrap_or(0);

        Ok(PreviewSummary {
            changed_count,
            can_commit: changed_count > 0,
            message: format!(
                "Moved {} block(s) by [{}, {}, {}]",
                changed_count, req.delta[0], req.delta[1], req.delta[2]
            ),
        })
    }

    pub fn preview_rotate(&mut self, req: RotateRequest) -> Result<PreviewSummary, String> {
        let ws = self.workspace.as_mut().ok_or("No document loaded")?;
        let reg_id = RegionId::new(&req.region_id);
        let _ = ws
            .committed()
            .region(&reg_id)
            .ok_or_else(|| format!("Region '{}' not found", req.region_id))?;

        let selection = Selection::from_corners(
            Position::new(
                req.selection.min[0],
                req.selection.min[1],
                req.selection.min[2],
            ),
            Position::new(
                req.selection.max[0],
                req.selection.max[1],
                req.selection.max[2],
            ),
        );

        let angle = match req.angle_deg {
            90 => RotationAngle::Deg90,
            180 => RotationAngle::Deg180,
            270 => RotationAngle::Deg270,
            other => {
                return Err(format!(
                    "Unsupported rotation angle: {other} (must be 90, 180, or 270)"
                ))
            }
        };

        let cmd = RotateCommand::new(reg_id, selection, angle);
        ws.preview_command(&cmd).map_err(|e| e.to_string())?;

        let changed_count = ws
            .pending_preview()
            .map(|p| p.changed_block_count())
            .unwrap_or(0);

        Ok(PreviewSummary {
            changed_count,
            can_commit: changed_count > 0,
            message: format!("Rotated {} block(s) by {}°", changed_count, req.angle_deg),
        })
    }

    pub fn preview_mirror(&mut self, req: MirrorRequest) -> Result<PreviewSummary, String> {
        let ws = self.workspace.as_mut().ok_or("No document loaded")?;
        let reg_id = RegionId::new(&req.region_id);
        let _ = ws
            .committed()
            .region(&reg_id)
            .ok_or_else(|| format!("Region '{}' not found", req.region_id))?;

        let selection = Selection::from_corners(
            Position::new(
                req.selection.min[0],
                req.selection.min[1],
                req.selection.min[2],
            ),
            Position::new(
                req.selection.max[0],
                req.selection.max[1],
                req.selection.max[2],
            ),
        );

        let axis = match req.axis.to_ascii_lowercase().as_str() {
            "x" => MirrorAxis::X,
            "z" => MirrorAxis::Z,
            other => {
                return Err(format!(
                    "Unsupported mirror axis: {other} (must be 'x' or 'z')"
                ))
            }
        };

        let cmd = MirrorCommand::new(reg_id, selection, axis);
        ws.preview_command(&cmd).map_err(|e| e.to_string())?;

        let changed_count = ws
            .pending_preview()
            .map(|p| p.changed_block_count())
            .unwrap_or(0);

        Ok(PreviewSummary {
            changed_count,
            can_commit: changed_count > 0,
            message: format!(
                "Mirrored {} block(s) along {}-axis",
                changed_count,
                req.axis.to_uppercase()
            ),
        })
    }

    pub fn preview_cleanup(&mut self, req: CleanupRequest) -> Result<PreviewSummary, String> {
        let ws = self.workspace.as_mut().ok_or("No document loaded")?;
        let reg_id = RegionId::new(&req.region_id);
        let region = ws
            .committed()
            .region(&reg_id)
            .ok_or_else(|| format!("Region '{}' not found", req.region_id))?;

        let target = OperationTarget::unconstrained(Selection::from_bounds(region.bounds()));
        let policy = NonAirPolicy;
        let surface = SurfaceAnalyzer::analyze(ws.committed(), &reg_id, &target, &policy)
            .map_err(|e| format!("{e:?}"))?;
        let features = SurfaceFeatureAnalyzer::analyze(&surface);

        let repl_str = req.replacement_block.as_deref().unwrap_or("minecraft:air");
        let replacement_state = parse_block_state(repl_str)?;

        let mut protected = BTreeSet::new();
        for kind_str in req.protected_kinds {
            match kind_str.to_ascii_lowercase().replace('_', "").as_str() {
                "tip" => {
                    protected.insert(SurfaceFeatureKind::Tip);
                }
                "thin" | "thinfeature" => {
                    protected.insert(SurfaceFeatureKind::ThinFeature);
                }
                "edge" => {
                    protected.insert(SurfaceFeatureKind::Edge);
                }
                "corner" => {
                    protected.insert(SurfaceFeatureKind::Corner);
                }
                "face" => {
                    protected.insert(SurfaceFeatureKind::Face);
                }
                _ => {}
            }
        }

        let cleanup_req = IslandCleanupRequest::new(req.max_size, replacement_state, protected)
            .map_err(|e| e.to_string())?;

        ws.preview_island_cleanup(&reg_id, &surface, &features, &cleanup_req)
            .map_err(|e| e.to_string())?;

        let changed_count = ws
            .pending_preview()
            .map(|p| p.changed_block_count())
            .unwrap_or(0);

        Ok(PreviewSummary {
            changed_count,
            can_commit: changed_count > 0,
            message: format!(
                "Island cleanup: {} candidate block(s) identified for removal (max size <= {})",
                changed_count, req.max_size
            ),
        })
    }

    pub fn commit_preview(&mut self) -> Result<HistorySummary, String> {
        let ws = self.workspace.as_mut().ok_or("No document loaded")?;
        ws.commit_preview().map_err(|e| e.to_string())?;
        Ok(HistorySummary {
            can_undo: ws.can_undo(),
            can_redo: ws.can_redo(),
            has_preview: ws.has_preview(),
            is_dirty: ws.is_dirty(),
        })
    }

    pub fn cancel_preview(&mut self) -> Result<HistorySummary, String> {
        let ws = self.workspace.as_mut().ok_or("No document loaded")?;
        ws.cancel_preview();
        Ok(HistorySummary {
            can_undo: ws.can_undo(),
            can_redo: ws.can_redo(),
            has_preview: ws.has_preview(),
            is_dirty: ws.is_dirty(),
        })
    }

    pub fn undo(&mut self) -> Result<HistorySummary, String> {
        let ws = self.workspace.as_mut().ok_or("No document loaded")?;
        ws.undo().map_err(|e| e.to_string())?;
        Ok(HistorySummary {
            can_undo: ws.can_undo(),
            can_redo: ws.can_redo(),
            has_preview: ws.has_preview(),
            is_dirty: ws.is_dirty(),
        })
    }

    pub fn redo(&mut self) -> Result<HistorySummary, String> {
        let ws = self.workspace.as_mut().ok_or("No document loaded")?;
        ws.redo().map_err(|e| e.to_string())?;
        Ok(HistorySummary {
            can_undo: ws.can_undo(),
            can_redo: ws.can_redo(),
            has_preview: ws.has_preview(),
            is_dirty: ws.is_dirty(),
        })
    }

    pub fn export_litematic(&self) -> Result<Vec<u8>, String> {
        let ws = self.workspace.as_ref().ok_or("No document loaded")?;
        let litematic = self.litematic.as_ref().ok_or("No litematic template")?;
        litematic.export(ws.committed()).map_err(|e| e.to_string())
    }

    pub fn inspect_block(
        &self,
        req: &BlockInspectionRequest,
    ) -> Result<Option<BlockInspection>, String> {
        let ws = self
            .workspace
            .as_ref()
            .ok_or_else(|| "No document loaded".to_string())?;
        let doc = ws.committed();
        let region_id = RegionId::new(&req.region_id);
        let region = match doc.region(&region_id) {
            Some(r) => r,
            None => return Ok(None),
        };

        let local_pos = BlockPosition::new(req.x as i64, req.y as i64, req.z as i64);
        let pal_idx = match region.block_index_at(local_pos) {
            Ok(Some(idx)) => idx,
            Ok(None) => return Ok(None),
            Err(_) => return Ok(None),
        };

        let block_state = match region.palette().get(pal_idx) {
            Some(s) => s,
            None => return Ok(None),
        };

        let mut properties = BTreeMap::new();
        for prop in block_state.properties() {
            properties.insert(prop.name().to_string(), prop.value().to_string());
        }

        let world_pos = region
            .local_to_world(local_pos)
            .map(|p| [p.x, p.y, p.z])
            .unwrap_or([
                region.origin().x + req.x,
                region.origin().y + req.y,
                region.origin().z + req.z,
            ]);

        let mut block_entity_json = None;
        if let Some(litematic) = &self.litematic {
            if let Some(raw_region) = litematic.raw_regions.get(&region_id) {
                for tag in &raw_region.block_entities {
                    if let Some(compound) = tag.as_compound() {
                        let bx = compound.get("x").and_then(|t| match t {
                            NbtTag::Int(v) => Some(*v),
                            NbtTag::Short(v) => Some(*v as i32),
                            _ => None,
                        });
                        let by = compound.get("y").and_then(|t| match t {
                            NbtTag::Int(v) => Some(*v),
                            NbtTag::Short(v) => Some(*v as i32),
                            _ => None,
                        });
                        let bz = compound.get("z").and_then(|t| match t {
                            NbtTag::Int(v) => Some(*v),
                            NbtTag::Short(v) => Some(*v as i32),
                            _ => None,
                        });

                        if (bx == Some(req.x) && by == Some(req.y) && bz == Some(req.z))
                            || (bx == Some(world_pos[0])
                                && by == Some(world_pos[1])
                                && bz == Some(world_pos[2]))
                        {
                            block_entity_json = Some(nbt_to_json(tag));
                            break;
                        }
                    }
                }
            }
        }

        Ok(Some(BlockInspection {
            region_id: req.region_id.clone(),
            local_position: [req.x, req.y, req.z],
            world_position: world_pos,
            block_id: block_state.id().to_string(),
            properties,
            palette_index: pal_idx.get(),
            block_entity: block_entity_json,
        }))
    }

    pub fn inspect_document(&self) -> Result<DocumentInspection, String> {
        let ws = self
            .workspace
            .as_ref()
            .ok_or_else(|| "No document loaded".to_string())?;
        let doc = ws.committed();
        let litematic = self.litematic.as_ref();

        let meta = doc.metadata();
        let metadata = DocumentMetadataInspection {
            name: meta.name.clone(),
            author: meta.author.clone(),
            description: meta.description.clone(),
            minecraft_data_version: litematic.map(|l| l.data_version).unwrap_or(0),
            version: litematic.map(|l| l.version).unwrap_or(0),
        };

        let mut regions = Vec::new();
        let mut total_be = 0;
        let mut total_ent = 0;

        for r in doc.regions() {
            let region_id = r.id();
            let (be_count, ent_count) = if let Some(l) = litematic {
                if let Some(raw) = l.raw_regions.get(region_id) {
                    (raw.block_entities.len(), raw.entities.len())
                } else {
                    (0, 0)
                }
            } else {
                (0, 0)
            };
            total_be += be_count;
            total_ent += ent_count;

            let mut non_air = 0;
            let palette = r.palette();
            for (_, chunk) in r.chunks() {
                for idx in 0..schematic_core::CHUNK_VOLUME {
                    if let Ok(Some(pal_idx)) = chunk.get(idx) {
                        if let Some(state) = palette.get(pal_idx) {
                            if !is_air(state.id()) {
                                non_air += 1;
                            }
                        }
                    }
                }
            }

            regions.push(RegionInspectionSummary {
                name: region_id.as_str().to_string(),
                origin: [r.origin().x, r.origin().y, r.origin().z],
                size: [r.size().x, r.size().y, r.size().z],
                palette_size: palette.len(),
                non_air_blocks: non_air,
                block_entity_count: be_count,
                entity_count: ent_count,
            });
        }

        let raw_nbt_keys = if let Some(l) = litematic {
            let mut keys: Vec<String> = l.raw_root_unknown.keys().cloned().collect();
            keys.extend(l.raw_metadata.keys().cloned());
            keys.sort();
            keys.dedup();
            keys
        } else {
            Vec::new()
        };

        Ok(DocumentInspection {
            metadata,
            regions,
            total_entities: total_ent,
            total_block_entities: total_be,
            raw_nbt_keys,
        })
    }

    pub fn validate_document(&self) -> Result<Vec<Diagnostic>, String> {
        let ws = self
            .workspace
            .as_ref()
            .ok_or_else(|| "No document loaded".to_string())?;
        Ok(core_validate_document(ws.committed()))
    }

    pub fn diff_with_source(&self) -> Result<schematic_diff::DocumentDiff, String> {
        let ws = self
            .workspace
            .as_ref()
            .ok_or_else(|| "No session active".to_string())?;
        let orig = self
            .litematic
            .as_ref()
            .ok_or_else(|| "No source document available".to_string())?;
        Ok(schematic_diff::diff_documents(
            &orig.document,
            ws.committed(),
            false,
        ))
    }

    pub fn diff_with_litematic(
        &self,
        bytes: &[u8],
    ) -> Result<schematic_diff::DocumentDiff, String> {
        let ws = self
            .workspace
            .as_ref()
            .ok_or_else(|| "No session active".to_string())?;
        let other = LitematicDocument::parse(bytes).map_err(|e| e.to_string())?;
        Ok(schematic_diff::diff_documents(
            &other.document,
            ws.committed(),
            false,
        ))
    }
}

pub fn nbt_to_json(tag: &NbtTag) -> serde_json::Value {
    match tag {
        NbtTag::End => serde_json::Value::Null,
        NbtTag::Byte(b) => serde_json::json!(b),
        NbtTag::Short(s) => serde_json::json!(s),
        NbtTag::Int(i) => serde_json::json!(i),
        NbtTag::Long(l) => serde_json::json!(l),
        NbtTag::Float(f) => serde_json::json!(f),
        NbtTag::Double(d) => serde_json::json!(d),
        NbtTag::ByteArray(bytes) => serde_json::json!(bytes),
        NbtTag::String(s) => serde_json::json!(s),
        NbtTag::List(_, list) => serde_json::Value::Array(list.iter().map(nbt_to_json).collect()),
        NbtTag::Compound(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                obj.insert(k.clone(), nbt_to_json(v));
            }
            serde_json::Value::Object(obj)
        }
        NbtTag::IntArray(ints) => serde_json::json!(ints),
        NbtTag::LongArray(longs) => serde_json::json!(longs),
    }
}

pub fn is_air(id: &str) -> bool {
    matches!(
        id,
        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
    )
}

pub fn block_state_to_string(state: &BlockState) -> String {
    let mut props: Vec<String> = state
        .properties()
        .map(|p| format!("{}={}", p.name(), p.value()))
        .collect();
    if props.is_empty() {
        state.id().to_string()
    } else {
        props.sort();
        format!("{}[{}]", state.id(), props.join(","))
    }
}

pub fn parse_block_state(s: &str) -> Result<BlockState, String> {
    let s = s.trim();
    if let Some(open) = s.find('[') {
        if !s.ends_with(']') {
            return Err("missing closing ']' on block state".to_string());
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
                return Err("empty property name".to_string());
            }
            props.push(BlockProperty::new(name, val));
        }
        BlockState::new(id, props).map_err(|e| e.to_string())
    } else {
        BlockState::new(s, []).map_err(|e| e.to_string())
    }
}
