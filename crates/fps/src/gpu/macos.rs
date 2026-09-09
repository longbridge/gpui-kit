//! Sums the `accumulatedGPUTime` this process' accelerator clients report in
//! the IO registry, and turns the time gained between two readings into a share
//! of the wall clock that passed. This is the counter behind Activity Monitor's
//! per-process GPU column, and reading it needs no privileges — unlike
//! `powermetrics`.
//!
//! The accelerator's own `PerformanceStatistics` would be easier to read but is
//! device wide, so it would report the compositor and every other application
//! as this one's load. `task_info`'s `task_gpu_utilisation` is per process but
//! is left at zero on Apple silicon, so it is no use either.

use std::{ffi::c_char, ptr::NonNull};

use objc2_core_foundation::{CFArray, CFDictionary, CFNumber, CFNumberType, CFRetained, CFString};
use objc2_io_kit::{
    IOIteratorNext, IOObjectRelease, IORegistryEntryCreateCFProperties,
    IORegistryEntryGetChildIterator, IOServiceGetMatchingServices, IOServiceMatching,
    io_iterator_t, io_registry_entry_t, kIOMainPortDefault, kIOReturnSuccess,
};
use web_time::Instant;

/// The registry plane the accelerator's clients hang off, as the fixed size
/// buffer the call takes.
fn service_plane() -> [c_char; 128] {
    let mut plane = [0; 128];
    for (slot, byte) in plane.iter_mut().zip(b"IOService") {
        *slot = *byte as c_char;
    }
    plane
}

pub(super) struct Probe {
    /// How the registry tags the clients this process opened, as in
    /// `pid 4242, my_app`. Only the pid is matched — the name is truncated to
    /// 16 characters, so it is not reliable to compare against.
    creator: String,
    /// The previous total and when it was taken, once a first reading has
    /// landed. GPU time only accumulates, so a percentage is the time gained
    /// between two readings over the wall clock between them.
    last: Option<(u64, Instant)>,
}

impl Probe {
    pub(super) fn new() -> Option<Self> {
        // Doubles as the support check: no accelerator, no readings, and the
        // HUD leaves the row out rather than showing a flat zero.
        let accelerators = accelerators();
        if accelerators.is_empty() {
            return None;
        }
        release(accelerators);

        Some(Self {
            creator: format!("pid {},", std::process::id()),
            last: None,
        })
    }

    pub(super) fn sample(&mut self) -> Option<f32> {
        // `None` while this process has yet to open an accelerator client,
        // which is true for the moment between startup and the first frame.
        let used = accumulated_nanoseconds(&self.creator)?;
        let now = Instant::now();
        let (previous, at) = self.last.replace((used, now))?;

        let elapsed = now.duration_since(at).as_nanos();
        if elapsed == 0 {
            return None;
        }
        // Saturating because a client that goes away takes its share of the
        // total with it, which can walk the sum backwards.
        let busy = used.saturating_sub(previous);
        Some((busy as f64 / elapsed as f64 * 100.) as f32)
    }
}

/// Nanoseconds every accelerator has spent on this process, or `None` when it
/// owns no client that reports them.
fn accumulated_nanoseconds(creator: &str) -> Option<u64> {
    // Passed by mutable pointer because the call's parameter type says so, not
    // because it writes to the name.
    let mut plane = service_plane();
    let mut total = None;

    for accelerator in accelerators() {
        let mut clients: io_iterator_t = 0;
        // SAFETY: the accelerator is live until it is released below, and both
        // the plane name and the iterator are valid out parameters.
        let result =
            unsafe { IORegistryEntryGetChildIterator(accelerator, &mut plane, &mut clients) };
        IOObjectRelease(accelerator);
        if result != kIOReturnSuccess || clients == 0 {
            continue;
        }

        loop {
            let client = IOIteratorNext(clients);
            if client == 0 {
                break;
            }
            if let Some(used) = client_nanoseconds(client, creator) {
                *total.get_or_insert(0) += used;
            }
            IOObjectRelease(client);
        }
        IOObjectRelease(clients);
    }

    total
}

/// What one client has accumulated, or `None` if it belongs to another process
/// or reports no usage at all — the latter being how an Intel Mac, which does
/// not publish `AppUsage`, ends up with no GPU row.
fn client_nanoseconds(client: io_registry_entry_t, creator: &str) -> Option<u64> {
    let properties = properties(client)?;

    // SAFETY: the registry publishes the creator as a string and the usage as
    // an array of dictionaries, and every borrow below is taken out of
    // `properties` while it is alive.
    unsafe {
        let owner: NonNull<CFString> = value(&properties, "IOUserClientCreator")?;
        if !owner.as_ref().to_string().starts_with(creator) {
            return None;
        }

        let usage: NonNull<CFArray> = value(&properties, "AppUsage")?;
        let usage = usage.as_ref();
        let total = (0..usage.count())
            .filter_map(|index| {
                let queue = NonNull::new(usage.value_at_index(index).cast_mut())?;
                let queue: NonNull<CFDictionary> = queue.cast();
                let time: NonNull<CFNumber> = value(queue.as_ref(), "accumulatedGPUTime")?;
                integer(time.as_ref())
            })
            .sum();
        Some(total)
    }
}

/// Every registered accelerator. The caller owns what is returned and must
/// release it.
fn accelerators() -> Vec<io_registry_entry_t> {
    let mut accelerators = Vec::new();

    // SAFETY: the class name is a valid C string, and the dictionary it returns
    // is handed straight to the lookup below, which takes over its reference.
    let Some(matching) = (unsafe { IOServiceMatching(c"IOAccelerator".as_ptr()) }) else {
        return accelerators;
    };

    let mut iterator: io_iterator_t = 0;
    // SAFETY: the matching dictionary is a plain `CFDictionary` under its
    // mutable type, and the iterator is a live out parameter.
    let result = unsafe {
        IOServiceGetMatchingServices(
            kIOMainPortDefault,
            Some(CFRetained::cast_unchecked::<CFDictionary>(matching)),
            &mut iterator,
        )
    };
    if result != kIOReturnSuccess || iterator == 0 {
        return accelerators;
    }

    loop {
        let accelerator = IOIteratorNext(iterator);
        if accelerator == 0 {
            break;
        }
        accelerators.push(accelerator);
    }

    IOObjectRelease(iterator);
    accelerators
}

fn release(entries: Vec<io_registry_entry_t>) {
    for entry in entries {
        IOObjectRelease(entry);
    }
}

/// A snapshot of one registry entry's properties.
fn properties(entry: io_registry_entry_t) -> Option<CFRetained<CFDictionary>> {
    let mut properties = std::ptr::null_mut();
    // SAFETY: the entry is live for the length of the call, and `properties` is
    // a valid out parameter.
    let result = unsafe { IORegistryEntryCreateCFProperties(entry, &mut properties, None, 0) };
    if result != kIOReturnSuccess {
        return None;
    }

    // SAFETY: on success the call returns a snapshot owned by this task, which
    // the `CFRetained` releases when it is dropped.
    let properties = unsafe { CFRetained::from_raw(NonNull::new(properties)?) };
    // SAFETY: a mutable dictionary is a dictionary; nothing here mutates it.
    Some(unsafe { CFRetained::cast_unchecked::<CFDictionary>(properties) })
}

/// Borrows `key` out of `dictionary`.
///
/// # Safety
///
/// The value stored under `key` must be a `T`, and the borrow must not outlive
/// `dictionary`.
unsafe fn value<T>(dictionary: &CFDictionary, key: &str) -> Option<NonNull<T>> {
    let key = CFString::from_str(key);
    // SAFETY: the key is alive for the length of the lookup, and the value is
    // borrowed from the dictionary rather than owned.
    let value = unsafe { dictionary.value(std::ptr::from_ref(&*key).cast()) };
    NonNull::new(value.cast_mut().cast::<T>())
}

fn integer(number: &CFNumber) -> Option<u64> {
    let mut value: i64 = 0;
    // SAFETY: the destination matches the type asked for.
    let read = unsafe {
        number.value(
            CFNumberType::SInt64Type,
            std::ptr::from_mut(&mut value).cast(),
        )
    };
    read.then(|| value.max(0) as u64)
}
