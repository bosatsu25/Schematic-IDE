pub mod session;

pub use session::{
    BlockInspection, BlockInspectionRequest, CleanupRequest, Diagnostic, DiagnosticSeverity,
    DocumentInspection, DocumentMetadataInspection, DocumentSummary, Fixability, HistorySummary,
    PreviewDiff, PreviewSummary, RegionInspectionSummary, RegionMeshData, RegionSummary,
    ReplaceRequest, SelectionBounds, Session, SessionStatus,
};

#[cfg(target_arch = "wasm32")]
mod wasm_exports {
    use super::*;
    use std::cell::RefCell;

    thread_local! {
        static SESSION: RefCell<Session> = RefCell::new(Session::new());
    }

    #[no_mangle]
    pub extern "C" fn wasm_alloc(size: usize) -> *mut u8 {
        let mut buf = Vec::with_capacity(size);
        let ptr = buf.as_mut_ptr();
        std::mem::forget(buf);
        ptr
    }

    #[no_mangle]
    pub extern "C" fn wasm_dealloc(ptr: *mut u8, size: usize) {
        if !ptr.is_null() {
            unsafe {
                drop(Vec::from_raw_parts(ptr, size, size));
            }
        }
    }

    #[no_mangle]
    pub extern "C" fn wasm_get_response_ptr() -> *const u8 {
        SESSION.with(|s| s.borrow().response_buffer().as_ptr())
    }

    #[no_mangle]
    pub extern "C" fn wasm_get_response_len() -> usize {
        SESSION.with(|s| s.borrow().response_buffer().len())
    }

    #[no_mangle]
    pub extern "C" fn schematic_init() -> i32 {
        SESSION.with(|s| {
            *s.borrow_mut() = Session::new();
        });
        0
    }

    #[no_mangle]
    pub extern "C" fn schematic_load_litematic(ptr: *const u8, len: usize) -> i32 {
        let bytes = unsafe { std::slice::from_raw_parts(ptr, len) };
        SESSION.with(|s| {
            let mut session = s.borrow_mut();
            match session.load_litematic(bytes) {
                Ok(summary) => {
                    session.set_json_response(&summary);
                    0
                }
                Err(e) => {
                    session.set_error_response(&e);
                    -1
                }
            }
        })
    }

    #[no_mangle]
    pub extern "C" fn schematic_get_region_mesh(ptr: *const u8, len: usize) -> i32 {
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
        let region_name = match std::str::from_utf8(slice) {
            Ok(s) => s,
            Err(e) => {
                SESSION.with(|s| s.borrow_mut().set_error_response(&e.to_string()));
                return -1;
            }
        };
        SESSION.with(|s| {
            let mut session = s.borrow_mut();
            match session.get_region_mesh(region_name) {
                Ok(mesh) => {
                    session.set_json_response(&mesh);
                    0
                }
                Err(e) => {
                    session.set_error_response(&e);
                    -1
                }
            }
        })
    }

    #[no_mangle]
    pub extern "C" fn schematic_preview_replace(ptr: *const u8, len: usize) -> i32 {
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
        let req: ReplaceRequest = match serde_json::from_slice(slice) {
            Ok(r) => r,
            Err(e) => {
                SESSION.with(|s| s.borrow_mut().set_error_response(&e.to_string()));
                return -1;
            }
        };
        SESSION.with(|s| {
            let mut session = s.borrow_mut();
            match session.preview_replace(req) {
                Ok(res) => {
                    session.set_json_response(&res);
                    0
                }
                Err(e) => {
                    session.set_error_response(&e);
                    -1
                }
            }
        })
    }

    #[no_mangle]
    pub extern "C" fn schematic_preview_cleanup(ptr: *const u8, len: usize) -> i32 {
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
        let req: CleanupRequest = match serde_json::from_slice(slice) {
            Ok(r) => r,
            Err(e) => {
                SESSION.with(|s| s.borrow_mut().set_error_response(&e.to_string()));
                return -1;
            }
        };
        SESSION.with(|s| {
            let mut session = s.borrow_mut();
            match session.preview_cleanup(req) {
                Ok(res) => {
                    session.set_json_response(&res);
                    0
                }
                Err(e) => {
                    session.set_error_response(&e);
                    -1
                }
            }
        })
    }

    #[no_mangle]
    pub extern "C" fn schematic_commit_preview() -> i32 {
        SESSION.with(|s| {
            let mut session = s.borrow_mut();
            match session.commit_preview() {
                Ok(res) => {
                    session.set_json_response(&res);
                    0
                }
                Err(e) => {
                    session.set_error_response(&e);
                    -1
                }
            }
        })
    }

    #[no_mangle]
    pub extern "C" fn schematic_cancel_preview() -> i32 {
        SESSION.with(|s| {
            let mut session = s.borrow_mut();
            match session.cancel_preview() {
                Ok(res) => {
                    session.set_json_response(&res);
                    0
                }
                Err(e) => {
                    session.set_error_response(&e);
                    -1
                }
            }
        })
    }

    #[no_mangle]
    pub extern "C" fn schematic_undo() -> i32 {
        SESSION.with(|s| {
            let mut session = s.borrow_mut();
            match session.undo() {
                Ok(res) => {
                    session.set_json_response(&res);
                    0
                }
                Err(e) => {
                    session.set_error_response(&e);
                    -1
                }
            }
        })
    }

    #[no_mangle]
    pub extern "C" fn schematic_redo() -> i32 {
        SESSION.with(|s| {
            let mut session = s.borrow_mut();
            match session.redo() {
                Ok(res) => {
                    session.set_json_response(&res);
                    0
                }
                Err(e) => {
                    session.set_error_response(&e);
                    -1
                }
            }
        })
    }

    #[no_mangle]
    pub extern "C" fn schematic_export_litematic() -> i32 {
        SESSION.with(|s| {
            let mut session = s.borrow_mut();
            match session.export_litematic() {
                Ok(bytes) => {
                    session.set_bytes_response(bytes);
                    0
                }
                Err(e) => {
                    session.set_error_response(&e);
                    -1
                }
            }
        })
    }

    #[no_mangle]
    pub extern "C" fn schematic_get_status() -> i32 {
        SESSION.with(|s| {
            let mut session = s.borrow_mut();
            let status = session.get_status();
            session.set_json_response(&status);
            0
        })
    }

    #[no_mangle]
    pub extern "C" fn schematic_inspect_block(ptr: *const u8, len: usize) -> i32 {
        let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
        let req: BlockInspectionRequest = match serde_json::from_slice(slice) {
            Ok(r) => r,
            Err(e) => {
                SESSION.with(|s| s.borrow_mut().set_error_response(&e.to_string()));
                return -1;
            }
        };
        SESSION.with(|s| {
            let mut session = s.borrow_mut();
            match session.inspect_block(&req) {
                Ok(inspection) => {
                    session.set_json_response(&inspection);
                    0
                }
                Err(e) => {
                    session.set_error_response(&e);
                    -1
                }
            }
        })
    }

    #[no_mangle]
    pub extern "C" fn schematic_inspect_document() -> i32 {
        SESSION.with(|s| {
            let mut session = s.borrow_mut();
            match session.inspect_document() {
                Ok(inspection) => {
                    session.set_json_response(&inspection);
                    0
                }
                Err(e) => {
                    session.set_error_response(&e);
                    -1
                }
            }
        })
    }

    #[no_mangle]
    pub extern "C" fn schematic_validate_document() -> i32 {
        SESSION.with(|s| {
            let mut session = s.borrow_mut();
            match session.validate_document() {
                Ok(diagnostics) => {
                    session.set_json_response(&diagnostics);
                    0
                }
                Err(e) => {
                    session.set_error_response(&e);
                    -1
                }
            }
        })
    }
}
