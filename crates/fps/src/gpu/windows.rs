//! Reads the `GPU Engine` performance counters through PDH, the same source
//! Task Manager's GPU column uses. Each instance is named after the process
//! that owns it, so this process' share falls out of the instance names. No
//! driver SDK is involved, so it works the same on any vendor's adapter.

use std::collections::HashMap;

use windows::{
    Win32::System::Performance::{
        PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA,
        PdhAddEnglishCounterW, PdhCloseQuery, PdhCollectQueryData, PdhGetFormattedCounterArrayW,
        PdhOpenQueryW,
    },
    core::w,
};

/// Every engine of every adapter, for every process; this process' instances
/// are picked out of the result. PDH has no way to ask for one process' engines
/// up front, since the instance name is not known until it is enumerated. The
/// counter is registered under its English name so it resolves on a localized
/// Windows too.
const COUNTER_PATH: windows::core::PCWSTR = w!("\\GPU Engine(*)\\Utilization Percentage");

/// PDH's own success code, which is `ERROR_SUCCESS` rather than a PDH status.
const PDH_SUCCESS: u32 = 0;

/// Marks the end of an instance name's engine type, as in
/// `pid_4242_luid_0x00000000_0x0000BEEF_phys_0_eng_1_engtype_3D`. The same name
/// opens with the owning process, which is what [`Probe::owner`] matches.
const ENGINE_TYPE: &str = "engtype_";

pub(super) struct Probe {
    query: PDH_HQUERY,
    counter: PDH_HCOUNTER,
    /// How every instance name this process owns opens, as in `pid_4242_`.
    owner: String,
    /// Reused across samples so a reading twice a second does not reallocate.
    /// PDH writes the instance name strings into the tail of the same buffer,
    /// past the array it fills, so this is sized in bytes rather than items.
    /// 64-bit words provide the array's required alignment without retaining
    /// the non-`Send` pointers in `PDH_FMT_COUNTERVALUE_ITEM_W` between calls.
    buffer: Vec<u64>,
}

// SAFETY: The PDH query and counter handles are owned exclusively by this
// `Probe`, accessed only through `&mut self`, and closed once in `Drop`.
unsafe impl Send for Probe {}

impl Probe {
    pub(super) fn new() -> Option<Self> {
        let mut query = PDH_HQUERY::default();
        // SAFETY: both handles are live out parameters, and the counter path is
        // a static wide string.
        let opened = unsafe { PdhOpenQueryW(None, 0, &mut query) };
        if opened != PDH_SUCCESS {
            return None;
        }

        let mut probe = Self {
            query,
            counter: PDH_HCOUNTER::default(),
            owner: format!("pid_{}_", std::process::id()),
            buffer: Vec::new(),
        };
        // SAFETY: the query was just opened, and is closed by `Drop` either way.
        let added =
            unsafe { PdhAddEnglishCounterW(probe.query, COUNTER_PATH, 0, &mut probe.counter) };
        if added != PDH_SUCCESS {
            return None;
        }

        // Utilization is a rate, so it is the difference between two
        // collections. This one is the baseline the first sample subtracts
        // from; without it that sample would read zero.
        //
        // SAFETY: the query is live.
        unsafe { PdhCollectQueryData(probe.query) };
        Some(probe)
    }

    /// The busiest engine *type* this process is using, not the sum over all of
    /// them: 3D, Copy and Video Decode run concurrently, so adding them up can
    /// pass 100% while the adapter still has headroom. Within one type this
    /// process' engines are summed, since it can be on several at once.
    pub(super) fn sample(&mut self) -> Option<f32> {
        // SAFETY: the query is live until `Drop`.
        if unsafe { PdhCollectQueryData(self.query) } != PDH_SUCCESS {
            return None;
        }

        let mut busy: HashMap<String, f64> = HashMap::new();
        for (instance, utilization) in self.collect()? {
            if !instance.starts_with(&self.owner) {
                continue;
            }
            let Some((_, engine)) = instance.rsplit_once(ENGINE_TYPE) else {
                continue;
            };
            *busy.entry(engine.to_owned()).or_default() += utilization;
        }

        // Zero rather than nothing once the process has engines at all: it is
        // idle, not unmeasurable. Nothing means the adapter reported none.
        busy.into_values().reduce(f64::max).map(|busy| busy as f32)
    }

    /// Reads the formatted counter array, growing the buffer to whatever PDH
    /// asks for. The first call reports the size it needs, which changes as
    /// processes come and go.
    fn collect(&mut self) -> Option<Vec<(String, f64)>> {
        let mut bytes = 0;
        let mut count = 0;
        // SAFETY: passing no buffer asks for the required size, which PDH
        // reports through `PDH_MORE_DATA`.
        let sized = unsafe {
            PdhGetFormattedCounterArrayW(self.counter, PDH_FMT_DOUBLE, &mut bytes, &mut count, None)
        };
        if sized != PDH_MORE_DATA || bytes == 0 {
            return None;
        }

        const {
            assert!(
                align_of::<u64>() >= align_of::<PDH_FMT_COUNTERVALUE_ITEM_W>(),
                "PDH buffer word must satisfy the counter item alignment"
            );
        }
        let words = (bytes as usize).div_ceil(size_of::<u64>());
        self.buffer.clear();
        self.buffer.resize(words, 0);

        // SAFETY: the buffer holds at least the `bytes` PDH asked for, so it
        // can write both the array and the names that follow it.
        let read = unsafe {
            PdhGetFormattedCounterArrayW(
                self.counter,
                PDH_FMT_DOUBLE,
                &mut bytes,
                &mut count,
                Some(self.buffer.as_mut_ptr().cast()),
            )
        };
        if read != PDH_SUCCESS {
            return None;
        }

        // SAFETY: PDH filled `count` items at the start of this suitably
        // aligned buffer, and `read` above succeeded.
        let items = unsafe {
            std::slice::from_raw_parts(
                self.buffer.as_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>(),
                count as usize,
            )
        };
        let readings = items
            .iter()
            .filter_map(|item| {
                // SAFETY: PDH filled `count` items, each naming its instance in
                // the buffer's tail, and the value is the `f64` the double
                // format asked for.
                let instance = unsafe { item.szName.to_string() }.ok()?;
                Some((instance, unsafe { item.FmtValue.Anonymous.doubleValue }))
            })
            .collect();
        Some(readings)
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        // SAFETY: the query was opened in `new` and is closed exactly once.
        // Closing it also releases the counter added to it.
        unsafe { PdhCloseQuery(self.query) };
    }
}
