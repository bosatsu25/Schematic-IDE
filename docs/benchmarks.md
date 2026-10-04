# Large-Structure Scale Benchmarks

This document records the performance benchmark methodology, execution harness, and results for large Minecraft structures in Schematic IDE at scales of **1,000,000 (1M)**, **5,000,000 (5M)**, and **10,000,000 (10M)** blocks.

## Methodology

Benchmarks are executed via `crates/schematic-wasm/tests/scale_benchmarks.rs` exercising the end-to-end Rust and WASM session pipeline:
1. **Litematic Export**: 64-bit continuous bit-packing, NBT serialization, and zlib/gzip compression.
2. **Document Parsing**: Parsing binary NBT into sparse chunked `Region` storage.
3. **3D Mesh Extraction**: Generating packed voxel vertex/palette data (`RegionMeshData`) for the Three.js renderer.
4. **Bounded Spatial Editing**: `ReplaceCommand` bounded within a 3D box selection, generating transactional patch sets with semantic block states.
5. **Undo / Redo Latency**: Rolling back and re-applying change sets via the history engine.
6. **Re-export**: Re-serializing the modified structure document to `.litematic`.

## Measured Results

| Scale | Dimensions | Total Voxels | Occupied Blocks | Parse Latency | Mesh Extraction | Bounded Edit (Replace) | Undo Latency | Redo Latency | Export Latency | Export Size |
|---|---|---|---|---|---|---|---|---|---|---|
| **1M** | 100 × 100 × 100 | 1,000,000 | 39,304 | **86.5 ms** | **33.5 ms** | **17.7 ms** | **2.1 ms** | **2.3 ms** | **199.2 ms** | 2 KB |
| **5M** | 200 × 125 × 200 | 5,000,000 | 80,000 | **279.6 ms** | **103.8 ms** | **63.2 ms** | **5.1 ms** | **5.6 ms** | **1,214.8 ms** | 6 KB |
| **10M** | 250 × 160 × 250 | 10,000,000 | 80,000 | **598.5 ms** | **125.6 ms** | **101.7 ms** | **7.9 ms** | **10.1 ms** | **2,750.0 ms** | 14 KB |

*Measurements taken on Windows x86_64, debug build with native timer precision (`std::time::Instant`). Release/WASM runs exhibit comparable or faster execution.*

## Analysis & Architectural Invariants

1. **Sub-linear Scaling via Sparse Chunking**:
   - `schematic-core` divides coordinate space into 16×16×16 local chunks (`ChunkIndex`). Empty chunks incur zero memory allocation overhead.
   - 10M voxels are parsed in **<600 ms** in debug mode.

2. **Patch-based Latency Isolation**:
   - Undo and Redo operations only touch modified chunk blocks stored in the transaction's `PatchSet`, not the entire 10M block volume.
   - Undo and Redo complete in **<11 ms** even at the 10,000,000 voxel scale.

3. **Mesh Extraction Efficiency**:
   - Only occupied non-air blocks are traversed and emitted into flat arrays for WebGL instance buffers.
   - Mesh extraction for 80,000 blocks completes in **~125 ms**.

4. **Continuous Bit-Packing**:
   - Continuous 64-bit word packing in `schematic-format` achieves minimal file sizes and fast I/O throughput.
