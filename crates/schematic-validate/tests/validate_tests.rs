use schematic_core::{
    BlockEntityRef, BlockPosition, BlockState, Document, DocumentMetadata, Position, Region,
    RegionId, Size,
};
use schematic_validate::{validate_document, DiagnosticSeverity};

#[test]
fn test_valid_document_has_no_errors() {
    let mut doc = Document::new(DocumentMetadata {
        name: Some("Test Structure".to_string()),
        author: Some("Tester".to_string()),
        description: Some("A test".to_string()),
        created_at_unix_ms: None,
        modified_at_unix_ms: None,
    });

    let mut region = Region::new(
        RegionId::new("Main"),
        Position::new(0, 0, 0),
        Size::new(16, 16, 16),
    );
    let stone = BlockState::new("minecraft:stone", std::iter::empty()).unwrap();
    let pal_idx = region.palette_mut().intern(stone).unwrap();
    region
        .set_block_index(BlockPosition::new(0, 0, 0), Some(pal_idx))
        .unwrap();

    doc.insert_region(region);

    let diagnostics = validate_document(&doc);
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == DiagnosticSeverity::Error)
        .collect();
    assert!(errors.is_empty(), "Expected 0 errors, got {:?}", errors);
}

#[test]
fn test_detects_empty_document_name() {
    let doc = Document::new(DocumentMetadata {
        name: Some("".to_string()),
        author: Some("Tester".to_string()),
        description: Some("A test".to_string()),
        created_at_unix_ms: None,
        modified_at_unix_ms: None,
    });

    let diagnostics = validate_document(&doc);
    let warning = diagnostics.iter().find(|d| d.code == "WARN_EMPTY_DOC_NAME");
    assert!(
        warning.is_some(),
        "Expected warning for empty document name"
    );
}

#[test]
fn test_detects_empty_palette() {
    let mut doc = Document::new(DocumentMetadata {
        name: Some("Test".to_string()),
        author: Some("Tester".to_string()),
        description: Some("A test".to_string()),
        created_at_unix_ms: None,
        modified_at_unix_ms: None,
    });

    let region = Region::new(
        RegionId::new("EmptyPaletteRegion"),
        Position::new(0, 0, 0),
        Size::new(5, 5, 5),
    );
    doc.insert_region(region);

    let diagnostics = validate_document(&doc);
    let err = diagnostics.iter().find(|d| d.code == "ERR_EMPTY_PALETTE");
    assert!(err.is_some(), "Expected error for empty palette");
    assert_eq!(err.unwrap().severity, DiagnosticSeverity::Error);
}

#[test]
fn test_detects_block_entity_outside_regions() {
    let mut doc = Document::new(DocumentMetadata {
        name: Some("Test".to_string()),
        author: Some("Tester".to_string()),
        description: Some("A test".to_string()),
        created_at_unix_ms: None,
        modified_at_unix_ms: None,
    });

    let mut region = Region::new(
        RegionId::new("Main"),
        Position::new(0, 0, 0),
        Size::new(10, 10, 10),
    );
    region
        .palette_mut()
        .intern(BlockState::new("minecraft:chest", std::iter::empty()).unwrap())
        .unwrap();
    doc.insert_region(region);

    // Block entity at (100, 100, 100) is far outside
    doc.add_block_entity(BlockEntityRef::new(
        "minecraft:chest",
        Position::new(100, 100, 100),
    ));

    let diagnostics = validate_document(&doc);
    let err = diagnostics
        .iter()
        .find(|d| d.code == "WARN_BLOCK_ENTITY_OUTSIDE_REGIONS");
    assert!(
        err.is_some(),
        "Expected warning for block entity outside region"
    );
    assert_eq!(err.unwrap().position, Some([100, 100, 100]));
}

#[test]
fn test_detects_suspicious_block_identifier() {
    let mut doc = Document::new(DocumentMetadata {
        name: Some("Test".to_string()),
        author: Some("Tester".to_string()),
        description: Some("A test".to_string()),
        created_at_unix_ms: None,
        modified_at_unix_ms: None,
    });

    let mut region = Region::new(
        RegionId::new("Main"),
        Position::new(0, 0, 0),
        Size::new(5, 5, 5),
    );
    // Missing namespace
    region
        .palette_mut()
        .intern(BlockState::new("stone_no_namespace", std::iter::empty()).unwrap())
        .unwrap();
    doc.insert_region(region);

    let diagnostics = validate_document(&doc);
    let warn = diagnostics
        .iter()
        .find(|d| d.code == "WARN_SUSPICIOUS_BLOCK_ID");
    assert!(warn.is_some(), "Expected warning for suspicious block ID");
}
