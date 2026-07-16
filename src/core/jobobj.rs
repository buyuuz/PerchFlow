//! Windows Job Object 封装:把子进程及其后续派生的所有子孙进程纳入同一个 job,
//! `TerminateJobObject` 可一次性杀掉整棵树,即使子孙进程已从父进程树 detach
//! (uvicorn --reload、multiprocessing 等场景)。
use std::io;
use std::os::windows::io::RawHandle;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
    SetInformationJobObject, TerminateJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
};

pub struct JobObject {
    handle: HANDLE,
}

// HANDLE 只是一个不透明的内核对象句柄,不绑定线程,可安全跨线程传递/使用
// (tokio 的多线程 runtime 会在 .await 点之间跨线程移动 future,SpawnedChild
// 因而需要 Send)。
unsafe impl Send for JobObject {}

impl JobObject {
    /// 创建一个新 job,并设置 KILL_ON_JOB_CLOSE:job handle 被关闭时
    /// (例如 Perch 进程本身异常退出),OS 会自动杀掉 job 里所有进程,不留孤儿。
    pub fn create() -> io::Result<JobObject> {
        let handle = unsafe { CreateJobObjectW(None, None) }.map_err(to_io_err)?;
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let result = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const std::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        if let Err(e) = result {
            unsafe { let _ = CloseHandle(handle); }
            return Err(to_io_err(e));
        }
        Ok(JobObject { handle })
    }

    /// 把进程纳入该 job。之后该进程派生的所有子孙进程都自动属于同一 job。
    pub fn assign(&self, raw_handle: RawHandle) -> io::Result<()> {
        let process_handle = HANDLE(raw_handle);
        unsafe { AssignProcessToJobObject(self.handle, process_handle) }.map_err(to_io_err)
    }

    /// 终止 job 内所有进程。忽略错误(例如进程已退出)。
    pub fn terminate(&self) {
        unsafe {
            let _ = TerminateJobObject(self.handle, 1);
        }
    }
}

impl Drop for JobObject {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.handle);
        }
    }
}

/// windows crate 的 Error::code() 返回 HRESULT(FACILITY_WIN32 封装,如 Win32 错误 5
/// 会变成 0x80070005)。io::Error::from_raw_os_error 期望的是裸 Win32 码,直接传
/// HRESULT 会导致日志里错误信息乱码。这里解出低 16 位还原为 Win32 码。
fn to_io_err(e: windows::core::Error) -> io::Error {
    io::Error::from_raw_os_error((e.code().0 & 0xFFFF) as i32)
}
