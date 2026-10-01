# ADR 0004: Chunk-oriented storage model

## Status

Accepted

## Context

A full-document vector-of-blocks approach is too costly for large structures and makes partial updates difficult.

## Decision

The document model will be organized into chunks with palette-aware indexing, dirty-region tracking, and partial mesh rebuild support.

## Consequences

- Better edit locality
- Lower memory churn during history updates
- Easier implementation of partial validation and rendering
