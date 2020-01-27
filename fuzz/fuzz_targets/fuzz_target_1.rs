// Copyright (c) 2020 MicroDoc Software GmbH.
// See the "LICENSE.txt" file at the top-level directory of this distribution.
//
// Licensed under the MIT license. This file may not be copied, modified,
// or distributed except according to those terms.

#![no_main]

use std::io::{Read, Seek, SeekFrom, Write};
use std::{cmp, process};

use libfuzzer_sys::arbitrary::Arbitrary;
use libfuzzer_sys::{arbitrary, fuzz_target};
use process_vm_io::ProcessVirtualMemoryIO;

struct SeekFromWrapper(SeekFrom);

#[derive(Debug)]
struct FuzzInput {
    initial_address: u64,
    position1: SeekFrom,
    position2: SeekFrom,
    position3: SeekFrom,
    io_size: usize,
}

impl Arbitrary for SeekFromWrapper {
    fn arbitrary<U: arbitrary::Unstructured + ?Sized>(u: &mut U) -> Result<Self, U::Error> {
        let origin = u8::arbitrary(u)?;
        let offset = i64::arbitrary(u)?;
        match origin % 3 {
            0 => Ok(SeekFromWrapper(SeekFrom::Start(offset as u64))),
            1 => Ok(SeekFromWrapper(SeekFrom::Current(offset))),
            2 => Ok(SeekFromWrapper(SeekFrom::End(offset))),
            _ => unreachable!(),
        }
    }
}

impl Arbitrary for FuzzInput {
    fn arbitrary<U: arbitrary::Unstructured + ?Sized>(u: &mut U) -> Result<Self, U::Error> {
        Ok(Self {
            initial_address: u64::arbitrary(u)?,
            position1: SeekFromWrapper::arbitrary(u)?.0,
            position2: SeekFromWrapper::arbitrary(u)?.0,
            position3: SeekFromWrapper::arbitrary(u)?.0,
            io_size: cmp::min(usize::arbitrary(u)?, 10 * 1024 * 1024),
        })
    }
}

fuzz_target!(|data: FuzzInput| {
    let mut io = unsafe {
        ProcessVirtualMemoryIO::new(process::id() as libc::pid_t, data.initial_address).unwrap()
    };

    let _ = io.seek(data.position1);
    let _ = io.seek(data.position2);
    let _ = io.seek(data.position3);

    let mut read_buf = vec![0_u8; data.io_size];
    let read_count = io.read(&mut read_buf).unwrap_or(0);

    if read_count != 0 {
        let mut write_buf = vec![0_u8; read_count];
        io.seek(SeekFrom::Start(write_buf.as_mut_ptr() as u64))
            .unwrap();
        io.write_all(&read_buf[..read_count]).unwrap();

        assert_eq!(read_buf[..read_count], *write_buf.as_slice());
    }
});
