// Copyright (c) 2020 MicroDoc Software GmbH.
// See the "LICENSE.txt" file at the top-level directory of this distribution.
//
// Licensed under the the MIT license. This file may not be copied, modified,
// or distributed except according to those terms.

use assert_matches::assert_matches;

use super::*;

#[test]
fn sensible_virtual_memory_page_size() {
    let size = *MIN_SYSTEM_PAGE_SIZE;
    assert!(size > 0);
    assert!(size < u64::max_value());
    assert!(size.is_power_of_two());
}

#[test]
fn sensible_io_vectors_count() {
    let count = *SYSTEM_IOV_MAX;
    assert!(count > 0);
}

#[test]
fn new_page_aware_address_range_1page() {
    assert_eq!(
        PageAwareAddressRange::new(0, 0),
        PageAwareAddressRange::default()
    );
    assert_eq!(
        PageAwareAddressRange::new(0x1000_0000, 0),
        PageAwareAddressRange {
            start_address: 0x1000_0000,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(u64::max_value(), 0),
        PageAwareAddressRange {
            start_address: u64::max_value(),
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(0, 16),
        PageAwareAddressRange {
            size_in_first_page: 16,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(0x1000_0000, 16),
        PageAwareAddressRange {
            start_address: 0x1000_0000,
            size_in_first_page: 16,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(0x1000_0000 - 16, 16),
        PageAwareAddressRange {
            start_address: 0x1000_0000 - 16,
            size_in_first_page: 16,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(u64::max_value() - 16, 13),
        PageAwareAddressRange {
            start_address: u64::max_value() - 16,
            size_in_first_page: 13,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(u64::max_value() - 16, 16),
        PageAwareAddressRange {
            start_address: u64::max_value() - 16,
            size_in_first_page: 16,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(u64::max_value() - 16, 17),
        PageAwareAddressRange {
            start_address: u64::max_value() - 16,
            size_in_first_page: 17,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(0x1000_0000 + 16, 32),
        PageAwareAddressRange {
            start_address: 0x1000_0000 + 16,
            size_in_first_page: 32,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(0, *MIN_SYSTEM_PAGE_SIZE),
        PageAwareAddressRange {
            size_of_inner_pages: *MIN_SYSTEM_PAGE_SIZE,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(0x1000_0000, *MIN_SYSTEM_PAGE_SIZE),
        PageAwareAddressRange {
            start_address: 0x1000_0000,
            size_of_inner_pages: *MIN_SYSTEM_PAGE_SIZE,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(
            u64::max_value() - *MIN_SYSTEM_PAGE_SIZE + 1,
            *MIN_SYSTEM_PAGE_SIZE
        ),
        PageAwareAddressRange {
            start_address: u64::max_value() - *MIN_SYSTEM_PAGE_SIZE + 1,
            size_of_inner_pages: *MIN_SYSTEM_PAGE_SIZE,
            ..Default::default()
        }
    );
}

#[test]
fn new_page_aware_address_range_2pages() {
    for addr in &[
        u64::max_value() - 7,
        *MIN_SYSTEM_PAGE_SIZE - 8,
        0x1000_0000 - 8,
    ] {
        assert_eq!(
            PageAwareAddressRange::new(*addr, 32),
            PageAwareAddressRange {
                start_address: *addr,
                size_in_first_page: 8,
                size_of_inner_pages: 0,
                size_in_last_page: 24,
            }
        );
    }
}

#[test]
fn new_page_aware_address_range_manypages() {
    assert_eq!(
        PageAwareAddressRange::new(u64::max_value() - 7, 32 + *MIN_SYSTEM_PAGE_SIZE * 5),
        PageAwareAddressRange {
            start_address: u64::max_value() - 7,
            size_in_first_page: 8,
            size_of_inner_pages: *MIN_SYSTEM_PAGE_SIZE * 5,
            size_in_last_page: 24,
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(u64::max_value() - 7, 32 + *MIN_SYSTEM_PAGE_SIZE),
        PageAwareAddressRange {
            start_address: u64::max_value() - 7,
            size_in_first_page: 8,
            size_of_inner_pages: *MIN_SYSTEM_PAGE_SIZE,
            size_in_last_page: 24,
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(u64::max_value() - 7, 32 + *MIN_SYSTEM_PAGE_SIZE * 2),
        PageAwareAddressRange {
            start_address: u64::max_value() - 7,
            size_in_first_page: 8,
            size_of_inner_pages: *MIN_SYSTEM_PAGE_SIZE * 2,
            size_in_last_page: 24,
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(
            u64::max_value() - *MIN_SYSTEM_PAGE_SIZE - 7,
            32 + *MIN_SYSTEM_PAGE_SIZE
        ),
        PageAwareAddressRange {
            start_address: u64::max_value() - *MIN_SYSTEM_PAGE_SIZE - 7,
            size_in_first_page: 8,
            size_of_inner_pages: *MIN_SYSTEM_PAGE_SIZE,
            size_in_last_page: 24,
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(
            u64::max_value() - *MIN_SYSTEM_PAGE_SIZE - 7,
            32 + *MIN_SYSTEM_PAGE_SIZE * 2
        ),
        PageAwareAddressRange {
            start_address: u64::max_value() - *MIN_SYSTEM_PAGE_SIZE - 7,
            size_in_first_page: 8,
            size_of_inner_pages: *MIN_SYSTEM_PAGE_SIZE * 2,
            size_in_last_page: 24,
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(*MIN_SYSTEM_PAGE_SIZE - 8, 32 + *MIN_SYSTEM_PAGE_SIZE * 5),
        PageAwareAddressRange {
            start_address: *MIN_SYSTEM_PAGE_SIZE - 8,
            size_in_first_page: 8,
            size_of_inner_pages: *MIN_SYSTEM_PAGE_SIZE * 5,
            size_in_last_page: 24,
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(0x1000_0000 - 8, 32 + *MIN_SYSTEM_PAGE_SIZE * 5),
        PageAwareAddressRange {
            start_address: 0x1000_0000 - 8,
            size_in_first_page: 8,
            size_of_inner_pages: *MIN_SYSTEM_PAGE_SIZE * 5,
            size_in_last_page: 24,
        }
    );
}

#[test]
fn new_invalid_process_id() {
    assert_matches!(
        unsafe { ProcessVirtualMemoryIO::new(1, 0) }.unwrap_err().kind(),
        ErrorKind::Io { error, process_id: Some(1), .. } if error.kind() == std::io::ErrorKind::PermissionDenied
    );

    for pid in -2_i32..=0 {
        assert_matches!(
            unsafe { ProcessVirtualMemoryIO::new(pid as u32, 0) }.unwrap_err().kind(),
            ErrorKind::Io { error, .. } if error.kind() == std::io::ErrorKind::InvalidInput
        );
    }
}

#[test]
fn new_invalid_address() {
    let process_id = std::process::id();
    assert!(unsafe { ProcessVirtualMemoryIO::new(process_id, 0) }.is_ok());
    assert!(unsafe { ProcessVirtualMemoryIO::new(process_id, u64::max_value()) }.is_ok());
}

#[test]
fn access_address_zero() {
    let process_id = std::process::id();
    let mut buf = [0u8; 1];

    assert_matches!(
        unsafe { ProcessVirtualMemoryIO::new(process_id, 0) }.unwrap()
            .read(&mut buf).unwrap_err()
            .into_inner().unwrap().downcast::<Error>().unwrap()
            .kind(),
        ErrorKind::Io { error, .. } if error.raw_os_error() == Some(libc::EFAULT)
    );

    assert_matches!(
        unsafe { ProcessVirtualMemoryIO::new(process_id, 0) }.unwrap()
            .write(&mut buf).unwrap_err()
            .into_inner().unwrap().downcast::<Error>().unwrap()
            .kind(),
        ErrorKind::Io { error, .. } if error.raw_os_error() == Some(libc::EFAULT)
    );
}
