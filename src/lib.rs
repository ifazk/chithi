//  Chithi: OpenZFS replication tools
//  Copyright (C) 2025-2026  Ifaz Kabir

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

pub mod args;
mod cmd;
pub mod compress;
mod fs;
pub mod send_recv_opts;
pub mod sync_pipelines;
pub mod sys;
pub mod util;
pub mod zfs;

#[cfg(feature = "list")]
pub mod list;
#[cfg(any(feature = "run-bin", feature = "run-bundle"))]
pub mod run;
pub mod spec;
pub mod sync;

pub use cmd::Cmd;
pub use cmd::CmdTarget;
pub use cmd::CmdVec;
pub use cmd::Pipeline;
pub use cmd::Sequence;
pub use fs::{Fs, Role, get_is_roots};

/// Automatically reaps the child's pid when it goes out of scope
pub struct AutoTerminate {
    inner: std::process::Child,
}

impl AutoTerminate {
    pub fn new(child: std::process::Child) -> Self {
        Self { inner: child }
    }
    /// Terminate the program, if it hasn't been done already.
    /// Should not be called if there's reason to believe that the program has
    /// terminated already (e.g. it closed it's output file descriptor), and
    /// wait() should be called directly instead.
    fn terminate(&mut self) {
        let pid = self.pid();
        let _ = unsafe { libc::kill(pid, libc::SIGTERM) };
    }
    fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.inner.wait()
    }
    fn is_reaped(&mut self) -> bool {
        self.inner.try_wait().as_ref().is_ok_and(Option::is_some)
    }
    fn pid(&self) -> libc::pid_t {
        self.inner.id() as libc::pid_t
    }
}

impl Drop for AutoTerminate {
    fn drop(&mut self) {
        if self.is_reaped() {
            return;
        }
        // try terminate
        self.terminate();
        // This won't be an interrupt because wait() loops on interrupts, and we
        // shouldn't really have any other permission issues, etc. So the result
        // should be okay to ignore.
        let _ = self.wait();
    }
}
