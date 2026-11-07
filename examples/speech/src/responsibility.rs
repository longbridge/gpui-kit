//! Make this process responsible for its own privacy access on macOS.
//!
//! macOS attributes a process launched from a terminal to the terminal app,
//! and checks the terminal's `Info.plist` for usage descriptions instead of
//! the one `build.rs` links into this executable. A terminal that declares
//! microphone use but not speech recognition, such as Orca, gets the process
//! killed the moment it asks for speech recognition access.
//!
//! [`disclaim`] relaunches the executable with the terminal's responsibility
//! disclaimed, the way debuggers and editors launch the programs they run, so
//! the system reads this executable's own usage descriptions. The first
//! process waits for the relaunched one and exits with its status.

use std::{
    ffi::{CString, OsString},
    os::unix::ffi::OsStrExt as _,
    ptr,
};

/// Set in the relaunched process, so it does not relaunch again.
const DISCLAIMED: &str = "SPEECH_RESPONSIBILITY_DISCLAIMED";

unsafe extern "C" {
    /// Exported by libSystem; declared in no public header.
    fn responsibility_spawnattrs_setdisclaim(
        attrs: *mut libc::posix_spawnattr_t,
        disclaim: libc::c_int,
    ) -> libc::c_int;
}

/// Relaunch this executable as its own responsible process, unless this one
/// already is. Returns only in the process that should run the app; when the
/// relaunch fails, that is this process, and access requests may fail as
/// before.
pub fn disclaim() {
    if std::env::var_os(DISCLAIMED).is_some() {
        return;
    }
    match relaunch() {
        Ok(status) => std::process::exit(status),
        Err(err) => eprintln!("Speech could not disclaim the terminal's responsibility: {err}"),
    }
}

/// Spawn this executable with the same arguments, wait for it, and return the
/// status to exit with.
fn relaunch() -> std::io::Result<i32> {
    let path = c_string(std::env::current_exe()?.into_os_string())?;
    let args = std::env::args_os()
        .map(c_string)
        .collect::<std::io::Result<Vec<_>>>()?;
    let env = std::env::vars_os()
        .map(|(key, value)| {
            let mut pair = key;
            pair.push("=");
            pair.push(value);
            c_string(pair)
        })
        .chain(std::iter::once(c_string(format!("{DISCLAIMED}=1").into())))
        .collect::<std::io::Result<Vec<_>>>()?;

    let argv = null_terminated(&args);
    let envp = null_terminated(&env);

    let mut pid: libc::pid_t = 0;
    // SAFETY: `attrs` is initialized before use and destroyed after the
    // spawn; `argv` and `envp` are null-terminated arrays of pointers into
    // `args` and `env`, which outlive the call.
    let spawned = unsafe {
        let mut attrs: libc::posix_spawnattr_t = ptr::null_mut();
        check(libc::posix_spawnattr_init(&mut attrs))?;
        let spawned = check(responsibility_spawnattrs_setdisclaim(&mut attrs, 1)).and_then(|_| {
            check(libc::posix_spawn(
                &mut pid,
                path.as_ptr(),
                ptr::null(),
                &attrs,
                argv.as_ptr(),
                envp.as_ptr(),
            ))
        });
        libc::posix_spawnattr_destroy(&mut attrs);
        spawned
    };
    spawned?;

    let mut status = 0;
    loop {
        // SAFETY: `pid` is the child spawned above and `status` is a valid
        // out pointer.
        if unsafe { libc::waitpid(pid, &mut status, 0) } != -1 {
            break;
        }
        let err = std::io::Error::last_os_error();
        if err.kind() != std::io::ErrorKind::Interrupted {
            return Err(err);
        }
    }

    Ok(if libc::WIFEXITED(status) {
        libc::WEXITSTATUS(status)
    } else {
        128 + libc::WTERMSIG(status)
    })
}

fn c_string(value: OsString) -> std::io::Result<CString> {
    CString::new(value.as_bytes()).map_err(std::io::Error::other)
}

fn null_terminated(values: &[CString]) -> Vec<*mut libc::c_char> {
    values
        .iter()
        .map(|value| value.as_ptr().cast_mut())
        .chain(std::iter::once(ptr::null_mut()))
        .collect()
}

/// Turn a `posix_spawn`-style return code into a result.
fn check(code: libc::c_int) -> std::io::Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(std::io::Error::from_raw_os_error(code))
    }
}
