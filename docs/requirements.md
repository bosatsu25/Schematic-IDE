# Schematic IDE Requirements

## Product intent

Schematic IDE is a local-first browser-based IDE for Minecraft structure files. It is designed to help users open, inspect, edit, validate, diff, repair, and export `.litematic`, `.schem`, and Java Structure NBT files without requiring a mandatory backend or login.

## Clean-room principle

This repository intentionally avoids copying another implementation's source code, class structure, component organization, APIs, or naming. This project is designed from first principles around the product goals in this repository and the constraints of large Minecraft structure files.

## Core user workflows

- Open structure files from local disk or drag-and-drop imports
- View a structure in 3D in the browser
- Select regions / blocks / entities / block entities
- Fill, replace, delete, and transform sections
- Inspect metadata, block state, and NBT details
- Validate and repair correctness issues
- Compare different versions or regions via diff tools
- Export normalized or converted files

## v1.0 Definition of Done

- Web/PWA app launches locally in browser
- Supports `.litematic`, `.schem`, and Java Structure NBT
- Enables open, view, select, fill, replace, delete, copy, paste, move, rotate, mirror, undo, redo, inspect, analyze, validate, diff, and export
- Keeps structure data local-first
- Includes round-trip and property tests
- Includes E2E coverage for a practical editing workflow
- Includes large-structure performance tests
- CI stays green
- Architecture and README remain aligned with implementation
- No material silent data-loss issues are introduced

## Non-goals for Phase 0

The initial baseline intentionally focuses on repository structure, architecture documentation, validation strategy, and a clean workspace foundation. Functional feature implementation begins in subsequent phases.
