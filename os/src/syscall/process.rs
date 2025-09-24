//! Process management syscalls
use crate::{
    task::{exit_current_and_run_next, suspend_current_and_run_next},
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

/// 实现系统调用 sys_trace 的三种功能
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");

    match trace_request {
        // 读取用户程序 id 地址处的一个字节的无符号整数值
        0 => {
            // 使用 unsafe 进行类型转换
            let addr = id as *const u8;
            unsafe {
                // 读取地址处的值
                let value = *addr;
                value as isize
            }
        },
        // 将 data 的最低字节写入用户程序的 id 地址处
        1 => {
            // 使用 unsafe 进行类型转换
            let addr = id as *mut u8;
            unsafe {
                // 只取 data 的最低一个字节
                *addr = data as u8;
            }
            0  // 返回值为 0
        },
        // 查询当前任务调用编号为 id 的系统调用的次数
        2 => {
            // 获取当前任务的系统调用计数
            let syscall_counts = crate::task::get_current_task_syscall_counts();
            // 返回系统调用次数
            syscall_counts[id] as isize
        },
        // 其他情况，返回 -1
        _ => -1
    }
}
