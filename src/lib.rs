// Copyright (c) 2020-2025 MicroDoc Software GmbH.
// See the "LICENSE.txt" file at the top-level directory of this distribution.
//
// Licensed under the MIT license. This file may not be copied, modified,
// or distributed except according to those terms.

// TODO: https://rust-lang.github.io/api-guidelines/checklist.html

#![doc = include_str!("../README.md")]
#![doc(html_root_url = "https://docs.rs/process_vm_io/1.0.12")]
#![warn(
    unsafe_op_in_unsafe_fn,
    missing_docs,
    keyword_idents,
    macro_use_extern_crate,
    missing_debug_implementations,
    non_ascii_idents,
    trivial_casts,
    trivial_numeric_casts,
    unstable_features,
    unused_extern_crates,
    unused_import_braces,
    unused_labels,
    variant_size_differences,
    unused_qualifications
)]
//#![warn(clippy::pedantic, clippy::restriction)]
#![allow(
    clippy::alloc_instead_of_core,
    clippy::std_instead_of_core,
    clippy::std_instead_of_alloc,
    clippy::upper_case_acronyms,
    clippy::arbitrary_source_item_ordering,
    clippy::single_call_fn,
    clippy::pub_use,
    clippy::missing_docs_in_private_items,
    clippy::implicit_return,
    clippy::unwrap_used,
    clippy::separated_literal_suffix,
    clippy::absolute_paths,
    clippy::assertions_on_result_states,
    clippy::undocumented_unsafe_blocks,
    clippy::unwrap_in_result,
    clippy::missing_inline_in_public_items,
    clippy::missing_trait_methods,
    clippy::question_mark_used,
    clippy::else_if_without_else,
    clippy::shadow_reuse,
    clippy::default_numeric_fallback
)]

mod errors;
#[cfg(test)]
mod tests;
mod utils;

extern crate alloc;

use errors::Result;
pub use errors::{Error, ErrorKind};

use core::cmp;
use core::ffi::c_void;
use std::io::{IoSlice, IoSliceMut, Read, Seek, SeekFrom, Write};
use std::os::raw::c_ulong;
use std::{io, panic};

use smallvec::SmallVec;

use crate::utils::{
    align_down, ensure_process_exists, io_vectors_from_io_slices, io_vectors_from_io_slices_mut,
    min_system_page_size, system_iov_max,
};

/// Prototype of the APIs `process_vm_readv()` and `process_vm_writev()`.
type ProcessVMReadVProc = unsafe extern "C" fn(
    libc::pid_t,
    *const libc::iovec,
    c_ulong,
    *const libc::iovec,
    c_ulong,
    c_ulong,
) -> isize;

/// An address range which is split, at page boundaries, over multiple sections.
///
/// The address range spans as follows:
/// 1. A possibly empty part of a first page, then
/// 2. Zero or more full pages, then
/// 3. A possibly empty part of a last page.
#[derive(Debug, PartialEq, Eq, Default)]
struct PageAwareAddressRange {
    start_address: u64,
    size_in_first_page: u64, // end_of_first_page = start_address + size_in_first_page
    size_of_inner_pages: u64, // start_of_last_page = end_of_first_page + size_of_inner_pages
    size_in_last_page: u64,  // end_address = start_of_last_page + size_in_last_page
}

impl PageAwareAddressRange {
    /// Convert a plain address range into an address range which is split,
    /// at page boundaries, over multiple sections.
    fn new(start_address: u64, mut size: u64) -> Result<Self> {
        if size == 0 {
            return Ok(Self {
                start_address,
                size_in_first_page: 0,
                size_of_inner_pages: 0,
                size_in_last_page: 0,
            });
        }

        let min_page_size = min_system_page_size()?;
        let distance_to_preceeding_page_boundary =
            start_address - align_down(start_address, min_page_size);

        let inside_one_page = (size <= min_page_size.get())
            && ((distance_to_preceeding_page_boundary + size) <= min_page_size.get());

        if inside_one_page {
            //             | -- distance_to_preceeding_page_boundary -- v ---- size ---- v                  |
            // preceeding_page_boundary           -->             start_address --> end_address --> next_page_boundary
            return if distance_to_preceeding_page_boundary == 0 && size == min_page_size.get() {
                Ok(Self {
                    start_address,
                    size_in_first_page: 0,
                    size_of_inner_pages: size,
                    size_in_last_page: 0,
                })
            } else {
                Ok(Self {
                    start_address,
                    size_in_first_page: size,
                    size_of_inner_pages: 0,
                    size_in_last_page: 0,
                })
            };
        }

        //                                                          v --------------- size --------------- v
        // preceeding_page_boundary            -->           start_address     -->    next_page_boundary --> end_address
        //             | -- distance_to_preceeding_page_boundary -- ^ -- size_in_first_page -- |

        let size_in_first_page = if distance_to_preceeding_page_boundary == 0 {
            0
        } else {
            min_page_size.get() - distance_to_preceeding_page_boundary
        };

        size -= size_in_first_page;

        //                                                         v ----- size ----- v
        // preceeding_page_boundary --> start_address --> next_page_boundary --> end_address
        //             |                                           |

        let size_of_inner_pages = align_down(size, min_page_size);
        let size_in_last_page = size - size_of_inner_pages;

        Ok(Self {
            start_address,
            size_in_first_page,
            size_of_inner_pages,
            size_in_last_page,
        })
    }

    /// Transform this address range into a vector of `iovec`s.
    /// Each returned `iovec` describes a page (or a part of a page).
    ///
    /// Due to system/memory limits, some suffix of this address range might not
    /// be included in the returned vector of `iovec`s. The size of that suffix
    /// (if any) is also returned. Returning a vector of `iovec`s that covers
    /// only a prefix of this address range is not considered a failure.
    fn into_iov_buffers(mut self) -> Result<(SmallVec<[libc::iovec; 3]>, u64)> {
        let min_page_size = min_system_page_size()?.get();
        let max_iov_count = system_iov_max().get();
        let mut size_of_not_covered_suffix = 0;

        let mut inner_pages_count = usize::try_from(self.size_of_inner_pages / min_page_size)?;
        let mut total_count = usize::from(self.size_in_first_page != 0)
            + usize::from(self.size_in_last_page != 0)
            + inner_pages_count;

        let mut count = {
            if total_count > max_iov_count && self.size_in_last_page != 0 {
                // Skip the last page.
                self.size_in_last_page = 0;
                size_of_not_covered_suffix += self.size_in_last_page;
                total_count -= 1;
            }

            if total_count > max_iov_count && self.size_of_inner_pages != 0 {
                // Skip as many inner pages as necessary.
                let excess_pages = total_count - max_iov_count;
                let excess_size = min_page_size * excess_pages as u64;
                self.size_of_inner_pages -= excess_size;
                size_of_not_covered_suffix += excess_size;
                inner_pages_count -= excess_pages;
                total_count -= excess_pages;
            }
            total_count
        };

        let mut result = loop {
            if count <= 3 {
                // This allocates on the stack. It should never fail.
                break SmallVec::with_capacity(count);
            }

            match panic::catch_unwind(|| SmallVec::with_capacity(count)) {
                Ok(result) => {
                    // Successfully allocated on the heap.
                    break result;
                }

                Err(_err) => {
                    if self.size_in_last_page != 0 {
                        // Retry without allocating the `iovec` of the last page.
                        self.size_in_last_page = 0;
                        size_of_not_covered_suffix += self.size_in_last_page;
                        count -= 1;
                    } else if inner_pages_count >= 2 {
                        // Retry without allocating half the `iovec`s of the inner pages.
                        let half_inner_pages = inner_pages_count / 2;
                        let size_of_half_inner_pages = min_page_size * half_inner_pages as u64;
                        self.size_of_inner_pages -= size_of_half_inner_pages;
                        size_of_not_covered_suffix += size_of_half_inner_pages;
                        inner_pages_count -= half_inner_pages;
                        count -= half_inner_pages;
                    }
                }
            }
        };

        if self.size_in_first_page != 0 {
            result.push(libc::iovec {
                iov_base: usize::try_from(self.start_address)? as *mut c_void,
                iov_len: usize::try_from(self.size_in_first_page)?,
            });
        }

        let mut page_address = self.start_address.wrapping_add(self.size_in_first_page);
        let start_of_last_page = page_address.wrapping_add(self.size_of_inner_pages);

        let mut remaining_size = self.size_of_inner_pages;
        while remaining_size != 0 {
            result.push(libc::iovec {
                iov_base: usize::try_from(page_address)? as *mut c_void,
                iov_len: usize::try_from(min_page_size)?,
            });
            remaining_size -= min_page_size;
            page_address = page_address.wrapping_add(min_page_size);
        }

        if self.size_in_last_page != 0 {
            result.push(libc::iovec {
                iov_base: usize::try_from(start_of_last_page)? as *mut c_void,
                iov_len: usize::try_from(self.size_in_last_page)?,
            });
        }
        Ok((result, size_of_not_covered_suffix))
    }
}

/// Input/Output object transferring data to/from the virtual memory contents
/// of a particular process.
///
/// For better performance, consider doing buffered I/O based on the standard
/// [`BufReader`](std::io::BufReader) and [`BufWriter`](std::io::BufWriter).
#[derive(Debug)]
#[non_exhaustive]
pub struct ProcessVirtualMemoryIO {
    process_id: libc::pid_t,

    /// Current virtual memory address where I/O happens in the target process.
    /// A value of `None` means we are **past** the end of the address space.
    address: Option<u64>,
}

impl ProcessVirtualMemoryIO {
    /// Create a new object to perform input/output of data from/to the virtual
    /// memory contents of the process identified by `process_id`.
    ///
    /// The initial virtual memory address where data transfers initially
    /// happen is specified by `initial_address`. Inquiring the system for the
    /// layout of the process address space can help specify this value.
    /// On Linux, the file `/proc/[process_id]/maps` can help with this.
    ///
    /// # Common errors
    ///
    /// If the specified process does not exist, or this process does not have
    /// the right to send signals to the specified process, then an error is
    /// returned.
    ///
    /// # Safety
    ///
    /// Writing to the virtual memory of a process is a potentially unsafe
    /// operation because it may introduce memory unsafety in that process,
    /// and may lead to unexpected states in that process. This is even more
    /// dangerous when the target process is the currently running process.
    ///
    /// # Running processes
    ///
    /// Apart from the process identifier, there is no strong link between the
    /// specified process and this [`ProcessVirtualMemoryIO`] instance.
    /// If the specified process runs and terminates, then further I/O
    /// operations involving this instance may fail.
    /// If the process identifier of the dead process is reused by another
    /// process, then I/O may successfully continue with the new process,
    /// which might not be the desired behavior.
    pub unsafe fn new(process_id: u32, initial_address: u64) -> Result<Self> {
        let process_id = process_id as libc::pid_t;
        if process_id <= 0 {
            return Err(Error::from_io3(
                io::ErrorKind::InvalidInput.into(),
                "process_vm_io::ProcessVirtualMemoryIO::new",
                process_id,
            ));
        }

        ensure_process_exists(process_id)?;

        Ok(Self {
            process_id,
            address: Some(initial_address),
        })
    }

    /// Return the process identifier of the target process.
    #[must_use]
    pub fn process_id(&self) -> u32 {
        self.process_id as u32
    }

    /// Perform vectored (i.e., scatter/gather) I/O on the virtual memory of the
    /// target process.
    fn io_vectored(
        &mut self,
        process_vm_io_v: ProcessVMReadVProc,
        process_vm_io_v_name: &'static str,
        local_io_vectors: &[libc::iovec],
        mut byte_count: u64,
    ) -> Result<usize> {
        if byte_count == 0 || self.address.is_none() {
            return Ok(0);
        }

        let address = self.address.unwrap();

        // Do not overflow the address space.
        let max_remaining_bytes = (u64::MAX - address).saturating_add(1);
        byte_count = cmp::min(byte_count, max_remaining_bytes);

        let (remote_io_vectors, _size_of_not_covered_suffix) =
            PageAwareAddressRange::new(address, byte_count)?.into_iov_buffers()?;

        let transferred_bytes_count = unsafe {
            process_vm_io_v(
                self.process_id,
                local_io_vectors.as_ptr(),
                local_io_vectors.len() as c_ulong,
                remote_io_vectors.as_ptr(),
                remote_io_vectors.len() as c_ulong,
                0,
            )
        };

        if transferred_bytes_count == -1 {
            return Err(Error::from_io3(
                io::Error::last_os_error(),
                process_vm_io_v_name,
                self.process_id,
            ));
        }

        self.address = ((transferred_bytes_count as u64) < max_remaining_bytes)
            .then_some(address + (transferred_bytes_count as u64));
        // If self.address is None, then we reached the end of address space.

        Ok(transferred_bytes_count as usize)
    }
}

impl Seek for ProcessVirtualMemoryIO {
    /// Seek to an address in the virtual memory address space of the associated
    /// process.
    ///
    /// If the seek operation completed successfully, this method returns
    /// the new position from the start of the stream.
    ///
    /// The target address need not be readable/writable or even mapped.
    /// Seeking beyond the address space size puts the cursor after the end
    /// of the address space.
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.address = match (self.address, pos) {
            (_, SeekFrom::Start(n)) => Some(n),

            (None, SeekFrom::Current(n)) if n >= 0 => None,
            (_, SeekFrom::End(n)) if n >= 0 => None,

            (Some(address), SeekFrom::Current(n)) if n >= 0 => {
                let forward = n as u64;
                address.checked_add(forward)
            }

            (None, SeekFrom::Current(n)) | (_, SeekFrom::End(n)) => {
                // n < 0
                let backward = n.wrapping_neg() as u64;
                Some((u64::MAX - backward) + 1)
            }

            (Some(address), SeekFrom::Current(n)) => {
                // n < 0
                let backward = n.wrapping_neg() as u64;
                address
                    .checked_sub(backward)
                    .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))
                    .map(Some)?
            }
        };

        Ok(self.address.unwrap_or(u64::MAX))
    }
}

impl Read for ProcessVirtualMemoryIO {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let local_io_vector = libc::iovec {
            iov_base: buf.as_mut_ptr().cast(),
            iov_len: buf.len(),
        };

        self.io_vectored(
            libc::process_vm_readv,
            "process_vm_readv",
            &[local_io_vector],
            buf.len() as u64,
        )
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut]) -> io::Result<usize> {
        let (byte_count, local_io_vectors) = io_vectors_from_io_slices_mut(bufs);

        self.io_vectored(
            libc::process_vm_readv,
            "process_vm_readv",
            local_io_vectors,
            byte_count,
        )
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
    }
}

impl Write for ProcessVirtualMemoryIO {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let local_io_vector = libc::iovec {
            iov_base: buf.as_ptr() as *mut c_void,
            iov_len: buf.len(),
        };

        self.io_vectored(
            libc::process_vm_writev,
            "process_vm_writev",
            &[local_io_vector],
            buf.len() as u64,
        )
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
    }

    fn write_vectored(&mut self, bufs: &[IoSlice]) -> io::Result<usize> {
        let (byte_count, local_io_vectors) = io_vectors_from_io_slices(bufs);

        self.io_vectored(
            libc::process_vm_writev,
            "process_vm_writev",
            local_io_vectors,
            byte_count,
        )
        .map_err(|err| io::Error::new(io::ErrorKind::Other, err))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
