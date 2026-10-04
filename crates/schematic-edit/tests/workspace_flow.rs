use schematic_core::{
    BlockPosition, BlockState, Bounds, Document, DocumentMetadata, Position, Region, RegionId,
    Selection, SelectionBox, Size,
};
use schematic_edit::{EditCommand, EditWorkspace, FillCommand, ReplaceCommand};

fn state(id: &str) -> BlockState {
    BlockState::new(id, []).unwrap()
}

fn test_document(size: Size) -> Document {
    let mut doc = Document::new(DocumentMetadata::default());
    let region = Region::new(RegionId::new("main"), Position::new(0, 0, 0), size);
    doc.insert_region(region);
    doc
}

#[test]
fn preview_does_not_mutate_committed_or_source_state() {
    let mut doc = test_document(Size::new(2, 1, 1));
    let region_id = RegionId::new("main");
    let state_a = state("minecraft:stone");
    let state_b = state("minecraft:quartz_block");

    // Initialize block 0,0,0 with state_a
    {
        let region = doc.region_mut(&region_id).unwrap();
        let idx_a = region.palette_mut().intern(state_a.clone()).unwrap();
        region
            .set_block_index(BlockPosition::new(0, 0, 0), Some(idx_a))
            .unwrap();
        region
            .set_block_index(BlockPosition::new(1, 0, 0), Some(idx_a))
            .unwrap();
    }

    let mut workspace = EditWorkspace::start(doc);
    let sel = Selection::from_corners(Position::new(0, 0, 0), Position::new(0, 0, 0));
    let cmd = ReplaceCommand::new(region_id.clone(), sel, state_a.clone(), state_b.clone());

    workspace.preview_command(&cmd).unwrap();
    assert!(workspace.has_preview());
    assert!(!workspace.is_dirty());

    // Preview block should be B, committed and source should still be A
    let preview_idx_0 = workspace
        .preview_block_at(&region_id, BlockPosition::new(0, 0, 0))
        .unwrap();
    assert!(preview_idx_0.is_some());

    let committed_idx_0 = workspace
        .committed()
        .region(&region_id)
        .unwrap()
        .block_index_at(BlockPosition::new(0, 0, 0))
        .unwrap();
    let source_idx_0 = workspace
        .source()
        .region(&region_id)
        .unwrap()
        .block_index_at(BlockPosition::new(0, 0, 0))
        .unwrap();

    let committed_palette = workspace.committed().region(&region_id).unwrap().palette();
    assert_eq!(
        committed_palette.get(committed_idx_0.unwrap()),
        Some(&state_a)
    );
    assert_eq!(committed_palette.get(source_idx_0.unwrap()), Some(&state_a));

    // After preview, we commit
    workspace.commit_preview().unwrap();
    assert!(!workspace.has_preview());
    assert!(workspace.is_dirty());
    assert!(workspace.can_undo());
    assert!(!workspace.can_redo());

    let new_committed_idx_0 = workspace
        .committed()
        .region(&region_id)
        .unwrap()
        .block_index_at(BlockPosition::new(0, 0, 0))
        .unwrap();
    let new_palette = workspace.committed().region(&region_id).unwrap().palette();
    assert_eq!(
        new_palette.get(new_committed_idx_0.unwrap()),
        Some(&state_b)
    );

    // Cancel preview check
    let sel2 = Selection::from_corners(Position::new(1, 0, 0), Position::new(1, 0, 0));
    let cmd2 = ReplaceCommand::new(region_id.clone(), sel2, state_a.clone(), state_b.clone());
    workspace.preview_command(&cmd2).unwrap();
    assert!(workspace.has_preview());
    workspace.cancel_preview();
    assert!(!workspace.has_preview());

    // Undo and redo
    assert!(workspace.undo().unwrap());
    assert!(!workspace.is_dirty());
    let undone_idx_0 = workspace
        .committed()
        .region(&region_id)
        .unwrap()
        .block_index_at(BlockPosition::new(0, 0, 0))
        .unwrap();
    assert_eq!(
        workspace
            .committed()
            .region(&region_id)
            .unwrap()
            .palette()
            .get(undone_idx_0.unwrap()),
        Some(&state_a)
    );

    assert!(workspace.redo().unwrap());
    assert!(workspace.is_dirty());
    let redone_idx_0 = workspace
        .committed()
        .region(&region_id)
        .unwrap()
        .block_index_at(BlockPosition::new(0, 0, 0))
        .unwrap();
    assert_eq!(
        workspace
            .committed()
            .region(&region_id)
            .unwrap()
            .palette()
            .get(redone_idx_0.unwrap()),
        Some(&state_b)
    );
}

#[test]
fn lifo_history_across_multiple_commits() {
    let doc = test_document(Size::new(2, 1, 1));
    let region_id = RegionId::new("main");
    let state_b = state("minecraft:quartz_block");
    let state_c = state("minecraft:dirt");

    let mut workspace = EditWorkspace::start(doc);
    let sel0 = Selection::from_corners(Position::new(0, 0, 0), Position::new(0, 0, 0));
    let fill0 = FillCommand::new(region_id.clone(), sel0, state_b.clone());
    workspace.preview_command(&fill0).unwrap();
    workspace.commit_preview().unwrap();

    let sel1 = Selection::from_corners(Position::new(1, 0, 0), Position::new(1, 0, 0));
    let fill1 = FillCommand::new(region_id.clone(), sel1, state_c.clone());
    workspace.preview_command(&fill1).unwrap();
    workspace.commit_preview().unwrap();

    // First undo reverts block 1
    assert!(workspace.undo().unwrap());
    let block1 = workspace
        .committed()
        .region(&region_id)
        .unwrap()
        .block_index_at(BlockPosition::new(1, 0, 0))
        .unwrap();
    assert_eq!(block1, None);

    // Second undo reverts block 0
    assert!(workspace.undo().unwrap());
    let block0 = workspace
        .committed()
        .region(&region_id)
        .unwrap()
        .block_index_at(BlockPosition::new(0, 0, 0))
        .unwrap();
    assert_eq!(block0, None);
    assert!(!workspace.can_undo());

    // Redo restores block 0, then block 1
    assert!(workspace.redo().unwrap());
    let block0_restored = workspace
        .committed()
        .region(&region_id)
        .unwrap()
        .block_index_at(BlockPosition::new(0, 0, 0))
        .unwrap();
    assert!(block0_restored.is_some());

    assert!(workspace.redo().unwrap());
    let block1_restored = workspace
        .committed()
        .region(&region_id)
        .unwrap()
        .block_index_at(BlockPosition::new(1, 0, 0))
        .unwrap();
    assert!(block1_restored.is_some());
    assert!(!workspace.can_redo());
}

#[test]
fn multi_box_selection_bounded_replacement_and_deduplication() {
    let mut doc = test_document(Size::new(5, 1, 1));
    let region_id = RegionId::new("main");
    let state_a = state("minecraft:stone");
    let state_b = state("minecraft:gold_block");

    // Fill positions 0, 1, 2, 3 with stone
    {
        let region = doc.region_mut(&region_id).unwrap();
        let idx = region.palette_mut().intern(state_a.clone()).unwrap();
        for x in 0..4 {
            region
                .set_block_index(BlockPosition::new(x, 0, 0), Some(idx))
                .unwrap();
        }
    }

    // Overlapping boxes: [0, 3) and [1, 4)
    let box1 = SelectionBox::new(Bounds::new(Position::new(0, 0, 0), [3, 1, 1]));
    let box2 = SelectionBox::new(Bounds::new(Position::new(1, 0, 0), [4, 1, 1]));
    let selection = Selection::from_boxes([box1, box2]);

    let cmd = ReplaceCommand::new(
        region_id.clone(),
        selection,
        state_a.clone(),
        state_b.clone(),
    );
    let patch = cmd.create_patch(&doc).unwrap();

    // Changed block count must be exactly 4 (positions 0, 1, 2, 3), not duplicated!
    assert_eq!(patch.changed_block_count(), 4);
}
