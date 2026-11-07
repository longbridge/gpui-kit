//! Bounded child-process execution for the standard runtime adapter.

use std::{
    ffi::OsStr,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

const STDOUT_EXCEEDED: u8 = 1;
const STDERR_EXCEEDED: u8 = 2;

#[derive(Clone)]
pub(crate) struct Cancellation(Arc<AtomicBool>);

impl Cancellation {
    pub(crate) fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub(crate) fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Limits {
    timeout: Duration,
    stdout: usize,
    stderr: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            stdout: 8 * 1024 * 1024,
            stderr: 8 * 1024 * 1024,
        }
    }
}

impl Limits {
    /// The same output bounds with a different deadline, for a caller that
    /// asked for one.
    pub(crate) fn with_timeout(self, timeout: Duration) -> Self {
        Self { timeout, ..self }
    }
}

#[cfg(test)]
impl Limits {
    fn for_test(timeout: Duration, output: usize) -> Self {
        Self {
            timeout,
            stdout: output,
            stderr: output,
        }
    }
}

#[derive(Debug)]
pub(crate) struct Output {
    pub(crate) code: i32,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

pub(crate) fn run_bounded(
    command: &str,
    args: &[String],
    cwd: Option<&Path>,
    limits: Limits,
    cancellation: Cancellation,
) -> Result<Output, String> {
    let executable = resolve_executable(command, std::env::var_os("PATH").as_deref())?;
    let mut command_builder = Command::new(&executable);
    if let Some(cwd) = cwd {
        command_builder.current_dir(cwd);
    }
    command_builder
        // A process grant authorizes one executable, not the host's ambient
        // credentials. Keep the child environment empty until the public API
        // grows an explicit, capability-reviewed environment allowlist.
        .env_clear()
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process_tree(&mut command_builder);
    let mut process_tree = ProcessTree::new()?;
    let mut child = command_builder
        .spawn()
        .map_err(|error| format!("running `{command}` failed: {error}"))?;
    if let Err(error) = process_tree.attach(&child) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!(
            "isolating `{command}` process tree failed: {error}"
        ));
    }

    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let exceeded = Arc::new(AtomicU8::new(0));
    let stdout_reader = read_stream(stdout, limits.stdout, STDOUT_EXCEEDED, exceeded.clone());
    let stderr_reader = read_stream(stderr, limits.stderr, STDERR_EXCEEDED, exceeded.clone());
    let started = Instant::now();
    let mut status = None;
    let mut stdout = None;
    let mut stderr = None;

    loop {
        if let Err(error) = receive_stream(&stdout_reader, &mut stdout, "stdout")
            .and_then(|()| receive_stream(&stderr_reader, &mut stderr, "stderr"))
        {
            kill_process_tree(&mut process_tree, &mut child);
            let _ = child.wait();
            return Err(error);
        }

        let reason = if cancellation.is_cancelled() {
            Some("was cancelled".to_owned())
        } else if started.elapsed() >= limits.timeout {
            Some(format!("timed out after {} ms", limits.timeout.as_millis()))
        } else {
            match exceeded.load(Ordering::Acquire) {
                STDOUT_EXCEEDED => Some(format!("stdout exceeded {} bytes", limits.stdout)),
                STDERR_EXCEEDED => Some(format!("stderr exceeded {} bytes", limits.stderr)),
                _ => None,
            }
        };

        if let Some(reason) = reason {
            kill_process_tree(&mut process_tree, &mut child);
            let _ = child.wait();
            return Err(format!("`{command}` {reason}"));
        }

        if status.is_none() {
            match child.try_wait() {
                Ok(Some(done)) => status = Some(done),
                Ok(None) => {}
                Err(error) => {
                    kill_process_tree(&mut process_tree, &mut child);
                    let _ = child.wait();
                    return Err(format!("waiting for `{command}` failed: {error}"));
                }
            }
        }

        if status.is_some() && stdout.is_some() && stderr.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }

    let status = status.expect("loop exits only with child status");
    let stdout = stdout.expect("loop exits only after stdout closes")?;
    let stderr = stderr.expect("loop exits only after stderr closes")?;
    Ok(Output {
        code: status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}

fn resolve_executable(command: &str, path: Option<&OsStr>) -> Result<PathBuf, String> {
    let command_path = Path::new(command);
    if command_path.components().count() > 1 {
        return Ok(command_path.to_path_buf());
    }
    let path = path.ok_or_else(|| {
        format!("running `{command}` failed: the host PATH environment variable is not set")
    })?;
    for directory in std::env::split_paths(path) {
        let candidate = directory.join(command);
        if executable_file(&candidate) {
            return Ok(candidate);
        }
        #[cfg(windows)]
        if candidate.extension().is_none() {
            for extension in ["exe", "com", "bat", "cmd"] {
                let candidate = candidate.with_extension(extension);
                if executable_file(&candidate) {
                    return Ok(candidate);
                }
            }
        }
    }
    Err(format!(
        "running `{command}` failed: executable was not found on the host PATH"
    ))
}

#[cfg(unix)]
fn executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn executable_file(path: &Path) -> bool {
    path.is_file()
}

fn read_stream(
    mut stream: impl Read + Send + 'static,
    limit: usize,
    flag: u8,
    exceeded: Arc<AtomicU8>,
) -> mpsc::Receiver<Result<Vec<u8>, String>> {
    let (sender, receiver) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let result = (|| {
            let mut output = Vec::new();
            let mut chunk = [0_u8; 8192];
            loop {
                let count = stream.read(&mut chunk).map_err(|error| error.to_string())?;
                if count == 0 {
                    return Ok(output);
                }
                if output.len().saturating_add(count) > limit {
                    let _ = exceeded.compare_exchange(0, flag, Ordering::AcqRel, Ordering::Acquire);
                    return Err(format!("exceeded {limit} bytes"));
                }
                output.extend_from_slice(&chunk[..count]);
            }
        })();
        let _ = sender.send(result);
    });
    receiver
}

fn receive_stream(
    reader: &mpsc::Receiver<Result<Vec<u8>, String>>,
    output: &mut Option<Result<Vec<u8>, String>>,
    name: &str,
) -> Result<(), String> {
    if output.is_some() {
        return Ok(());
    }
    match reader.try_recv() {
        Ok(result) => *output = Some(result.map_err(|error| format!("child {name} {error}"))),
        Err(mpsc::TryRecvError::Empty) => {}
        Err(mpsc::TryRecvError::Disconnected) => {
            return Err(format!("reading child {name} stopped unexpectedly"));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn configure_process_tree(command: &mut Command) {
    use std::os::unix::process::CommandExt as _;
    command.process_group(0);
}

#[cfg(windows)]
fn configure_process_tree(command: &mut Command) {
    use std::os::windows::process::CommandExt as _;
    use windows::Win32::System::Threading::CREATE_SUSPENDED;

    // Assignment to a Job Object is not an atomic option on `Command`. Keep
    // the primary thread suspended so the process cannot create a descendant
    // in the interval between `spawn` and `AssignProcessToJobObject`.
    command.creation_flags(CREATE_SUSPENDED.0);
}

#[cfg(not(any(unix, windows)))]
fn configure_process_tree(_: &mut Command) {}

#[cfg(unix)]
struct ProcessTree;

#[cfg(unix)]
impl ProcessTree {
    fn new() -> Result<Self, String> {
        Ok(Self)
    }

    fn attach(&mut self, _: &std::process::Child) -> Result<(), String> {
        Ok(())
    }

    fn terminate(&mut self, child: &mut std::process::Child) {
        self.terminate_group(child.id());
    }

    fn terminate_group(&mut self, pid: u32) {
        // The child is the leader of the process group configured above. A
        // negative pid targets the whole group, including descendants that
        // inherited stdout/stderr and would otherwise keep the readers alive.
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }

    fn close_after_exit(&mut self) {}
}

#[cfg(windows)]
struct ProcessTree {
    job: Option<std::os::windows::io::OwnedHandle>,
}

#[cfg(windows)]
impl ProcessTree {
    fn new() -> Result<Self, String> {
        use std::os::windows::io::{AsRawHandle as _, FromRawHandle as _, OwnedHandle};
        use windows::{
            Win32::System::JobObjects::{
                CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                SetInformationJobObject,
            },
            core::PCWSTR,
        };

        let job = unsafe { CreateJobObjectW(None, PCWSTR::null()) }
            .map_err(|error| format!("creating Windows Job Object failed: {error}"))?;
        // SAFETY: CreateJobObjectW returned a valid, uniquely owned handle.
        let job = unsafe { OwnedHandle::from_raw_handle(job.0) };
        let mut information = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        information.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if let Err(error) = unsafe {
            SetInformationJobObject(
                windows::Win32::Foundation::HANDLE(job.as_raw_handle()),
                JobObjectExtendedLimitInformation,
                std::ptr::from_ref(&information).cast(),
                std::mem::size_of_val(&information) as u32,
            )
        } {
            return Err(format!("configuring Windows Job Object failed: {error}"));
        }
        Ok(Self { job: Some(job) })
    }

    fn attach(&mut self, child: &std::process::Child) -> Result<(), String> {
        use std::os::windows::io::AsRawHandle as _;
        use windows::Win32::{Foundation::HANDLE, System::JobObjects::AssignProcessToJobObject};

        let job = self.job.as_ref().expect("a live process tree owns its job");
        let process = HANDLE(child.as_raw_handle());
        unsafe { AssignProcessToJobObject(HANDLE(job.as_raw_handle()), process) }
            .map_err(|error| format!("assigning child to Windows Job Object failed: {error}"))?;
        resume_suspended_process(child.id())
    }

    fn terminate(&mut self, _: &mut std::process::Child) {
        self.close();
    }

    fn terminate_group(&mut self, _: u32) {
        self.close();
    }

    fn close_after_exit(&mut self) {
        self.close();
    }

    fn close(&mut self) {
        self.job.take();
    }
}

#[cfg(windows)]
fn resume_suspended_process(process_id: u32) -> Result<(), String> {
    use windows::Win32::{
        Foundation::CloseHandle,
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First,
                Thread32Next,
            },
            Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME},
        },
    };

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }
        .map_err(|error| format!("snapshotting Windows threads failed: {error}"))?;
    let result = (|| {
        let mut entry = THREADENTRY32 {
            dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
            ..Default::default()
        };
        unsafe { Thread32First(snapshot, &mut entry) }
            .map_err(|error| format!("enumerating Windows threads failed: {error}"))?;
        let thread_id = loop {
            if entry.th32OwnerProcessID == process_id {
                break entry.th32ThreadID;
            }
            if unsafe { Thread32Next(snapshot, &mut entry) }.is_err() {
                return Err(format!(
                    "the suspended Windows process {process_id} had no primary thread"
                ));
            }
        };
        let thread = unsafe { OpenThread(THREAD_SUSPEND_RESUME, false, thread_id) }
            .map_err(|error| format!("opening the suspended Windows thread failed: {error}"))?;
        let resumed = unsafe { ResumeThread(thread) };
        unsafe {
            let _ = CloseHandle(thread);
        }
        if resumed == u32::MAX {
            return Err(format!(
                "resuming the suspended Windows thread failed: {}",
                windows::core::Error::from_win32()
            ));
        }
        Ok(())
    })();
    unsafe {
        let _ = CloseHandle(snapshot);
    }
    result
}

#[cfg(windows)]
impl Drop for ProcessTree {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(not(any(unix, windows)))]
struct ProcessTree;

#[cfg(not(any(unix, windows)))]
impl ProcessTree {
    fn new() -> Result<Self, String> {
        Ok(Self)
    }

    fn attach(&mut self, _: &std::process::Child) -> Result<(), String> {
        Ok(())
    }

    fn terminate(&mut self, _: &mut std::process::Child) {}

    fn terminate_group(&mut self, _: u32) {}

    fn close_after_exit(&mut self) {}
}

fn kill_process_tree(tree: &mut ProcessTree, child: &mut std::process::Child) {
    tree.terminate(child);
    let _ = child.kill();
}

// ---------------------------------------------------------------------------
// Streaming
// ---------------------------------------------------------------------------

/// The longest line a line-mode process may write before it is stopped.
///
/// There is no bound on a process's total output — a long-running agent
/// streams for as long as it runs — but a line is held whole until its newline
/// arrives, so an endless one would grow without limit.
pub(crate) const MAX_LINE_BYTES: usize = 8 * 1024 * 1024;

/// How the waiting thread polls for the child, the deadline and a kill.
const WAIT_SLICE: Duration = Duration::from_millis(20);

/// How a streaming child was asked to run.
#[derive(Clone, Debug, Default)]
pub(crate) struct SpawnOptions {
    pub(crate) cwd: Option<PathBuf>,
    pub(crate) stdin: bool,
    pub(crate) timeout: Option<Duration>,
    pub(crate) lines: bool,
}

/// Something a streaming child did, in the order it happened.
///
/// `Exit` is always the last event, and comes only after both streams have
/// delivered everything they had.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ProcessEvent {
    Stdout(String),
    Stderr(String),
    Exit {
        code: Option<i32>,
        signal: Option<String>,
        error: Option<String>,
    },
}

impl ProcessEvent {
    pub(crate) fn is_exit(&self) -> bool {
        matches!(self, Self::Exit { .. })
    }
}

/// A running child: its events, its stdin, and the means to stop it.
pub(crate) struct SpawnedProcess {
    pub(crate) pid: u32,
    pub(crate) events: async_channel::Receiver<ProcessEvent>,
    pub(crate) stdin: Option<StdinWriter>,
    pub(crate) killer: Killer,
}

enum StdinMessage {
    Write(String),
    End,
}

/// Feeds a child's stdin from its own thread, so a child that is slow to read
/// never blocks the caller.
pub(crate) struct StdinWriter {
    sender: Option<mpsc::Sender<StdinMessage>>,
}

impl StdinWriter {
    pub(crate) fn write(&mut self, text: String) -> Result<(), String> {
        let sender = self
            .sender
            .as_ref()
            .ok_or("stdin has already been closed")?;
        sender
            .send(StdinMessage::Write(text))
            .map_err(|_| "stdin is closed: the process is no longer reading it".to_owned())
    }

    pub(crate) fn end(&mut self) {
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(StdinMessage::End);
        }
    }
}

/// Stops a streaming child and everything it started.
#[derive(Clone)]
pub(crate) struct Killer {
    pid: u32,
    tree: Arc<std::sync::Mutex<Option<ProcessTree>>>,
    /// Set by an explicit kill, so the exit says nothing went wrong.
    killed: Arc<AtomicBool>,
    /// Why the runtime stopped the child, when it was the runtime.
    reason: Arc<std::sync::Mutex<Option<String>>>,
    /// Set once the child has been reaped and its group stopped for the last
    /// time; its pid may then belong to something else and is not signalled
    /// again.
    reaped: Arc<AtomicBool>,
}

impl Killer {
    /// `ChildProcess.kill()`, and the cancellation the runtime uses when the
    /// view or application that started the child goes away.
    pub(crate) fn kill(&self) {
        self.killed.store(true, Ordering::Release);
        self.terminate();
    }

    fn stop(&self, reason: String) {
        self.reason
            .lock()
            .expect("kill reason lock")
            .get_or_insert(reason);
        self.terminate();
    }

    fn terminate(&self) {
        if self.reaped.load(Ordering::Acquire) {
            return;
        }
        let mut tree = self.tree.lock().expect("process tree lock");
        if let Some(tree) = tree.as_mut() {
            tree.terminate_group(self.pid);
        }
    }
}

/// Starts `command` and streams what it does.
///
/// The same rules as [`run_bounded`] for what runs: the executable is resolved
/// on the host PATH, the environment is empty, and the child leads a process
/// group (a Job Object on Windows) so stopping it stops its descendants too.
/// What differs is time: output is delivered as it arrives, there is no
/// deadline unless one is asked for, and stdin can be written.
pub(crate) fn spawn_streaming(
    command: &str,
    args: &[String],
    options: SpawnOptions,
) -> Result<SpawnedProcess, String> {
    let executable = resolve_executable(command, std::env::var_os("PATH").as_deref())?;
    let mut builder = Command::new(&executable);
    if let Some(cwd) = &options.cwd {
        builder.current_dir(cwd);
    }
    builder
        // As for `run_bounded`: a grant authorizes one executable, not the
        // host's ambient credentials.
        .env_clear()
        .args(args)
        .stdin(if options.stdin {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_process_tree(&mut builder);
    let mut tree = ProcessTree::new()?;
    let mut child = builder
        .spawn()
        .map_err(|error| format!("running `{command}` failed: {error}"))?;
    if let Err(error) = tree.attach(&child) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!(
            "isolating `{command}` process tree failed: {error}"
        ));
    }

    let pid = child.id();
    let killer = Killer {
        pid,
        tree: Arc::new(std::sync::Mutex::new(Some(tree))),
        killed: Arc::new(AtomicBool::new(false)),
        reason: Arc::new(std::sync::Mutex::new(None)),
        reaped: Arc::new(AtomicBool::new(false)),
    };
    let (sender, events) = async_channel::unbounded();

    let stdin = child.stdin.take().map(|mut pipe| {
        let (writer, messages) = mpsc::channel::<StdinMessage>();
        thread::spawn(move || {
            use std::io::Write as _;
            for message in messages {
                match message {
                    StdinMessage::Write(text) => {
                        if pipe
                            .write_all(text.as_bytes())
                            .and_then(|()| pipe.flush())
                            .is_err()
                        {
                            break;
                        }
                    }
                    StdinMessage::End => break,
                }
            }
            // Dropping the pipe is what closes it.
        });
        StdinWriter {
            sender: Some(writer),
        }
    });

    let readers = [
        stream_reader(
            child.stdout.take().expect("piped stdout"),
            ProcessEvent::Stdout,
            options.lines,
            sender.clone(),
            killer.clone(),
        ),
        stream_reader(
            child.stderr.take().expect("piped stderr"),
            ProcessEvent::Stderr,
            options.lines,
            sender.clone(),
            killer.clone(),
        ),
    ];

    let waiter = killer.clone();
    let command = command.to_owned();
    thread::spawn(move || {
        use wait_timeout::ChildExt as _;

        let deadline = options
            .timeout
            .map(|timeout| (Instant::now() + timeout, timeout));
        let status = loop {
            match child.wait_timeout(WAIT_SLICE) {
                Ok(Some(status)) => break Ok(status),
                Ok(None) => {}
                Err(error) => break Err(format!("waiting for `{command}` failed: {error}")),
            }
            if let Some((deadline, timeout)) = deadline
                && Instant::now() >= deadline
            {
                waiter.stop(format!(
                    "`{command}` timed out after {} ms",
                    timeout.as_millis()
                ));
            }
        };

        // The leader is gone, but a descendant still holding the pipes would
        // keep the readers — and so the exit — waiting forever. Nothing the
        // child started outlives it.
        waiter.terminate();
        if status.is_err() {
            let _ = child.kill();
            let _ = child.wait();
        }
        waiter.reaped.store(true, Ordering::Release);
        for reader in readers {
            let _ = reader.join();
        }
        if let Some(mut tree) = waiter.tree.lock().expect("process tree lock").take() {
            tree.close_after_exit();
        }

        let reason = waiter.reason.lock().expect("kill reason lock").take();
        let event = match status {
            Ok(status) => ProcessEvent::Exit {
                code: status.code(),
                signal: exit_signal(&status),
                error: reason,
            },
            Err(error) => ProcessEvent::Exit {
                code: None,
                signal: None,
                error: Some(reason.unwrap_or(error)),
            },
        };
        let _ = sender.send_blocking(event);
    });

    Ok(SpawnedProcess {
        pid,
        events,
        stdin,
        killer,
    })
}

#[cfg(unix)]
fn exit_signal(status: &std::process::ExitStatus) -> Option<String> {
    use std::os::unix::process::ExitStatusExt as _;
    status.signal().map(|signal| {
        match signal {
            libc::SIGKILL => "SIGKILL",
            libc::SIGTERM => "SIGTERM",
            libc::SIGINT => "SIGINT",
            libc::SIGHUP => "SIGHUP",
            libc::SIGQUIT => "SIGQUIT",
            libc::SIGABRT => "SIGABRT",
            libc::SIGSEGV => "SIGSEGV",
            libc::SIGPIPE => "SIGPIPE",
            _ => return format!("signal {signal}"),
        }
        .to_owned()
    })
}

#[cfg(not(unix))]
fn exit_signal(_: &std::process::ExitStatus) -> Option<String> {
    None
}

/// Reads one stream to its end and sends what it said.
fn stream_reader(
    mut stream: impl Read + Send + 'static,
    event: fn(String) -> ProcessEvent,
    lines: bool,
    sender: async_channel::Sender<ProcessEvent>,
    killer: Killer,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut decoder = Utf8Decoder::default();
        let mut splitter = lines.then(LineSplitter::default);
        let mut chunk = vec![0_u8; 64 * 1024];
        let deliver = |text: String| sender.send_blocking(event(text)).is_ok();

        loop {
            let count = match stream.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(count) => count,
            };
            let text = decoder.decode(&chunk[..count]);
            let open = match splitter.as_mut() {
                None => text.is_empty() || deliver(text),
                Some(splitter) => match splitter.push(&text) {
                    Ok(lines) => lines.into_iter().all(deliver),
                    Err(error) => {
                        killer.stop(error);
                        break;
                    }
                },
            };
            if !open {
                // Nobody is listening any more; the child is being stopped.
                break;
            }
        }

        let rest = decoder.finish();
        match splitter.as_mut() {
            None => {
                if !rest.is_empty() {
                    deliver(rest);
                }
            }
            Some(splitter) => {
                if let Ok(lines) = splitter.push(&rest) {
                    lines.into_iter().for_each(|line| {
                        deliver(line);
                    });
                }
                if let Some(line) = splitter.finish() {
                    deliver(line);
                }
            }
        }
    })
}

/// UTF-8 decoding that survives a character split across two reads.
///
/// A read ends wherever the pipe's buffer did, which is often inside a
/// multi-byte character. The incomplete tail is held for the next read;
/// bytes that can never be valid become U+FFFD, as `TextDecoder` makes them.
#[derive(Default)]
struct Utf8Decoder {
    pending: Vec<u8>,
}

impl Utf8Decoder {
    fn decode(&mut self, bytes: &[u8]) -> String {
        self.pending.extend_from_slice(bytes);
        let mut text = String::new();
        let mut rest: &[u8] = &self.pending;
        loop {
            match std::str::from_utf8(rest) {
                Ok(valid) => {
                    text.push_str(valid);
                    rest = &[];
                    break;
                }
                Err(error) => {
                    let (valid, after) = rest.split_at(error.valid_up_to());
                    text.push_str(std::str::from_utf8(valid).expect("validated prefix"));
                    match error.error_len() {
                        Some(invalid) => {
                            text.push(char::REPLACEMENT_CHARACTER);
                            rest = &after[invalid..];
                        }
                        // An incomplete character at the end: wait for more.
                        None => {
                            rest = after;
                            break;
                        }
                    }
                }
            }
        }
        self.pending = rest.to_vec();
        text
    }

    /// Whatever is left once the stream has ended can never be completed.
    fn finish(&mut self) -> String {
        let rest = String::from_utf8_lossy(&self.pending).into_owned();
        self.pending.clear();
        rest
    }
}

/// Splits decoded text into lines, joining a line that spans several reads.
#[derive(Default)]
struct LineSplitter {
    partial: String,
}

impl LineSplitter {
    /// The lines `text` completed, without their line endings.
    fn push(&mut self, text: &str) -> Result<Vec<String>, String> {
        let mut lines = Vec::new();
        let mut rest = text;
        while let Some(newline) = rest.find('\n') {
            self.partial.push_str(&rest[..newline]);
            let mut line = std::mem::take(&mut self.partial);
            if line.ends_with('\r') {
                line.pop();
            }
            lines.push(line);
            rest = &rest[newline + 1..];
        }
        self.partial.push_str(rest);
        if self.partial.len() > MAX_LINE_BYTES {
            return Err(format!(
                "a line of output exceeded {} MiB without a newline",
                MAX_LINE_BYTES >> 20
            ));
        }
        Ok(lines)
    }

    /// The last line, when the stream ended without a newline.
    fn finish(&mut self) -> Option<String> {
        (!self.partial.is_empty()).then(|| std::mem::take(&mut self.partial))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[cfg(unix)]
    use std::ffi::OsStr;

    #[cfg(unix)]
    #[test]
    fn resolves_a_bare_command_from_the_host_path_before_clearing_the_environment() {
        let directory =
            std::env::temp_dir().join(format!("gpui-shell-command-path-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("test directory");
        let executable = directory.join("outside-default-path");
        std::fs::write(&executable, "#!/bin/sh\nprintf resolved").expect("test executable");
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))
            .expect("executable permissions");

        assert_eq!(
            resolve_executable("outside-default-path", Some(directory.as_os_str()))
                .expect("command on PATH"),
            executable
        );
        assert!(resolve_executable("missing", Some(OsStr::new("/not/here"))).is_err());

        let _ = std::fs::remove_dir_all(directory);
    }

    #[cfg(unix)]
    #[test]
    fn captures_a_successful_command() {
        let result = run_bounded(
            "/bin/sh",
            &["-c".into(), "printf out; printf err >&2".into()],
            None,
            Limits::for_test(Duration::from_secs(2), 1024),
            Cancellation::new(),
        )
        .expect("command");
        assert_eq!(result.code, 0);
        assert_eq!(result.stdout, "out");
        assert_eq!(result.stderr, "err");
    }

    #[cfg(unix)]
    #[test]
    fn child_does_not_inherit_the_host_environment() {
        let result = run_bounded(
            "/usr/bin/env",
            &[],
            None,
            Limits::for_test(Duration::from_secs(2), 1024),
            Cancellation::new(),
        )
        .expect("environment probe");

        assert_eq!(result.stdout, "");
        assert_eq!(result.stderr, "");
    }

    #[cfg(unix)]
    #[test]
    fn kills_a_command_that_times_out() {
        let error = run_bounded(
            "/bin/sh",
            &["-c".into(), "sleep 5".into()],
            None,
            Limits::for_test(Duration::from_millis(30), 1024),
            Cancellation::new(),
        )
        .expect_err("timeout");
        assert!(error.contains("timed out"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn a_descendant_inheriting_the_pipes_cannot_extend_the_timeout() {
        let started = Instant::now();
        let error = run_bounded(
            "/bin/sh",
            &["-c".into(), "(sleep 5) & exit 0".into()],
            None,
            Limits::for_test(Duration::from_millis(50), 1024),
            Cancellation::new(),
        )
        .expect_err("the inherited pipes must remain under the deadline");
        assert!(error.contains("timed out"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[cfg(unix)]
    #[test]
    fn kills_a_command_whose_stdout_exceeds_the_limit() {
        let error = run_bounded(
            "/bin/sh",
            &["-c".into(), "yes x | head -c 4096".into()],
            None,
            Limits::for_test(Duration::from_secs(2), 128),
            Cancellation::new(),
        )
        .expect_err("limit");
        assert!(error.contains("stdout exceeded"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn kills_a_command_whose_stderr_exceeds_the_limit() {
        let error = run_bounded(
            "/bin/sh",
            &["-c".into(), "yes x | head -c 4096 >&2".into()],
            None,
            Limits::for_test(Duration::from_secs(2), 128),
            Cancellation::new(),
        )
        .expect_err("limit");
        assert!(error.contains("stderr exceeded"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn cancellation_kills_and_reaps_the_command() {
        let cancellation = Cancellation::new();
        cancellation.cancel();
        let started = std::time::Instant::now();
        let error = run_bounded(
            "/bin/sh",
            &["-c".into(), "sleep 5".into()],
            None,
            Limits::for_test(Duration::from_secs(2), 1024),
            cancellation,
        )
        .expect_err("cancelled");
        assert!(error.contains("cancelled"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[cfg(windows)]
    #[test]
    fn windows_successful_command_is_resumed_after_job_assignment() {
        let result = run_bounded(
            "cmd.exe",
            &["/D".into(), "/C".into(), "echo out & echo err 1>&2".into()],
            None,
            Limits::for_test(Duration::from_secs(2), 1024),
            Cancellation::new(),
        )
        .expect("the suspended command must resume");

        assert_eq!(result.code, 0);
        assert_eq!(result.stdout.trim(), "out");
        assert_eq!(result.stderr.trim(), "err");
    }

    #[cfg(windows)]
    #[test]
    fn windows_timeout_terminates_the_job() {
        let started = Instant::now();
        let error = run_bounded(
            "cmd.exe",
            &[
                "/D".into(),
                "/C".into(),
                "ping.exe -n 20 127.0.0.1 >NUL".into(),
            ],
            None,
            Limits::for_test(Duration::from_millis(100), 1024),
            Cancellation::new(),
        )
        .expect_err("timeout");

        assert!(error.contains("timed out"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[cfg(windows)]
    #[test]
    fn windows_cancellation_terminates_the_job() {
        let cancellation = Cancellation::new();
        cancellation.cancel();
        let started = Instant::now();
        let error = run_bounded(
            "cmd.exe",
            &[
                "/D".into(),
                "/C".into(),
                "ping.exe -n 20 127.0.0.1 >NUL".into(),
            ],
            None,
            Limits::for_test(Duration::from_secs(2), 1024),
            cancellation,
        )
        .expect_err("cancelled");

        assert!(error.contains("cancelled"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[cfg(windows)]
    #[test]
    fn windows_timeout_kills_descendants() {
        let directory = std::env::temp_dir().join(format!(
            "gpui-shell-job-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let child_script = directory.join("child.cmd");
        let parent_script = directory.join("parent.cmd");
        let marker = directory.join("descendant-survived");
        std::fs::write(
            &child_script,
            format!(
                "@ping.exe -n 3 127.0.0.1 >NUL\r\n@echo survived>\"{}\"\r\n",
                marker.display()
            ),
        )
        .unwrap();
        std::fs::write(
            &parent_script,
            format!(
                "@start \"\" /B cmd.exe /D /C call \"{}\"\r\n@ping.exe -n 20 127.0.0.1 >NUL\r\n",
                child_script.display()
            ),
        )
        .unwrap();

        let error = run_bounded(
            "cmd.exe",
            &[
                "/D".into(),
                "/C".into(),
                parent_script.into_os_string().into_string().unwrap(),
            ],
            None,
            Limits::for_test(Duration::from_millis(100), 1024),
            Cancellation::new(),
        )
        .expect_err("timeout");
        assert!(error.contains("timed out"), "{error}");
        std::thread::sleep(Duration::from_secs(3));
        assert!(
            !marker.exists(),
            "a descendant escaped the Windows Job Object"
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[cfg(unix)]
    /// Every event until the exit, failing the test instead of hanging.
    fn collect(spawned: &SpawnedProcess) -> Vec<ProcessEvent> {
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut events = Vec::new();
        loop {
            match spawned.events.try_recv() {
                Ok(event) => {
                    let last = event.is_exit();
                    events.push(event);
                    if last {
                        // Nothing may follow the exit.
                        thread::sleep(Duration::from_millis(50));
                        assert!(
                            spawned.events.try_recv().is_err(),
                            "an event after the exit"
                        );
                        return events;
                    }
                }
                Err(_) => {
                    assert!(Instant::now() < deadline, "no exit within 10 s: {events:?}");
                    thread::sleep(Duration::from_millis(5));
                }
            }
        }
    }

    #[cfg(unix)]
    fn stdout(events: &[ProcessEvent]) -> Vec<String> {
        events
            .iter()
            .filter_map(|event| match event {
                ProcessEvent::Stdout(text) => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    #[cfg(unix)]
    fn exit(events: &[ProcessEvent]) -> (Option<i32>, Option<String>, Option<String>) {
        match events.last() {
            Some(ProcessEvent::Exit {
                code,
                signal,
                error,
            }) => (*code, signal.clone(), error.clone()),
            other => panic!("the last event must be the exit, found {other:?}"),
        }
    }

    #[cfg(unix)]
    fn shell(script: &str, options: SpawnOptions) -> SpawnedProcess {
        spawn_streaming("/bin/sh", &["-c".into(), script.into()], options).expect("spawn")
    }

    #[cfg(unix)]
    fn lines() -> SpawnOptions {
        SpawnOptions {
            lines: true,
            ..Default::default()
        }
    }

    #[cfg(unix)]
    #[test]
    fn lines_are_joined_across_writes_and_the_last_partial_line_is_delivered() {
        let spawned = shell(
            "printf 'a'; sleep 0.05; printf 'b\\nc'; sleep 0.05; printf '\\r\\nd'",
            lines(),
        );
        let events = collect(&spawned);
        assert_eq!(stdout(&events), ["ab", "c", "d"]);
        assert_eq!(exit(&events).0, Some(0));
    }

    #[cfg(unix)]
    #[test]
    fn a_character_split_between_writes_decodes_whole() {
        // 中 is E4 B8 AD and 😀 is F0 9F 98 80; each write stops inside one.
        let spawned = shell(
            "printf '\\344\\270'; sleep 0.05; printf '\\255\\360\\237\\230'; sleep 0.05; printf '\\200\\n'",
            lines(),
        );
        assert_eq!(stdout(&collect(&spawned)), ["中😀"]);

        let mut decoder = Utf8Decoder::default();
        let text = "中😀";
        let decoded: String = text
            .as_bytes()
            .iter()
            .map(|byte| decoder.decode(std::slice::from_ref(byte)))
            .collect();
        assert_eq!(decoded + &decoder.finish(), text);
        assert_eq!(Utf8Decoder::default().decode(b"a\xffb"), "a\u{FFFD}b");
    }

    #[cfg(unix)]
    #[test]
    fn stdin_round_trips_through_cat() {
        let mut spawned = spawn_streaming(
            "cat",
            &[],
            SpawnOptions {
                stdin: true,
                lines: true,
                ..Default::default()
            },
        )
        .expect("spawn cat");
        let stdin = spawned.stdin.as_mut().expect("piped stdin");
        stdin.write("first\n".into()).unwrap();
        stdin.write("second\n".into()).unwrap();
        stdin.end();
        assert!(
            stdin.write("late\n".into()).is_err(),
            "writing after end is refused"
        );

        let events = collect(&spawned);
        assert_eq!(stdout(&events), ["first", "second"]);
        assert_eq!(exit(&events).0, Some(0));
    }

    #[cfg(unix)]
    #[test]
    fn kill_stops_the_child_and_its_descendants_with_one_exit() {
        let spawned = shell("sleep 30 & echo $!; sleep 30", lines());
        let descendant = loop {
            match spawned
                .events
                .recv_blocking()
                .expect("the descendant's pid")
            {
                ProcessEvent::Stdout(pid) => break pid.parse::<i32>().unwrap(),
                _ => continue,
            }
        };

        spawned.killer.kill();
        spawned.killer.kill();
        let events = collect(&spawned);
        let (code, signal, error) = exit(&events);
        assert_eq!(code, None);
        assert_eq!(signal.as_deref(), Some("SIGKILL"));
        assert_eq!(error, None, "an asked-for kill is not an error");
        assert_eq!(events.iter().filter(|event| event.is_exit()).count(), 1);

        // The descendant was in the group; it is gone, not orphaned.
        let deadline = Instant::now() + Duration::from_secs(2);
        while unsafe { libc::kill(descendant, 0) } == 0 {
            assert!(
                Instant::now() < deadline,
                "the descendant {descendant} survived"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_timeout_stops_the_child_and_says_so() {
        let spawned = shell(
            "sleep 5",
            SpawnOptions {
                timeout: Some(Duration::from_millis(100)),
                ..Default::default()
            },
        );
        let (_, signal, error) = exit(&collect(&spawned));
        assert_eq!(signal.as_deref(), Some("SIGKILL"));
        assert!(error.is_some_and(|error| error.contains("timed out after 100 ms")));
    }

    /// `process.run`'s 30 seconds are not `spawn`'s: with no `timeoutMs` there
    /// is no deadline at all, which the waiting loop only ever checks against
    /// `options.timeout`.
    #[cfg(unix)]
    #[test]
    fn a_spawn_without_a_timeout_has_no_deadline() {
        assert_eq!(SpawnOptions::default().timeout, None);
        let spawned = shell("sleep 0.3; echo done", lines());
        let events = collect(&spawned);
        assert_eq!(stdout(&events), ["done"]);
        assert_eq!(exit(&events), (Some(0), None, None));
    }

    #[cfg(unix)]
    #[test]
    fn an_endless_line_stops_the_child() {
        let mut splitter = LineSplitter::default();
        assert!(splitter.push(&"x".repeat(MAX_LINE_BYTES)).is_ok());
        assert!(splitter.push("x").is_err(), "one byte past the limit");
        assert_eq!(
            LineSplitter::default().push("a\nb").unwrap(),
            ["a"],
            "an unfinished line waits"
        );
    }

    #[cfg(unix)]
    #[test]
    fn run_honours_a_working_directory_and_a_longer_timeout() {
        let directory = std::env::temp_dir();
        let result = run_bounded(
            "/bin/sh",
            &["-c".into(), "pwd".into()],
            Some(&directory),
            Limits::default().with_timeout(Duration::from_secs(5)),
            Cancellation::new(),
        )
        .expect("pwd");
        assert_eq!(
            std::fs::canonicalize(result.stdout.trim()).unwrap(),
            std::fs::canonicalize(&directory).unwrap()
        );
    }
}
