[![crates.io](https://img.shields.io/crates/v/process_vm_io.svg)](https://crates.io/crates/process_vm_io)
[![docs.rs](https://docs.rs/process_vm_io/badge.svg)](https://docs.rs/process_vm_io)
[![license](https://img.shields.io/github/license/mdcssw/process_vm_io?color=black)](https://raw.githubusercontent.com/mdcssw/process_vm_io/master/LICENSE.txt)
[![dependency status](https://deps.rs/crate/process_vm_io/1.0.10/status.svg)](https://deps.rs/crate/process_vm_io/1.0.12)

# I/O access to virtual memory contents of processes

Read and write data from/to the current process and other processes.
This can be used for process monitoring, debugging, testing, communication, etc.

## Examples

Reading the stack of the currently running process, through this library:

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use process_vm_io::ProcessVirtualMemoryIO;
use std::io::Read;

// Perform I/O on this current process.
let process_id = std::process::id();
let address_of_pid = &process_id as *const _ as u64;
let mut process_io = unsafe { ProcessVirtualMemoryIO::new(process_id, address_of_pid) }?;

// Read the stack of this current thread.
let mut buffer = [0u8; std::mem::size_of::<u32>()];
process_io.read_exact(&mut buffer)?;
let also_pid = u32::from_ne_bytes(buffer);
assert_eq!(process_id, also_pid);
# Ok(())
# }
```

Writing to the heap of the currently running process, through this library:

```rust
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use process_vm_io::ProcessVirtualMemoryIO;
use std::io::{Seek, Write};

// Perform I/O on this current process.
let process_id = std::process::id();
let mut process_io = unsafe { ProcessVirtualMemoryIO::new(process_id, 0) }?;

// Some location on the heap that we will write to.
let mut pid_on_the_heap = Box::new(0_u32);

// Seek to that location and write the PID there.
process_io.seek(std::io::SeekFrom::Start(pid_on_the_heap.as_mut() as *mut _ as u64))?;
process_io.write(&process_id.to_ne_bytes())?;

assert_eq!(process_id, *pid_on_the_heap);
# Ok(())
# }
```

## Safety

### Memory safety

Writing to the virtual memory of a process is a potentially unsafe
operation because it may introduce memory unsafety in that process,
and may lead to unexpected states in that process. This is even more
dangerous when the target process is the currently running process.

### Running processes

Performing I/O on a running process is not recommended, because the
layout of its virtual memory can change at any time, or the process
could simply terminate and vanish.
Consider pausing all threads of the specified process before performing
I/O on it. This can usually be done via the `SIGSTOP` and `SIGCONT`
[POSIX signals](https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/signal.h.html).

## Platform-specific notes

For the moment, only Linux is supported.

## Versioning

This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
The `CHANGELOG.md` file details notable changes over time.

## License

Copyright (c) 2020-2023 MicroDoc Software GmbH.

See the `LICENSE.txt` file at the top-level directory of this distribution.

Licensed under the **MIT license**. This file may not be copied, modified, or distributed except according to those terms.
