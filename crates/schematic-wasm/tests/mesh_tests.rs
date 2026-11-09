//! Face-culled chunk meshes, dirty-chunk tracking and multi-box selections
//! (issues #24, #27, #28).

use schematic_core::{
    BlockPosition, BlockState, Document, DocumentMetadata, Position, Region, RegionId, Size,
};
use schematic_format::LitematicDocument;
use schematic_wasm::{FillRequest, SelectionBounds, Session};

fn load(blocks: &[([i64; 3], &str)], size: Size) -> Session {
    let mut doc = Document::new(DocumentMetadata {
        name: Some("Mesh".into()),
        ..Default::default()
    });
    let mut region = Region::new(RegionId::new("R"), Position::new(0, 0, 0), size);
    for (p, id) in blocks {
        let i = region
            .palette_mut()
            .intern(BlockState::new(*id, []).unwrap())
            .unwrap();
        region
            .set_block_index(BlockPosition::new(p[0], p[1], p[2]), Some(i))
            .unwrap();
    }
    doc.insert_region(region);
    let lit = LitematicDocument::from_document(doc, 3465);
    let mut session = Session::new();
    session
        .load_schematic(&lit.export(&lit.document).unwrap())
        .unwrap();
    session
}

/// Decodes the binary stream into (chunk -> faces as [x, y, z, (pal << 3) | dir]).
fn decode(bytes: &[u8]) -> Vec<([i32; 3], Vec<[i32; 4]>)> {
    let ints: Vec<i32> = bytes
        .chunks_exact(4)
        .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .collect();
    let mut out = Vec::new();
    let mut i = 1;
    for _ in 0..ints[0] {
        let chunk = [ints[i], ints[i + 1], ints[i + 2]];
        let n = ints[i + 3] as usize;
        i += 4;
        let faces = (0..n)
            .map(|k| {
                let v = ints[i + k] as u32;
                [
                    chunk[0] * 16 + (v & 15) as i32,
                    chunk[1] * 16 + ((v >> 4) & 15) as i32,
                    chunk[2] * 16 + ((v >> 8) & 15) as i32,
                    (((v >> 15) << 3) | ((v >> 12) & 7)) as i32,
                ]
            })
            .collect();
        i += n;
        out.push((chunk, faces));
    }
    assert_eq!(i, ints.len(), "stream fully consumed");
    out
}

fn face_count(session: &Session) -> usize {
    decode(&session.region_faces("R", None).unwrap())
        .iter()
        .map(|(_, f)| f.len())
        .sum()
}

#[test]
fn solid_cube_emits_only_surface_faces() {
    let mut blocks = Vec::new();
    for x in 0..3 {
        for y in 0..3 {
            for z in 0..3 {
                blocks.push(([x, y, z], "minecraft:stone"));
            }
        }
    }
    let session = load(&blocks, Size::new(3, 3, 3));
    // 27 cubes would be 162 faces; only the 6 × 9 outer faces are visible.
    assert_eq!(face_count(&session), 54);
}

#[test]
fn transparent_neighbours_do_not_hide_faces() {
    let session = load(
        &[
            ([0, 0, 0], "minecraft:stone"),
            ([1, 0, 0], "minecraft:glass"),
        ],
        Size::new(2, 1, 1),
    );
    // Stone keeps its +X face behind glass; glass loses its -X face to stone.
    assert_eq!(face_count(&session), 6 + 5);
}

#[test]
fn faces_cross_chunk_borders_and_chunks_can_be_requested_individually() {
    let session = load(
        &[
            ([15, 0, 0], "minecraft:stone"),
            ([16, 0, 0], "minecraft:stone"),
        ],
        Size::new(32, 1, 1),
    );
    let chunks = decode(&session.region_faces("R", None).unwrap());
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks.iter().map(|(_, f)| f.len()).sum::<usize>(), 10);
    let only = decode(
        &session
            .region_faces("R", Some(&[[1, 0, 0], [5, 5, 5]]))
            .unwrap(),
    );
    assert_eq!(only.len(), 2, "requested chunks are always returned");
    assert_eq!(only[0].1.len(), 5);
    assert!(only[1].1.is_empty());
}

#[test]
fn preview_is_rendered_and_marks_chunks_dirty_until_taken() {
    let mut session = load(&[([0, 0, 0], "minecraft:stone")], Size::new(40, 1, 1));
    session.take_dirty_chunks();
    let preview = session
        .preview_fill(FillRequest {
            region_id: "R".into(),
            selection: SelectionBounds {
                min: [35, 0, 0],
                max: [35, 0, 0],
                ..Default::default()
            },
            block: "minecraft:gold_block".into(),
        })
        .unwrap();
    assert!(preview.can_commit);

    let view = session.region_view("R").unwrap();
    assert!(view.palette.contains(&"minecraft:gold_block".to_string()));
    assert_eq!(view.preview_positions, vec![[35, 0, 0]]);
    assert_eq!(face_count(&session), 12, "preview block is drawn");

    let dirty = session.take_dirty_chunks();
    assert!(dirty["R"].contains(&[2, 0, 0]));
    assert!(session.take_dirty_chunks().is_empty(), "taking clears");

    session.cancel_preview().unwrap();
    assert!(session.take_dirty_chunks()["R"].contains(&[2, 0, 0]));
    assert_eq!(face_count(&session), 6);
}

#[test]
fn multi_box_selection_edits_the_union() {
    let mut session = load(&[], Size::new(10, 1, 1));
    session
        .preview_fill(FillRequest {
            region_id: "R".into(),
            selection: SelectionBounds {
                min: [0, 0, 0],
                max: [1, 0, 0],
                extra_boxes: vec![[[5, 0, 0], [5, 0, 0]], [[8, 0, 0], [9, 0, 0]]],
            },
            block: "minecraft:stone".into(),
        })
        .unwrap();
    session.commit_preview().unwrap();
    let chunks = decode(&session.region_faces("R", None).unwrap());
    let mut xs: Vec<i32> = chunks
        .iter()
        .flat_map(|(_, f)| f.iter().map(|f| f[0]))
        .collect();
    xs.sort();
    xs.dedup();
    assert_eq!(xs, vec![0, 1, 5, 8, 9]);
}

#[test]
fn faces_are_packed_into_four_bytes_each() {
    let session = load(&[([17, 33, 2], "minecraft:stone")], Size::new(20, 40, 4));
    let bytes = session.region_faces("R", None).unwrap();
    // [count] + [cx, cy, cz, n] + 6 packed faces.
    assert_eq!(bytes.len(), (1 + 4 + 6) * 4);
    let chunks = decode(&bytes);
    assert_eq!(chunks[0].0, [1, 2, 0]);
    assert!(chunks[0].1.iter().all(|f| f[..3] == [17, 33, 2]));
}
