//! Types related to task management
use super::TaskContext;
use crate::config::{kernel_stack_position, TRAP_CONTEXT, MAX_SYSCALL_NUM};
use crate::mm::{MapPermission, MemorySet, PhysPageNum, VirtAddr, KERNEL_SPACE, VirtPageNum, num_free_frames};
use crate::trap::{trap_handler, TrapContext};
use crate::timer::get_time_us;

use alloc::vec;
use alloc::vec::Vec;

/// task control block structure
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    pub memory_set: MemorySet,
    pub trap_cx_ppn: PhysPageNum,
    pub base_size: usize,
    // task info utilities
    pub syscall_times: Vec<u32>,
    first_scheduled: bool,
    start_time: usize, // in us
}

impl TaskControlBlock {
    pub fn get_trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.get_mut()
    }
    pub fn get_user_token(&self) -> usize {
        self.memory_set.token()
    }
    pub fn new(elf_data: &[u8], app_id: usize) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data);
        let trap_cx_ppn = memory_set
            .translate(VirtAddr::from(TRAP_CONTEXT).into())
            .unwrap()
            .ppn();
        let task_status = TaskStatus::Ready;
        // map a kernel-stack in kernel space
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(app_id);
        KERNEL_SPACE.lock().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
        );
        let task_control_block = Self {
            task_status,
            task_cx: TaskContext::goto_trap_return(kernel_stack_top),
            memory_set,
            trap_cx_ppn,
            base_size: user_sp,
            syscall_times: vec![0; MAX_SYSCALL_NUM],
            first_scheduled: true,
            start_time: usize::MAX,
        };
        // prepare TrapContext in user space
        let trap_cx = task_control_block.get_trap_cx();
        *trap_cx = TrapContext::app_init_context(
            entry_point,
            user_sp,
            KERNEL_SPACE.lock().token(),
            kernel_stack_top,
            trap_handler as usize,
        );
        task_control_block
    }
    pub fn update_when_scheduled(&mut self) {
        if self.first_scheduled {
            self.first_scheduled = false;
            self.start_time = get_time_us();
        }
    }
    /// us
    pub fn running_time(&self) -> usize {
        get_time_us() - self.start_time
    }
    /// actually we can use Result to represent different types of errors
    pub fn mmap(&mut self, start: usize, len: usize, port: usize) -> isize {
        // check validity
        let start_va = VirtAddr::from(start);
        let end_va = VirtAddr::from(start + len);
        if self.memory_set.is_overlapped(start_va, end_va) {
            // overlapped with current memory areas
            return -1;
        }
        // map
        let perm: u8 = ((port & 0x7) << 1) as u8;
        let perm = MapPermission::from_bits(perm).unwrap();
        self.memory_set.insert_framed_area(
            start_va, 
            end_va, 
            perm | MapPermission::U
        );
        0
    }
    pub fn munmap(&mut self, start: usize, len: usize) -> isize {
        let start_va = VirtAddr::from(start);
        let end_va = VirtAddr::from(start + len);
        if self.memory_set.remove_framed_area(start_va, end_va) {
            0
        } else {
            -1
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
/// task status: UnInit, Ready, Running, Exited
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}
