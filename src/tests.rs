// Copyright (c) 2020-2025 MicroDoc Software GmbH.
// See the "LICENSE.txt" file at the top-level directory of this distribution.
//
// Licensed under the MIT license. This file may not be copied, modified,
// or distributed except according to those terms.

use assert_matches::assert_matches;

use super::*;

#[test]
fn new_page_aware_address_range_1page() {
    let min_page_size = min_system_page_size().unwrap().get();

    assert_eq!(
        PageAwareAddressRange::new(0, 0).unwrap(),
        PageAwareAddressRange::default()
    );
    assert_eq!(
        PageAwareAddressRange::new(0x1000_0000, 0).unwrap(),
        PageAwareAddressRange {
            start_address: 0x1000_0000,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(u64::MAX, 0).unwrap(),
        PageAwareAddressRange {
            start_address: u64::MAX,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(0, 16).unwrap(),
        PageAwareAddressRange {
            size_in_first_page: 16,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(0x1000_0000, 16).unwrap(),
        PageAwareAddressRange {
            start_address: 0x1000_0000,
            size_in_first_page: 16,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(0x1000_0000 - 16, 16).unwrap(),
        PageAwareAddressRange {
            start_address: 0x1000_0000 - 16,
            size_in_first_page: 16,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(u64::MAX - 16, 13).unwrap(),
        PageAwareAddressRange {
            start_address: u64::MAX - 16,
            size_in_first_page: 13,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(u64::MAX - 16, 16).unwrap(),
        PageAwareAddressRange {
            start_address: u64::MAX - 16,
            size_in_first_page: 16,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(u64::MAX - 16, 17).unwrap(),
        PageAwareAddressRange {
            start_address: u64::MAX - 16,
            size_in_first_page: 17,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(0x1000_0000 + 16, 32).unwrap(),
        PageAwareAddressRange {
            start_address: 0x1000_0000 + 16,
            size_in_first_page: 32,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(0, min_page_size).unwrap(),
        PageAwareAddressRange {
            size_of_inner_pages: min_page_size,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(0x1000_0000, min_page_size).unwrap(),
        PageAwareAddressRange {
            start_address: 0x1000_0000,
            size_of_inner_pages: min_page_size,
            ..Default::default()
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(u64::MAX - min_page_size + 1, min_page_size).unwrap(),
        PageAwareAddressRange {
            start_address: u64::MAX - min_page_size + 1,
            size_of_inner_pages: min_page_size,
            ..Default::default()
        }
    );
}

#[test]
fn new_page_aware_address_range_2pages() {
    let min_page_size = min_system_page_size().unwrap().get();

    for addr in &[u64::MAX - 7, min_page_size - 8, 0x1000_0000 - 8] {
        assert_eq!(
            PageAwareAddressRange::new(*addr, 32).unwrap(),
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
    let min_page_size = min_system_page_size().unwrap().get();

    assert_eq!(
        PageAwareAddressRange::new(u64::MAX - 7, 32 + min_page_size * 5).unwrap(),
        PageAwareAddressRange {
            start_address: u64::MAX - 7,
            size_in_first_page: 8,
            size_of_inner_pages: min_page_size * 5,
            size_in_last_page: 24,
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(u64::MAX - 7, 32 + min_page_size).unwrap(),
        PageAwareAddressRange {
            start_address: u64::MAX - 7,
            size_in_first_page: 8,
            size_of_inner_pages: min_page_size,
            size_in_last_page: 24,
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(u64::MAX - 7, 32 + min_page_size * 2).unwrap(),
        PageAwareAddressRange {
            start_address: u64::MAX - 7,
            size_in_first_page: 8,
            size_of_inner_pages: min_page_size * 2,
            size_in_last_page: 24,
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(u64::MAX - min_page_size - 7, 32 + min_page_size).unwrap(),
        PageAwareAddressRange {
            start_address: u64::MAX - min_page_size - 7,
            size_in_first_page: 8,
            size_of_inner_pages: min_page_size,
            size_in_last_page: 24,
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(u64::MAX - min_page_size - 7, 32 + min_page_size * 2).unwrap(),
        PageAwareAddressRange {
            start_address: u64::MAX - min_page_size - 7,
            size_in_first_page: 8,
            size_of_inner_pages: min_page_size * 2,
            size_in_last_page: 24,
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(min_page_size - 8, 32 + min_page_size * 5).unwrap(),
        PageAwareAddressRange {
            start_address: min_page_size - 8,
            size_in_first_page: 8,
            size_of_inner_pages: min_page_size * 5,
            size_in_last_page: 24,
        }
    );
    assert_eq!(
        PageAwareAddressRange::new(0x1000_0000 - 8, 32 + min_page_size * 5).unwrap(),
        PageAwareAddressRange {
            start_address: 0x1000_0000 - 8,
            size_in_first_page: 8,
            size_of_inner_pages: min_page_size * 5,
            size_in_last_page: 24,
        }
    );
}

#[test]
fn new_invalid_process_id() {
    assert_matches!(
        unsafe { ProcessVirtualMemoryIO::new(1, 0) }.unwrap_err().kind(),
        ErrorKind::Io { error, process_id: Some(1), .. } if error.kind() == io::ErrorKind::PermissionDenied
    );

    for pid in -2_i32..=0_i32 {
        assert_matches!(
            unsafe { ProcessVirtualMemoryIO::new(pid as u32, 0) }.unwrap_err().kind(),
            ErrorKind::Io { error, .. } if error.kind() == io::ErrorKind::InvalidInput
        );
    }
}

#[test]
fn new_invalid_address() {
    let process_id = std::process::id();
    assert!(unsafe { ProcessVirtualMemoryIO::new(process_id, 0) }.is_ok());
    assert!(unsafe { ProcessVirtualMemoryIO::new(process_id, u64::MAX) }.is_ok());
}

#[test]
fn access_address_zero() {
    let process_id = std::process::id();
    let mut buf = [0_u8; 1];

    assert_matches!(
        unsafe { ProcessVirtualMemoryIO::new(process_id, 0) }.unwrap()
            .read(&mut buf).unwrap_err()
            .into_inner().unwrap().downcast::<Error>().unwrap()
            .kind(),
        ErrorKind::Io { error, .. } if error.raw_os_error() == Some(libc::EFAULT)
    );

    assert_matches!(
        unsafe { ProcessVirtualMemoryIO::new(process_id, 0) }.unwrap()
            .write(&buf).unwrap_err()
            .into_inner().unwrap().downcast::<Error>().unwrap()
            .kind(),
        ErrorKind::Io { error, .. } if error.raw_os_error() == Some(libc::EFAULT)
    );
}
