// Copyright (c) 2020-2025 MicroDoc Software GmbH.
// See the "LICENSE.txt" file at the top-level directory of this distribution.
//
// Licensed under the MIT license. This file may not be copied, modified,
// or distributed except according to those terms.

/*! Error reporting. */

use alloc::sync::Arc;
use core::fmt;
use std::io;
use std::os::raw::c_int;
use std::sync::Mutex;

/// A result of a fallible operation.
pub(crate) type Result<T> = core::result::Result<T, Error>;

/// Actual storage for an error.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Virtual memory address range contains too many pages.
    TooManyVMPages,

    /// Some [`io::Error`](std::io::Error) occurred.
    #[non_exhaustive]
    Io {
        /// Name of the I/O operation that generated the error.
        operation: &'static str,
        /// The [`io::Error`](std::io::Error) that occurred.
        error: Arc<io::Error>,
        /// Identifier of the process that was the target of the I/O.
        process_id: Option<libc::pid_t>,
    },

    /// Casting an integer caused data loss.
    #[non_exhaustive]
    IntegerCast(core::num::TryFromIntError),
}

/// Call stack back trace where the `Error` object was created.
struct ErrorBackTrace {
    backtrace: backtrace::Backtrace,
    resolved: bool,
}

impl ErrorBackTrace {
    /// Resolve the call stack back trace to resolve all addresses
    /// to their symbolic names.
    fn resolve(&mut self) {
        if !self.resolved {
            self.resolved = true;
            self.backtrace.resolve();
        }
    }
}

impl fmt::Debug for ErrorBackTrace {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.backtrace, f)
    }
}

/// Data describing an `Error` that occurred.
#[derive(Clone)]
struct ErrorData {
    kind: ErrorKind,
    backtrace: Arc<Mutex<ErrorBackTrace>>,
}

impl fmt::Debug for ErrorData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut back_trace_lock = self.backtrace.lock().unwrap();
        back_trace_lock.resolve();
        write!(f, "Error: {:?}. At:\n{:?}", self.kind, *back_trace_lock)
    }
}

/// An error is a pointer that allocates when an error happens.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Error(Box<ErrorData>);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.0.kind {
            ErrorKind::TooManyVMPages => {
                write!(f, "virtual memory address range contains too many pages")
            }
            ErrorKind::Io {
                operation,
                error,
                process_id,
            } => match process_id {
                None => write!(f, "{operation}: {error}"),
                Some(process_id) => write!(f, "{operation}({process_id}): {error}"),
            },
            ErrorKind::IntegerCast(err) => err.fmt(f),
        }
    }
}

impl core::error::Error for Error {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match &self.0.kind {
            // Errors that are self-descriptive.
            ErrorKind::TooManyVMPages | ErrorKind::Io { .. } => None,

            // Errors that defer description to the inner error.
            ErrorKind::IntegerCast(err) => Some(err),
        }
    }
}

/// Convert an `ErrorKind` into an `Error`.
impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        let backtrace = backtrace::Backtrace::new_unresolved();

        Self(Box::new(ErrorData {
            kind,
            backtrace: Arc::new(Mutex::new(ErrorBackTrace {
                backtrace,
                resolved: false,
            })),
        }))
    }
}

/// Wrap another error into an instance of `Error`.
impl From<core::num::TryFromIntError> for Error {
    fn from(err: core::num::TryFromIntError) -> Self {
        Self::from(ErrorKind::IntegerCast(err))
    }
}

impl Error {
    /// Wrap an `io::Error` into an instance of `Error`, with an associated process ID.
    pub(crate) fn from_io3(
        error: io::Error,
        operation: &'static str,
        process_id: libc::pid_t,
    ) -> Self {
        ErrorKind::Io {
            operation,
            error: Arc::new(error),
            process_id: Some(process_id),
        }
        .into()
    }

    /// Returns the actual kind of this error.
    #[must_use]
    pub fn kind(&self) -> &ErrorKind {
        &self.0.kind
    }

    /// Returns the errno code for a given `Error`, if such a code has been
    /// reported by the operating system.
    #[must_use]
    pub fn os_error_code(&self) -> Option<c_int> {
        match &self.0.kind {
            ErrorKind::TooManyVMPages { .. } | ErrorKind::IntegerCast { .. } => None,
            ErrorKind::Io { error, .. } => error.raw_os_error(),
        }
    }
}
