//  Chobi and Chithi: Managment tools for ZFS snapshot, send, and recv
//  Copyright (C) 2025  Ifaz Kabir

//  This program is free software: you can redistribute it and/or modify
//  it under the terms of the GNU General Public License as published by
//  the Free Software Foundation, either version 3 of the License, or
//  (at your option) any later version.

//  This program is distributed in the hope that it will be useful,
//  but WITHOUT ANY WARRANTY; without even the implied warranty of
//  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//  GNU General Public License for more details.

//  You should have received a copy of the GNU General Public License
//  along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::os::fd::{AsRawFd, RawFd};
use std::{ffi, io, process};

// Ah hostnames, what wonderful fun
// We make some simplifying support decisions here about the size of hostnames.
// In linux, sysconf(_SC_HOST_NAME_MAX) cannot return a value less than _POSIX_HOST_NAME_MAX (255).
// Source https://www.man7.org/linux/man-pages/man3/sysconf.3.html.
// In FreeBSD, callers to gethostname "should be aware that {HOST_NAME_MAX} may
// be variable or infinite, but is guaranteed to be no less than
// {_POSIX_HOST_NAME_MAX}."
// Source https://man.freebsd.org/cgi/man.cgi?query=gethostname.
// The above two sizes do not include the null terminating byte.
// Note: The size limit is larger than the limit on DNS names
// In the DNS world, RFC 1035 Section 2.3.4 says names are 255 octets or less,
// with length encoded labels ending with a root label length 0, similar to null
// termination. But the 255 size does include the terminating length 0 in the
// DNS case.
const CHITHI_HOST_NAME_MAX: usize = 255;

/// Returns an error if hostname is too long
pub fn hostname() -> io::Result<String> {
    // Add 1 for null terminator
    const NAMELEN: usize = CHITHI_HOST_NAME_MAX + 1;
    let mut buffer = [0 as libc::c_char; NAMELEN];
    let rc = unsafe { libc::gethostname(buffer.as_mut_ptr(), NAMELEN) };
    // Returns 0 or -1 (https://pubs.opengroup.org/onlinepubs/9799919799/functions/gethostname.html)
    if rc == -1 {
        return Err(io::Error::last_os_error());
    };
    // If NAMELEN is too small, then the hostname is truncated, and
    // null-termination isn't specified. The man pages also do not say whether
    // gethostname will return -1 or not if truncation happens. Just do a
    // null-termination check just to be sure.
    if !buffer.contains(&0) {
        return Err(io::Error::other("hostname longer than 255 bytes"));
    };
    // Safety: already did a null-termination check above, so safe
    let hostname_cstr = unsafe { ffi::CStr::from_ptr(buffer.as_ptr()) };
    let hostname = hostname_cstr
        .to_str()
        .map_err(|e| io::Error::other(format!("failed to obtain hostname from c string: {e}")))?;
    Ok(hostname.to_string())
}

// Utility functions for captures
fn set_flags(fd: libc::c_int, flags: libc::c_int) -> io::Result<()> {
    unsafe {
        let ret = libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
        if ret == -1 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}
fn get_not_blocking_fd<T: AsRawFd>(fd: &T) -> io::Result<RawFd> {
    let fd = fd.as_raw_fd();
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags == -1 {
            return Err(io::Error::last_os_error());
        }
        set_flags(fd, flags | libc::O_NONBLOCK)?;
        Ok(fd)
    }
}

// Helpers for poll
fn poll_readable(revents: libc::c_short) -> bool {
    revents & libc::POLLIN != 0
}
fn poll_ended(revents: libc::c_short) -> bool {
    revents & libc::POLLHUP != 0
}

/// Run command and both prints and captures outputs (stdout and stderr)
pub fn capture(command: &mut process::Command) -> io::Result<process::Output> {
    use io::{Read, Write};
    use process::{ExitStatus, Stdio};

    let command = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Buffers
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();

    // Spawn child and collect outputs
    let status: ExitStatus = {
        let mut child = command.spawn()?;

        // Handles that implement Drop
        let mut child_out = child.stdout.take().expect("child stdout is piped");
        let mut child_err = child.stderr.take().expect("child stderr is piped");

        // Set child fds to non-blocking
        let child_out_fd = get_not_blocking_fd(&child_out)?;
        let child_err_fd = get_not_blocking_fd(&child_err)?;

        // Setup
        let mut pollfds = vec![
            libc::pollfd {
                fd: child_out_fd,
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: child_err_fd,
                events: libc::POLLIN,
                revents: 0,
            },
        ];
        let mut idx_map = (0..pollfds.len()).collect::<Vec<_>>();
        let mut remove_buffer = Vec::new();
        let mut readbuf = [0u8; 1024];
        loop {
            if pollfds.is_empty() {
                break;
            }
            let ret =
                unsafe { libc::poll(pollfds.as_mut_ptr(), pollfds.len() as libc::nfds_t, -1) };
            if ret == -1 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                } else {
                    return Err(err);
                }
            };

            for (pollfd_idx, pollfd) in pollfds.iter().enumerate() {
                let idx = idx_map[pollfd_idx];
                if idx == 0 {
                    // child_out
                    if poll_readable(pollfd.revents) {
                        match child_out.read(&mut readbuf) {
                            Ok(n) => {
                                stdout.extend_from_slice(&readbuf[..n]);
                                io::stdout().write_all(&readbuf[..n])?;
                            }
                            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                            Err(e) => return Err(e),
                        }
                    } else if poll_ended(pollfd.revents) {
                        remove_buffer.push(pollfd_idx);
                    }
                } else {
                    // child_err
                    if poll_readable(pollfd.revents) {
                        match child_err.read(&mut readbuf) {
                            Ok(n) => {
                                stderr.extend_from_slice(&readbuf[..n]);
                                io::stderr().write_all(&readbuf[..n])?;
                            }
                            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                            Err(e) => return Err(e),
                        }
                    } else if poll_ended(pollfd.revents) {
                        remove_buffer.push(pollfd_idx);
                    }
                }
            }

            if !remove_buffer.is_empty() {
                // sort so that removing later elements do not move earlier elements
                remove_buffer.sort();
                while let Some(idx) = remove_buffer.pop() {
                    pollfds.remove(idx);
                    idx_map.remove(idx);
                }
            }
        }

        child.wait()?
    };

    Ok(process::Output {
        status,
        stdout,
        stderr,
    })
}
