# Hot-Reload Architecture Design (v0.3.0)

## Goals

1. **Zero-downtime updates**: Reload rules and policies without stopping the engine
2. **Lock-free reads**: Ongoing authorization requests continue without blocking
3. **Atomic transitions**: All or nothing - no partial state updates
4. **File watching**: Automatic reload when .rune files change
5. **Safe memory reclamation**: Old rule sets cleaned up after all readers finish

## Current Architecture

### FactStore (Already RCU-Ready ✅)
- Uses `crossbeam::epoch` for lock-free memory reclamation
- `Atomic<Arc<Vec<Fact>>>` for all_facts
- `DashMap<Arc<str>, Arc<Vec<Fact>>>` for predicate index
- Version tracking with `AtomicU64`

### DatalogEngine (Needs Improvement)
- Currently: `Arc<Vec<Rule>>` wrapped in `RwLock` at RUNEEngine level
- Problem: RwLock blocks readers during write
- Solution: Use `Arc<ArcSwap<DatalogEngine>>` for truly lock-free reads

### PolicySet (Needs Improvement)
- Currently: Wrapped in `RwLock`
- Same solution: Use `Arc<ArcSwap<PolicySet>>`

## Proposed RCU Design

### Phase 1: Atomic Engine Swap

Replace:
```rust
pub struct RUNEEngine {
    datalog: Arc<RwLock<DatalogEngine>>,  // ❌ Blocks reads during write
    policies: Arc<RwLock<PolicySet>>,     // ❌ Blocks reads during write
    ...
}
```

With:
```rust
use arc_swap::ArcSwap;

pub struct RUNEEngine {
    datalog: Arc<ArcSwap<DatalogEngine>>,  // ✅ Lock-free reads
    policies: Arc<ArcSwap<PolicySet>>,     // ✅ Lock-free reads
    ...
}
```

**Reading** (lock-free, sub-nanosecond):
```rust
let engine_guard = self.datalog.load();
let result = engine_guard.evaluate(request, &self.facts)?;
```

**Writing** (atomic swap):
```rust
let new_engine = DatalogEngine::new(new_rules, self.facts.clone());
self.datalog.store(Arc::new(new_engine));
// Old engine is reference-counted, cleaned up when last reader drops
```

### Phase 2: File Watching

Use `notify` crate to watch .rune files:

```rust
use notify::{Watcher, RecursiveMode, watcher};

pub struct FileWatcher {
    watcher: RecommendedWatcher,
    reload_tx: mpsc::Sender<PathBuf>,
}

impl FileWatcher {
    pub fn watch_path(&mut self, path: impl AsRef<Path>) -> Result<()> {
        self.watcher.watch(path.as_ref(), RecursiveMode::NonRecursive)?;
        Ok(())
    }
}
```

### Phase 3: Reload Coordinator

Orchestrates the reload process:

```rust
pub struct ReloadCoordinator {
    engine: Arc<RUNEEngine>,
    file_paths: Vec<PathBuf>,
    reload_rx: mpsc::Receiver<PathBuf>,
}

impl ReloadCoordinator {
    pub async fn run(&mut self) {
        while let Some(changed_path) = self.reload_rx.recv().await {
            if let Err(e) = self.reload_config(&changed_path).await {
                eprintln!("Failed to reload config: {}", e);
                continue;
            }
            println!("Successfully reloaded: {:?}", changed_path);
        }
    }

    async fn reload_config(&self, path: &Path) -> Result<()> {
        // 1. Read and parse new config
        let content = tokio::fs::read_to_string(path).await?;
        let config = parse_rune_file(&content)?;

        // 2. Create new DatalogEngine
        let new_datalog = DatalogEngine::new(config.rules, self.engine.facts.clone());

        // 3. Create new PolicySet
        let new_policies = PolicySet::from_policies(config.policies)?;

        // 4. Atomic swap (lock-free!)
        self.engine.datalog.store(Arc::new(new_datalog));
        self.engine.policies.store(Arc::new(new_policies));

        // 5. Invalidate cache (old decisions may be stale)
        self.engine.cache.clear();

        // Old engines are automatically cleaned up when last reader drops
        Ok(())
    }
}
```

## Implementation Plan

### Step 1: Add Dependencies
```toml
[dependencies]
arc-swap = "1.7"
notify = "6.1"
tokio = { version = "1.35", features = ["fs", "sync"] }
```

### Step 2: Update DatalogEngine
- Remove `update_rules(&mut self)` (unsafe for concurrent access)
- Keep rules as `Arc<Vec<Rule>>` for zero-copy reads
- Engine itself becomes immutable (create new engine for updates)

### Step 3: Update RUNEEngine
- Replace `RwLock<DatalogEngine>` with `ArcSwap<DatalogEngine>`
- Replace `RwLock<PolicySet>` with `ArcSwap<PolicySet>`
- Update `authorize()` to use `load()` instead of `read()`

### Step 4: Implement FileWatcher
- New module: `rune-core/src/watcher.rs`
- Watch .rune files for changes
- Send notifications to reload coordinator

### Step 5: Implement ReloadCoordinator
- New module: `rune-core/src/reload.rs`
- Receive file change notifications
- Parse new config
- Atomically swap engines
- Handle errors gracefully (rollback on parse failure)

### Step 6: Add Tests
- Test atomic swap under load
- Test file watching
- Test reload with concurrent requests
- Test rollback on invalid config

## Performance Impact

### Before (RwLock)
- Read latency: ~50ns (uncontended) to 1μs+ (contended)
- Write blocks ALL readers
- Cache line bouncing on lock acquisition

### After (ArcSwap)
- Read latency: ~5-10ns (atomic load)
- Writes never block readers
- Zero cache line contention

**Expected improvement**: 5-10x faster reads during reload, zero downtime.

## Safety Guarantees

1. **Memory safety**: Arc reference counting ensures no use-after-free
2. **Atomic transitions**: ArcSwap guarantees readers see complete state
3. **No partial updates**: Parse failure = no swap, engine unchanged
4. **Graceful degradation**: Invalid config logged, old config retained

## Testing Strategy

### Unit Tests
- `test_atomic_engine_swap()`
- `test_concurrent_read_write()`
- `test_file_watcher_notifications()`

### Integration Tests
- `test_hot_reload_under_load()`
- `test_invalid_config_rollback()`
- `test_zero_downtime_reload()`

### Benchmark
- Measure latency during reload
- Confirm zero request failures
- Verify memory reclamation

## Dependencies

- **arc-swap**: Lock-free Arc swapping
- **notify**: Cross-platform file watching
- **tokio**: Async runtime for reload coordinator

## Rollout Plan

1. **v0.3.0-alpha**: Basic ArcSwap implementation
2. **v0.3.0-beta**: Add file watching
3. **v0.3.0**: Full hot-reload with tests

## Open Questions

1. **Reload debouncing**: Wait for file writes to settle before reloading?
2. **Partial updates**: Should we allow updating just rules OR just policies?
3. **Rollback strategy**: Keep N previous configs for instant rollback?
4. **Metrics**: Track reload count, success rate, rollback count?

## References

- [arc-swap documentation](https://docs.rs/arc-swap/)
- [notify documentation](https://docs.rs/notify/)
- [RCU in Linux kernel](https://www.kernel.org/doc/Documentation/RCU/whatisRCU.txt)
- [crossbeam epoch-based reclamation](https://docs.rs/crossbeam-epoch/)
