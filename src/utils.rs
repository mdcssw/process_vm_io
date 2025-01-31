#[cfg(test)]
mod tests;

use core::num::NonZero;
use core::slice;
use core::sync::atomic::{AtomicU64, Ordering};
use std::io::{self, IoSlice, IoSliceMut};
use std::sync::atomic::AtomicUsize;

use crate::errors::{Error, ErrorKind, Result};

/// Align a given number down to a specified alignment boundary.
pub(crate) const fn align_down(n: u64, alignment: NonZero<u64>) -> u64 {
    // Notice that the calculation below never causes an overflow.
    n & !alignment.get().saturating_sub(1)
}

pub(crate) fn io_vectors_from_io_slices<'slices>(
    bufs: &'slices [IoSlice],
) -> (u64, &'slices [libc::iovec]) {
    let byte_count = bufs.iter().map(|buf| buf.len() as u64).sum();
    let local_io_vectors = unsafe { slice::from_raw_parts(bufs.as_ptr().cast(), bufs.len()) };
    (byte_count, local_io_vectors)
}

pub(crate) fn io_vectors_from_io_slices_mut<'slices>(
    bufs: &'slices mut [IoSliceMut],
) -> (u64, &'slices [libc::iovec]) {
    let byte_count = bufs.iter().map(|buf| buf.len() as u64).sum();
    let local_io_vectors = unsafe { slice::from_raw_parts(bufs.as_ptr().cast(), bufs.len()) };
    (byte_count, local_io_vectors)
}

/// Ensure that the process, identified by the given process identifier,
/// currently exists in the system.
pub(crate) fn ensure_process_exists(process_id: libc::pid_t) -> Result<()> {
    if unsafe { libc::kill(process_id, 0) } != -1 {
        return Ok(());
    }

    let mut err = io::Error::last_os_error();
    err = match err.raw_os_error() {
        Some(libc::ESRCH) => io::Error::from(io::ErrorKind::NotFound),
        Some(libc::EINVAL) => io::Error::from(io::ErrorKind::InvalidInput),
        Some(libc::EPERM) => io::Error::from(io::ErrorKind::PermissionDenied),
        _ => err,
    };
    Err(Error::from_io3(err, "kill", process_id))
}

fn get_min_system_page_size() -> Result<NonZero<u64>> {
    let value = match unsafe { libc::sysconf(libc::_SC_PAGE_SIZE) } {
        -1 => return Err(Error::from(ErrorKind::UnknownPageSize)),
        r => r as u64,
    };

    if let Some(value) = NonZero::new(value) {
        if value.is_power_of_two() {
            return Ok(value);
        }
    }

    Err(Error::from(ErrorKind::InvalidPageSize(value)))
}

/// Size in bytes of the smallest possible virtual memory page.
pub(crate) fn min_system_page_size() -> Result<NonZero<u64>> {
    static VALUE: AtomicU64 = AtomicU64::new(0);

    if let Some(value) = NonZero::new(VALUE.load(Ordering::Acquire)) {
        return Ok(value);
    }

    // Initialize global VALUE.
    let value = get_min_system_page_size()?;

    loop {
        match VALUE.compare_exchange_weak(0, value.get(), Ordering::Release, Ordering::Acquire) {
            Ok(_) => break Ok(value), // Initialized in this thread.

            Err(previous_value) => {
                if let Some(previous_value) = NonZero::new(previous_value) {
                    break Ok(previous_value); // Initialized in another thread.
                }
                // Otherwise, spurious wake. Loop again.
            }
        }
    }
}

fn get_system_iov_max() -> NonZero<usize> {
    let value = match unsafe { libc::sysconf(libc::_SC_IOV_MAX) } {
        -1 | 0 => 1,
        r => r as usize,
    };

    unsafe { NonZero::new_unchecked(value) }
}

/// Maximum number of the `iovec` structures that can be provided to
/// one system call.
///
/// Failure to fetch the information will result in a count of `1`.
pub(crate) fn system_iov_max() -> NonZero<usize> {
    static VALUE: AtomicUsize = AtomicUsize::new(0);

    if let Some(value) = NonZero::new(VALUE.load(Ordering::Acquire)) {
        return value;
    }

    // Initialize global VALUE.
    let value = get_system_iov_max();

    loop {
        match VALUE.compare_exchange_weak(0, value.get(), Ordering::Release, Ordering::Acquire) {
            Ok(_) => break value, // Initialized in this thread.

            Err(previous_value) => {
                if let Some(previous_value) = NonZero::new(previous_value) {
                    break previous_value; // Initialized in another thread.
                }
                // Otherwise, spurious wake. Loop again.
            }
        }
    }
}
