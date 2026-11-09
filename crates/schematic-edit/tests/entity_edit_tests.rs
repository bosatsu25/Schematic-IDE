//! Block entities and entities must travel with their blocks through every
//! edit, and undo/redo must restore them exactly (issue #21).

use schematic_core::{
    BlockEntityRef, BlockPosition, BlockState, Document, DocumentMetadata, EntityRef, NbtCompound,
    NbtTag, Position, Region, RegionId, Selection, Size,
};
use schematic_edit::{
    Clipboard, DeleteCommand, EditCommand, EditWorkspace, FillCommand, MirrorAxis, MirrorCommand,
    MoveCommand, PasteCommand, ReplaceCommand, RotateCommand, RotationAngle,
};
use schematic_validate::validate_document;

fn rid() -> RegionId {
    RegionId::new("main")
}

fn state(id: &str) -> BlockState {
    BlockState::new(id, []).unwrap()
}

fn chest_items(tag: &str) -> NbtCompound {
    let mut item = NbtCompound::new();
    item.insert("id".into(), NbtTag::String("minecraft:diamond".into()));
    item.insert("Count".into(), NbtTag::Byte(64));
    item.insert("Slot".into(), NbtTag::Byte(0));
    let mut data = NbtCompound::new();
    data.insert(
        "Items".into(),
        NbtTag::List(10, vec![NbtTag::Compound(item)]),
    );
    data.insert("CustomName".into(), NbtTag::String(tag.into()));
    data
}

fn yaw_data(yaw: f32, uuid: bool) -> NbtCompound {
    let mut data = NbtCompound::new();
    data.insert(
        "Rotation".into(),
        NbtTag::List(5, vec![NbtTag::Float(yaw), NbtTag::Float(0.0)]),
    );
    if uuid {
        data.insert("UUID".into(), NbtTag::IntArray(vec![1, 2, 3, 4]));
    }
    data
}

/// 8x4x8 region at world origin (10, 0, 10): stone floor, a chest with items at
/// local (2,1,2), and an armor stand standing on (2,1,3).
fn fixture() -> Document {
    let mut doc = Document::new(DocumentMetadata::default());
    let mut region = Region::new(rid(), Position::new(10, 0, 10), Size::new(8, 4, 8));
    let stone = region
        .palette_mut()
        .intern(state("minecraft:stone"))
        .unwrap();
    let chest = region
        .palette_mut()
        .intern(state("minecraft:chest"))
        .unwrap();
    for x in 0..8 {
        for z in 0..8 {
            region
                .set_block_index(BlockPosition::new(x, 0, z), Some(stone))
                .unwrap();
        }
    }
    region
        .set_block_index(BlockPosition::new(2, 1, 2), Some(chest))
        .unwrap();
    doc.insert_region(region);
    doc.add_block_entity(
        BlockEntityRef::new(rid(), "minecraft:chest", BlockPosition::new(2, 1, 2))
            .with_data(chest_items("Treasure")),
    );
    doc.add_entity(
        EntityRef::new(rid(), "minecraft:armor_stand", [2.5, 1.0, 3.5])
            .with_data(yaw_data(180.0, true)),
    );
    doc
}

fn world_sel(min: [i32; 3], max: [i32; 3]) -> Selection {
    Selection::from_corners(
        Position::new(min[0], min[1], min[2]),
        Position::new(max[0], max[1], max[2]),
    )
}

fn commit(ws: &mut EditWorkspace, cmd: &impl EditCommand) {
    ws.preview_command(cmd).unwrap();
    ws.commit_preview().unwrap();
}

fn be_at(doc: &Document, p: [i64; 3]) -> Option<&BlockEntityRef> {
    doc.block_entity_at(&rid(), BlockPosition::new(p[0], p[1], p[2]))
}

fn block_id(doc: &Document, p: [i64; 3]) -> Option<String> {
    let region = doc.region(&rid()).unwrap();
    region
        .block_index_at(BlockPosition::new(p[0], p[1], p[2]))
        .unwrap()
        .and_then(|i| region.palette().get(i))
        .map(|s| s.id().to_string())
}

fn assert_no_orphans(doc: &Document) {
    let orphans: Vec<_> = validate_document(doc)
        .into_iter()
        .filter(|d| d.code.contains("BLOCK_ENTITY"))
        .collect();
    assert!(orphans.is_empty(), "unexpected diagnostics: {orphans:?}");
}

fn yaw_of(e: &EntityRef) -> f64 {
    e.data.get("Rotation").unwrap().as_list().unwrap()[0]
        .as_f64()
        .unwrap()
}

#[test]
fn move_carries_chest_contents_and_entities_and_undo_restores() {
    let original = fixture();
    let mut ws = EditWorkspace::start(original.clone());
    // Select chest + armor stand cells (world y=1 row, x 12..12, z 12..13), move +3 on X.
    let cmd = MoveCommand::new(rid(), world_sel([12, 1, 12], [12, 1, 13]), [3, 0, 0]);
    commit(&mut ws, &cmd);

    let doc = ws.committed();
    assert!(be_at(doc, [2, 1, 2]).is_none(), "old position keeps no BE");
    let moved = be_at(doc, [5, 1, 2]).expect("BE moved with its chest");
    assert_eq!(moved.data, chest_items("Treasure"));
    assert_eq!(block_id(doc, [5, 1, 2]).as_deref(), Some("minecraft:chest"));
    let ents: Vec<_> = doc.entities().collect();
    assert_eq!(ents.len(), 1);
    assert_eq!(ents[0].position, [5.5, 1.0, 3.5]);
    assert!(ents[0].data.contains_key("UUID"), "a move keeps identity");
    assert_no_orphans(doc);

    ws.undo().unwrap();
    assert!(original.is_same_content(ws.committed()));
    ws.redo().unwrap();
    assert!(be_at(ws.committed(), [5, 1, 2]).is_some());
}

#[test]
fn delete_removes_block_entity_and_entities_and_undo_restores() {
    let original = fixture();
    let mut ws = EditWorkspace::start(original.clone());
    commit(
        &mut ws,
        &DeleteCommand::new(rid(), world_sel([12, 1, 12], [12, 1, 13])),
    );
    assert!(be_at(ws.committed(), [2, 1, 2]).is_none());
    assert_eq!(ws.committed().entities().count(), 0);
    assert_no_orphans(ws.committed());

    ws.undo().unwrap();
    assert!(original.is_same_content(ws.committed()));
    assert_eq!(
        be_at(ws.committed(), [2, 1, 2]).unwrap().data,
        chest_items("Treasure")
    );
}

#[test]
fn fill_or_replace_with_different_block_drops_block_entity_but_same_block_keeps_it() {
    let mut ws = EditWorkspace::start(fixture());
    commit(
        &mut ws,
        &FillCommand::new(
            rid(),
            world_sel([12, 1, 12], [12, 1, 12]),
            state("minecraft:chest"),
        ),
    );
    assert!(
        be_at(ws.committed(), [2, 1, 2]).is_some(),
        "same block keeps BE"
    );

    commit(
        &mut ws,
        &ReplaceCommand::new(
            rid(),
            world_sel([10, 0, 10], [17, 3, 17]),
            state("minecraft:chest"),
            state("minecraft:gold_block"),
        ),
    );
    assert!(be_at(ws.committed(), [2, 1, 2]).is_none());
    assert_no_orphans(ws.committed());
    ws.undo().unwrap();
    assert!(be_at(ws.committed(), [2, 1, 2]).is_some());
}

#[test]
fn rotate_moves_block_entity_and_turns_entities_and_four_turns_are_identity() {
    let original = fixture();
    let mut ws = EditWorkspace::start(original.clone());
    // 4x1x4 box covering local x 1..4, z 1..4 at y=1 (world 11..14).
    let sel = world_sel([11, 1, 11], [14, 1, 14]);
    commit(
        &mut ws,
        &RotateCommand::new(rid(), sel.clone(), RotationAngle::Deg90),
    );

    let doc = ws.committed();
    // Block offset (1,_,1) in a 4x4 box -> (4-1-1, 1) = (2, 1) -> local (3,1,2).
    let be = be_at(doc, [3, 1, 2]).expect("BE follows rotated chest");
    assert_eq!(be.data, chest_items("Treasure"));
    assert_eq!(block_id(doc, [3, 1, 2]).as_deref(), Some("minecraft:chest"));
    assert!(be_at(doc, [2, 1, 2]).is_none());
    let ent = doc.entities().next().unwrap();
    // Point offset (1.5, 2.5) -> (4 - 2.5, 1.5) = (1.5, 1.5) -> local (2.5, 1, 2.5).
    assert_eq!(ent.position, [2.5, 1.0, 2.5]);
    assert_eq!(yaw_of(ent), 270.0, "north-facing (180) turns east (270)");
    assert_no_orphans(doc);

    for _ in 0..3 {
        commit(
            &mut ws,
            &RotateCommand::new(rid(), sel.clone(), RotationAngle::Deg90),
        );
    }
    assert!(original.is_same_content(ws.committed()));
}

#[test]
fn mirror_twice_is_identity_for_entities_and_block_entities() {
    let original = fixture();
    let mut ws = EditWorkspace::start(original.clone());
    let sel = world_sel([11, 1, 11], [14, 1, 14]);
    commit(
        &mut ws,
        &MirrorCommand::new(rid(), sel.clone(), MirrorAxis::X),
    );
    let ent = ws.committed().entities().next().unwrap();
    assert_eq!(ent.position, [3.5, 1.0, 3.5]);
    assert_eq!(yaw_of(ent), 180.0, "north stays north under an X mirror");
    assert!(be_at(ws.committed(), [3, 1, 2]).is_some());
    commit(&mut ws, &MirrorCommand::new(rid(), sel, MirrorAxis::X));
    assert!(original.is_same_content(ws.committed()));
}

#[test]
fn copy_paste_duplicates_block_entity_data_and_gives_entities_fresh_identity() {
    let doc = fixture();
    let clip =
        Clipboard::from_document_selection(&doc, &rid(), &world_sel([12, 1, 12], [12, 1, 13]))
            .unwrap();
    assert_eq!(clip.block_entities.len(), 1);
    assert_eq!(clip.entities.len(), 1);

    let mut ws = EditWorkspace::start(doc);
    commit(
        &mut ws,
        &PasteCommand::new(rid(), Position::new(16, 1, 14), clip),
    );
    let doc = ws.committed();
    assert_eq!(be_at(doc, [2, 1, 2]).unwrap().data, chest_items("Treasure"));
    assert_eq!(be_at(doc, [6, 1, 4]).unwrap().data, chest_items("Treasure"));
    assert_eq!(doc.entities().count(), 2);
    let pasted = doc
        .entities()
        .find(|e| e.position == [6.5, 1.0, 5.5])
        .expect("pasted entity at target");
    assert!(!pasted.data.contains_key("UUID"), "pasted copies drop UUID");
    assert_no_orphans(doc);
}

#[test]
fn paste_outside_region_drops_block_entities_with_their_blocks() {
    let doc = fixture();
    let clip =
        Clipboard::from_document_selection(&doc, &rid(), &world_sel([12, 1, 12], [12, 1, 13]))
            .unwrap();
    let mut ws = EditWorkspace::start(doc);
    ws.preview_command(&PasteCommand::new(rid(), Position::new(100, 1, 100), clip))
        .unwrap();
    assert!(ws.pending_preview().unwrap().is_empty());
}
