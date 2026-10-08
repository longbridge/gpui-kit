## Storage Engine Evaluation for the Telemetry Pipeline

Key takeaway: the embedded candidates cover about 80% of our query patterns out of the box. The columnar engine takes a near-rewrite hit on point lookups, but the hot paths (ingest and range scans) never touch that code, and batching can partially offset the lookup cost.

---

### 1. Candidate overview

Based on the v2025 benchmark suite and public documentation:

| Engine | Data model | Written in | Embedded |
|--------|------------|------------|----------|
| **RocksJar** (LSM tree) | ~69% key-value | C++ | **Yes** |
| **ColumnForge** (columnar) | ~15-18% analytical | Rust | **Yes** (static lib) |
| **TinyBtree** | ~5.2% key-value, **single writer** | Rust | **Yes** |
| **PagedHeap** | ~3.1% document | Go | **No** (sidecar process) |
| **Others** (remote OLAP, time-series) | ~3-5% | mixed | **No** |

**Directly affected paths:**

- **Ingest workers**: the pipeline holds a 63-shard write fan-out with one writer per shard [1]. Pre-migration throughput was about 650k rows/s; in the March 2026 load test the columnar prototype dropped roughly 80% on unbatched writes [2][3]
- **Dashboard queries**: an indirect dependency through two aggregation services, together serving about 29k queries/day in 2023 [1]. Point lookups can partially bypass the columnar store via the row cache, but that cache was also invalidated too aggressively in the May 2026 test [4]

**Unaffected core paths:** range scans (69%) plus the downsampling jobs (25% of load, now above 900k rows/s) [5] and the retention sweeps all run against immutable segments and never touch the lookup path.

---

### 2. Operational risk assessment

| Risk dimension | Severity | Details |
|----------------|----------|---------|
| **Unbatched writes stall** | **Severe** | Ingest workers were forced to cut throughput by 80% under the columnar prototype; back-pressure kicked in once the WAL saturated [6][7]. Single-row upserts through the compatibility shim effectively cannot be sustained |
| **Row cache constrained** | **Moderate** | The compatibility shim is capacity-limited (~1.5M entries), and eviction churn spiked after the cache-key change |
| **Cross-region replication lag** | **Moderate** | Replication fan-out costs jumped 50%, and p99 catch-up time surged 584% in the failover drill [8]. Snapshot shipping costs rise materially for the two largest regions |
| **Downsampling jobs** | **None** | Segment merges run against local immutable files and involve no cross-store traffic |

**Key fact**: the range-scan and downsampling paths — the pipeline's most important growth surface — never touch the lookup shim. Segment readers stream directly from immutable files onto the aggregation tier [9][10].

---

### 3. Benchmark summary

Using the v2025 suite as the baseline [11]:

| Metric | Result | Notes |
|--------|--------|-------|
| Sustained ingest | 556k rows/s | -5.3% vs current |
| Range scan p50 | 4.8ms (86.1% of SLO) | Core dashboard driver |
| Compaction debt | 43.3GB steady state | ~78% below ceiling |
| Point lookup p99 | 17.1ms | |

**Scenario estimate (pipeline level):**

- **Direct throughput loss**: shim-routed upserts (roughly 40-60k rows/s of normal load) plus cache misses (~29k queries/day) ≈ 3-5% of total traffic
- **Batching offset**: the March test briefly pushed batched ingest above 120-130% of the current baseline [8]. With segment write amplification holding near 1.28-1.30, per-shard headroom on the scan and downsampling paths expands sharply
- **Net effect**: a 3-5% loss on lookups versus a 20-30% gain on scans. If dashboards keep their current query mix, the scan-side gains likely cover or exceed the lookup regression

---

### 4. Key risks and buffers

**Risks:**

1. Long compaction pauses may degrade the read amplification budget; some shards might never return to pre-migration latencies [12]
2. Restoring full dual-write parity is expected to take 6+ weeks [12], during which rollback drills contribute zero coverage
3. Roughly 33-45% of dashboard traffic transits the lookup shim [13][8]; hot tenants could be redirected to prioritize interactive queries, compressing batch windows

**Buffers:**

1. Low concentration risk — 69% of traffic is range scans and independent of the lookup path
2. The downsampling tier is still ramping (the segment-cache project adds 250k rows/s in 2026) [5], which can backfill the ingest shortfall
3. Batching improves the economics of the highest-volume shards
4. Retained WAL of ~1.39bn rows (about 120 days of replay) [14] keeps near-term recovery unaffected

---

### 5. Summary

Among the three finalists, the columnar engine has **the lowest direct exposure to the lookup regression** — its core traffic comes from range scans (69%) and immutable segment reads rather than point lookups. Directly impaired paths total only about 3-5% of traffic. The bigger risks are indirect: compaction pressure from the write path, and long-tail uncertainty over restoring dual-write parity. On the latency dimension, the scan-side tailwind most likely covers the lookup-side loss.

### 6. Column fitting

Short columns keep their content on one line while the long one wraps. The
same table in Chinese and in English:

| 平台（架构） | 冷启动（毫秒） | 内存占用（MB） | 说明 |
|--------------|---------------:|---------------:|------|
| macOS（Apple Silicon） | 182 | 96.4 | 首帧包含字体加载；开启 Metal HUD 后约增加 8%（实测） |
| Windows（x86_64） | 241 | 118.0 | DirectWrite 字形缓存预热之前略慢（首次启动） |
| Linux（Wayland） | 205 | 102.7 | 与 X11 相差不到 5%（同一台机器） |
| Linux（X11） | 214 | 104.1 | 在同一台机器上测得（冷缓存） |

| Platform (arch) | Cold start (ms) | Memory (MB) | Notes |
|-----------------|----------------:|------------:|-------|
| macOS (Apple Silicon) | 182 | 96.4 | The first frame includes font loading; Metal HUD adds about 8% (measured) |
| Windows (x86_64) | 241 | 118.0 | Slightly slower before the DirectWrite glyph cache warms up (first launch) |
| Linux (Wayland) | 205 | 102.7 | Within 5% of X11 (same machine) |
| Linux (X11) | 214 | 104.1 | Measured on the same machine (cold cache) |

If you want the latency sensitivity quantified across batch sizes of 1k / 10k / 100k rows, I can run that analysis next.

[1]: https://example.com/architecture "Pipeline architecture overview"
[2]: https://example.com/load-test "March 2026 load test results"
[3]: https://example.com/load-test-2 "Unbatched write throughput follow-up"
[4]: https://example.com/cache-report "Row cache weekly report"
[5]: https://example.com/downsampling "Downsampling tier capacity forecast"
[6]: https://example.com/wal-saturation "Ingest throughput collapses with WAL saturated under load"
[7]: https://example.com/recovery "Ingest could restore pre-migration throughput within a week"
[8]: https://example.com/failover "Analysis of the failover drill impact on replication"
[9]: https://example.com/segments "Segment readers"
[10]: https://example.com/aggregation "Aggregation tier: the upcoming star of the pipeline"
[11]: https://example.com/benchmarks "v2025 benchmark suite results"
[12]: https://example.com/parity "Dual-write parity still weeks away"
[13]: https://example.com/traffic "Lookup shim: traffic security and query routing"
[14]: https://example.com/wal-retention "Implications of WAL retention for recovery"
