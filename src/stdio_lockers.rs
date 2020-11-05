//! Hold locks for the process' stdin and stdout.

use once_cell::sync::Lazy;
use parking::{Parker, Unparker};
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, RawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawHandle, RawHandle};
use std::{
    io::{stdin, stdout, Stdin, StdinLock, Stdout},
    sync::atomic::{AtomicBool, Ordering::SeqCst},
    thread::{self, JoinHandle},
};

// Static handles to `stdin()` and `stdout()` so that we can reference
// them with `StdinLock` and `StdoutLock` with `'static` lifetime
// parameters.
static STDIN: Lazy<Stdin> = Lazy::new(stdin);
static STDOUT: Lazy<Stdout> = Lazy::new(stdout);

// Statically track whether `STDIN` and `STDOUT` are claimed.
static STDIN_CLAIMED: AtomicBool = AtomicBool::new(false);
static STDOUT_CLAIMED: AtomicBool = AtomicBool::new(false);

/// This class acquires a lock on `stdin` and prevents applications from
/// accidentally accessing it through other means.
pub(crate) struct StdinLocker {
    _lock: StdinLock<'static>,
}

/// This class acquires a lock on `stdout` and prevents applications from
/// accidentally accessing it through other means.
pub(crate) struct StdoutLocker {
    unparker: Unparker,
    handle: Option<JoinHandle<()>>,
}

impl StdinLocker {
    /// An `InputByteStream` can take the value of the process' stdin, in which
    /// case we want it to have exclusive access to `stdin`. Lock the Rust standard
    /// library's `stdin` to prevent accidental misuse.
    ///
    /// Return `None` if a `StdinLocker` instance already exists.
    pub(crate) fn new() -> Option<Self> {
        if !STDIN_CLAIMED.compare_and_swap(false, true, SeqCst) {
            Some(Self {
                _lock: STDIN.lock(),
            })
        } else {
            None
        }
    }
}

impl StdoutLocker {
    /// An `OutputByteStream` can take the value of the process' stdout, in which
    /// case we want it to have exclusive access to `stdout`. Lock the Rust standard
    /// library's `stdout` to prevent accidental misuse.
    ///
    /// Return `None` if a `StdoutLocker` instance already exists.
    pub(crate) fn new() -> Option<Self> {
        if !STDOUT_CLAIMED.compare_and_swap(false, true, SeqCst) {
            // Unlike `stdin`, `stdout` is locked with a reentrent mutex, so in
            // order to prevent other uses of it, create a thread and have it
            // acquire the lock and park.
            let parker = Parker::new();
            let unparker = parker.unparker();
            let handle = Some(
                thread::Builder::new()
                    .name("ensure exclusive access to stdout".to_owned())
                    .stack_size(64)
                    .spawn(move || {
                        let _lock = STDOUT.lock();
                        parker.park()
                    })
                    .ok()?,
            );

            Some(Self { unparker, handle })
        } else {
            None
        }
    }
}

impl Drop for StdinLocker {
    #[inline]
    fn drop(&mut self) {
        STDIN_CLAIMED.store(false, SeqCst);
    }
}

impl Drop for StdoutLocker {
    #[inline]
    fn drop(&mut self) {
        self.unparker.unpark();
        self.handle.take().unwrap().join().unwrap();
        STDOUT_CLAIMED.store(false, SeqCst);
    }
}

#[cfg(not(windows))]
impl AsRawFd for StdinLocker {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        STDIN.as_raw_fd()
    }
}

#[cfg(not(windows))]
impl AsRawFd for StdoutLocker {
    #[inline]
    fn as_raw_fd(&self) -> RawFd {
        STDOUT.as_raw_fd()
    }
}

#[cfg(windows)]
impl AsRawHandle for StdinLocker {
    #[inline]
    fn as_raw_handle(&self) -> RawHandle {
        STDIN.as_raw_handle()
    }
}

#[cfg(windows)]
impl AsRawHandle for StdoutLocker {
    #[inline]
    fn as_raw_handle(&self) -> RawHandle {
        STDOUT.as_raw_handle()
    }
}
