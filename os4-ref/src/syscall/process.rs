//! Process management syscalls

use crate::config::MAX_SYSCALL_NUM;
use crate::task::{
    exit_current_and_run_next, suspend_current_and_run_next, TaskStatus,
    TaskInfo, current_task_info, current_user_token, current_mmap
};
use crate::timer::get_time_us;
use crate::mm::{translated_refmut, VirtAddr, num_free_frames};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

// #[derive(Clone, Copy)]
// pub struct TaskInfo {
//     pub status: TaskStatus,
//     pub syscall_times: [u32; MAX_SYSCALL_NUM],
//     pub time: usize,
// }

pub fn sys_exit(exit_code: i32) -> ! {
    info!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

// YOUR JOB: 引入虚地址后重写 sys_get_time
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let us = get_time_us();
    let time_val = translated_refmut(current_user_token(), ts);
    *time_val = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    // unsafe {
    //     *ts = TimeVal {
    //         sec: us / 1_000_000,
    //         usec: us % 1_000_000,
    //     };
    // }
    0
}

// CLUE: 从 ch4 开始不再对调度算法进行测试~
pub fn sys_set_priority(_prio: isize) -> isize {
    -1
}

// YOUR JOB: 扩展内核以实现 sys_mmap 和 sys_munmap
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    // check validity as early as possible
    let start_va = VirtAddr::from(start);
    if !start_va.aligned() {
        // not aligned
        return -1;
    }
    if (port & !0x7) != 0 || (port & 0x7) == 0 {
        // port should only use 3 bits
        // and it shouldn't be 0 (meaningless)
        return -1;
    }
    let end_va = VirtAddr::from(start + len);
    let end_vpn_usize: usize = end_va.ceil().into();
    let start_vpn_usize: usize = start_va.floor().into();
    let num_required_frames = end_vpn_usize - start_vpn_usize;
    if num_required_frames > num_free_frames() {
        // free frames are not enough
        return -1;
    }
    current_mmap(start, len, port)
}

pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    -1
}

// YOUR JOB: 引入虚地址后重写 sys_task_info
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    let task_info = translated_refmut(current_user_token(), ti);
    *task_info = current_task_info();
    // unsafe {
    //     *ti = current_task_info();
    // }
    0
}
