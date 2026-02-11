# Pebble TODO

## Design / Feature Gaps

### spill-ring-std: WorkerPool parallel serialization
When `spill-ring-std` is enabled, serialization should be parallelized across
a WorkerPool. Each worker owns its own SpillRing. Main thread dispatches
unserialized checkpoints, workers serialize and push bytes into their rings,
barrier sync, then sequential drain to storage. Requires `T: Send + 'static`,
`Ser: Send + Clone + 'static`. See architecture.md for full design.

### PebbleGame is orphaned
`game.rs` is a standalone Red-Blue Pebble Game simulator. It is tested but
never used by `PebbleManager`. Could be wired in as an optional invariant
checker under `#[cfg(debug_assertions)]`.
