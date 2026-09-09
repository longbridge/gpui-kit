//! Reads this process' DRM `fdinfo` counters, the per-client GPU accounting the
//! kernel publishes and tools like `nvtop` and `intel_gpu_top` read. amdgpu,
//! i915, xe and panfrost fill it in; a driver that does not — nvidia's
//! proprietary one, which keeps its accounting in NVML — leaves the HUD without
//! a GPU reading rather than showing a flat zero.

use std::{
    collections::{HashMap, HashSet},
    fs,
};

use web_time::Instant;

/// One file per open file descriptor of this process, a few of which are the
/// DRM devices it renders through.
const FDINFO: &str = "/proc/self/fdinfo";

/// Identifies the DRM client a descriptor belongs to. Several descriptors can
/// share one, each reporting the same running totals, so the counters are
/// gathered per client and not per descriptor.
const CLIENT_ID: &str = "drm-client-id:";

/// Prefixes the busy time of one engine, as in `drm-engine-render: 12345 ns`.
/// `drm-engine-capacity-*` shares the prefix but counts engines rather than
/// nanoseconds, which is what the unit is checked for.
const ENGINE: &str = "drm-engine-";
const NANOSECONDS: &str = "ns";

pub(super) struct Probe {
    /// The previous totals per engine and when they were taken, once a first
    /// reading has landed. Engine time only accumulates, so a percentage is
    /// the time gained between two readings over the wall clock between them.
    last: Option<(HashMap<String, u64>, Instant)>,
}

impl Probe {
    pub(super) fn new() -> Option<Self> {
        // Only a check that the counters are reachable at all. Whether this
        // process' driver publishes them is settled by the first sample, since
        // at startup it may not yet have opened the device.
        fs::metadata(FDINFO).ok()?;
        Some(Self { last: None })
    }

    /// The busiest engine, not the sum over all of them: render, copy and video
    /// decode run concurrently, so adding them up can pass 100% while the GPU
    /// still has headroom.
    pub(super) fn sample(&mut self) -> Option<f32> {
        let busy = engine_nanoseconds()?;
        let now = Instant::now();
        let (previous, at) = self.last.replace((busy.clone(), now))?;

        let elapsed = now.duration_since(at).as_nanos();
        if elapsed == 0 {
            return None;
        }

        busy.iter()
            .map(|(engine, used)| {
                // Saturating because an engine the process stops using can be
                // dropped and re-reported from zero.
                let gained = used.saturating_sub(previous.get(engine).copied().unwrap_or(0));
                gained as f64 / elapsed as f64 * 100.
            })
            .reduce(f64::max)
            .map(|busy| busy as f32)
    }
}

/// Nanoseconds each engine has spent on this process, or `None` when its driver
/// reports no engine at all.
fn engine_nanoseconds() -> Option<HashMap<String, u64>> {
    let mut totals: HashMap<String, u64> = HashMap::new();
    // Descriptors of one client repeat that client's totals, so only the first
    // of them is counted.
    let mut counted = HashSet::new();
    let mut found = false;

    for entry in fs::read_dir(FDINFO).ok()?.flatten() {
        // A descriptor can close between listing the directory and reading it,
        // and most of them are not DRM devices to begin with.
        let Ok(fdinfo) = fs::read_to_string(entry.path()) else {
            continue;
        };
        let Some(client) = client_id(&fdinfo) else {
            continue;
        };
        if !counted.insert(client) {
            continue;
        }

        for (engine, used) in engines(&fdinfo) {
            found = true;
            *totals.entry(engine).or_default() += used;
        }
    }

    found.then_some(totals)
}

fn client_id(fdinfo: &str) -> Option<u64> {
    fdinfo
        .lines()
        .find_map(|line| line.strip_prefix(CLIENT_ID))
        .and_then(|id| id.trim().parse().ok())
}

fn engines(fdinfo: &str) -> impl Iterator<Item = (String, u64)> {
    fdinfo.lines().filter_map(|line| {
        let (engine, busy) = line.strip_prefix(ENGINE)?.split_once(':')?;
        let nanoseconds = busy.trim().strip_suffix(NANOSECONDS)?;
        Some((engine.to_owned(), nanoseconds.trim().parse().ok()?))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FDINFO_SAMPLE: &str = "\
pos:	0
drm-driver:	amdgpu
drm-client-id:	42
drm-engine-gfx:	1200000 ns
drm-engine-compute:	0 ns
drm-engine-capacity-gfx:	2
drm-memory-vram:	131072 KiB
";

    #[test]
    fn reads_engine_times_and_skips_the_capacity_counters() {
        assert_eq!(client_id(FDINFO_SAMPLE), Some(42));

        let mut engines: Vec<_> = engines(FDINFO_SAMPLE).collect();
        engines.sort();
        assert_eq!(
            engines,
            vec![("compute".to_owned(), 0), ("gfx".to_owned(), 1_200_000)],
            "`drm-engine-capacity-gfx` counts engines, not nanoseconds"
        );
    }

    #[test]
    fn a_descriptor_without_drm_counters_reports_nothing() {
        let fdinfo = "pos:\t0\nflags:\t02\nmnt_id:\t24\n";
        assert_eq!(client_id(fdinfo), None);
        assert_eq!(engines(fdinfo).count(), 0);
    }
}
