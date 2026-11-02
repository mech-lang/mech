//! Process containment for package admission. Pipe IO and deadlines belong to
//! the caller; this owner never waits synchronously for a process or pipe.

use std::io;
use std::process::{Child, Command};

/// An owned subprocess tree. Unix children inherit an isolated process group;
/// Windows children join a kill-on-close Job before any user code can execute.
pub struct ContainedChild {
    child: Child,
    #[cfg(unix)]
    group: nix::unistd::Pid,
    #[cfg(windows)]
    job: Option<std::os::windows::io::OwnedHandle>,
    terminated: bool,
}

impl ContainedChild {
    pub fn spawn(command: &mut Command) -> io::Result<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
            let mut child = command.spawn()?;
            let group = match i32::try_from(child.id()) {
                Ok(id) => nix::unistd::Pid::from_raw(id),
                Err(_) => {
                    let _ = child.kill();
                    return Err(io::Error::other("child process group is not representable"));
                }
            };
            Ok(Self {
                child,
                group,
                terminated: false,
            })
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            use windows_sys::Win32::System::Threading::{
                CREATE_NEW_PROCESS_GROUP, CREATE_SUSPENDED,
            };

            // Installing the Job after an ordinary spawn permits a child to
            // create uncontained descendants before assignment. Suspend first.
            command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_SUSPENDED);
            let mut child = command.spawn()?;
            let job = match assign_kill_on_close_job(&child) {
                Ok(job) => job,
                Err(error) => {
                    let _ = child.kill();
                    return Err(error);
                }
            };
            let mut owned = Self {
                child,
                job: Some(job),
                terminated: false,
            };
            if let Err(error) = resume_child(&owned.child) {
                let _ = owned.terminate_tree();
                return Err(error);
            }
            Ok(owned)
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = command;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "bounded package admission requires Unix process groups or Windows Jobs",
            ))
        }
    }

    pub fn child_mut(&mut self) -> &mut Child {
        &mut self.child
    }

    /// Terminate descendants even if the direct child has already exited.
    /// Reaping is deliberately left to the caller's deadline-bounded try_wait.
    pub fn terminate_tree(&mut self) -> io::Result<()> {
        if self.terminated {
            return Ok(());
        }
        #[cfg(unix)]
        {
            match nix::sys::signal::killpg(self.group, nix::sys::signal::Signal::SIGKILL) {
                Ok(()) | Err(nix::errno::Errno::ESRCH) => {}
                Err(error) => return Err(io::Error::from_raw_os_error(error as i32)),
            }
        }
        #[cfg(windows)]
        {
            // Closing our only Job handle terminates every contained process,
            // including descendants which inherited the diagnostics pipe.
            drop(self.job.take());
        }
        self.terminated = true;
        Ok(())
    }
}

impl Drop for ContainedChild {
    fn drop(&mut self) {
        let _ = self.terminate_tree();
        let _ = self.child.try_wait();
    }
}

#[cfg(windows)]
fn assign_kill_on_close_job(child: &Child) -> io::Result<std::os::windows::io::OwnedHandle> {
    use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject,
    };

    // SAFETY: handles are either owned OS results or borrowed from the live
    // suspended Child. The information pointer is initialized for this call.
    unsafe {
        let raw_job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if raw_job.is_null() {
            return Err(io::Error::last_os_error());
        }
        let job = OwnedHandle::from_raw_handle(raw_job);
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if SetInformationJobObject(
            raw_job,
            JobObjectExtendedLimitInformation,
            std::ptr::from_ref(&limits).cast(),
            std::mem::size_of_val(&limits) as u32,
        ) == 0
            || AssignProcessToJobObject(raw_job, child.as_raw_handle()) == 0
        {
            return Err(io::Error::last_os_error());
        }
        Ok(job)
    }
}

#[cfg(windows)]
fn resume_child(child: &Child) -> io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    use std::os::windows::process::ChildExt;
    use windows_sys::Win32::System::Threading::ResumeThread;

    // SAFETY: the thread handle is borrowed from the live suspended Child.
    let previous = unsafe { ResumeThread(child.main_thread_handle().as_raw_handle()) };
    if previous == u32::MAX {
        return Err(io::Error::last_os_error());
    }
    if previous != 1 {
        return Err(io::Error::other(
            "unexpected package-admission suspend count",
        ));
    }
    Ok(())
}
