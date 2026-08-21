# ADR-0003: Keep exact, adaptive, surrogate, and cluster modes distinct

## Status

Accepted

## Context

The same words—“runs locally”—can describe an exact but very slow streamed model, a
faster precision-changing engine, a different small model, or a pool of machines. Those
outcomes are not interchangeable.

## Decision

The hardware plan exposes four separate modes and records whether each preserves the
official numerical path, is runnable now, and meets an observed interactive bandwidth
threshold. The product never automatically crosses from exact K3 to a different model.

## Consequences

- Users see the real tradeoff before downloading 1.56 TB.
- Exact and adaptive engines can improve independently without misleading labels.
- A small-model manager and K3 distributed worker protocol remain explicit future work,
  not hidden incomplete features in this release.
