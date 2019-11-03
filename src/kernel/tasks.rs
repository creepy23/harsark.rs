use crate::KernelError;

use crate::utils::arch::svc_call;
use crate::priv_execute;
use cortex_m::interrupt::free as execute_critical;
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m::register::control::Npriv;
use cortex_m_semihosting::hprintln;
use crate::system::task_manager::*;

use cortex_m::Peripherals;

use crate::utils::arch::is_privileged;

static empty_task: TaskControlBlock = TaskControlBlock { sp: 0 };

// GLOBALS:
static mut all_tasks: Scheduler = Scheduler::new();
#[no_mangle]
static mut os_curr_task: &TaskControlBlock = &empty_task;
#[no_mangle]
static mut os_next_task: &TaskControlBlock = &empty_task;
// end GLOBALS

/// Initialize the switcher system
pub fn init(is_preemptive: bool) {
    execute_critical(|_| unsafe { all_tasks.init(is_preemptive) })
}

// The below section just sets up the timer and starts it.
pub fn start_kernel(perif: &mut Peripherals, tick_interval: u32) -> Result<(), KernelError> {
    priv_execute!({
        let syst = &mut perif.SYST;
        syst.set_clock_source(SystClkSource::Core);
        syst.set_reload(tick_interval);
        syst.enable_counter();
        syst.enable_interrupt();

        execute_critical(|_| unsafe { all_tasks.start_kernel() });
        preempt()
    })
}

pub fn create_task<T: Sized>(
    priority: usize,
    stack: &mut [u32],
    handler_fn: fn(&T) -> !,
    param: &T,
) -> Result<(), KernelError>
    where T: Sync {
    priv_execute!({
        execute_critical(|_| unsafe { all_tasks.create_task(priority, stack, handler_fn, param) })
    })
}

pub fn schedule() {
    if is_privileged() == true {
        preempt();
    } else {
        svc_call();
    }
}

pub fn preempt() -> Result<(), KernelError> {
    execute_critical(|_| {
        let handler = unsafe { &mut all_tasks };
        let next_tid = handler.get_next_tid();
        if handler.is_running {
            if handler.curr_tid != next_tid {
                context_switch(handler.curr_tid, next_tid);
            }
        }
        return Ok(());
    })
}

fn context_switch(curr: usize, next: usize) {
    let handler = unsafe { &mut all_tasks };
    let task_curr = &handler.task_control_blocks[curr];
    if handler.started {
        unsafe {
            os_curr_task = task_curr.as_ref().unwrap();
        }
    } else {
        handler.started = true;
    }
    handler.curr_tid = next;
    let task_next = &handler.task_control_blocks[next];
    unsafe {
        os_next_task = task_next.as_ref().unwrap();
        cortex_m::peripheral::SCB::set_pendsv();
    }
}

pub fn is_preemptive() -> bool {
    execute_critical(|_| unsafe { all_tasks.is_preemptive })
}

pub fn get_curr_tid() -> usize {
    execute_critical(|_| {
        let handler = unsafe { &mut all_tasks };
        return handler.curr_tid;
    })
}

pub fn block_tasks(tasks_mask: u32) {
    execute_critical(|_| unsafe {
        all_tasks.block_tasks(tasks_mask);
    })
}

pub fn unblock_tasks(tasks_mask: u32) {
    execute_critical(|_| unsafe {
        all_tasks.unblock_tasks(tasks_mask);
    })
}

pub fn task_exit() {
    let rt = get_curr_tid();
    execute_critical(|_| {
        unsafe { all_tasks.active_tasks &= !(1 << rt as u32) };
    });
    schedule()
}

pub fn release(tasks_mask: u32) {
    execute_critical(|_|
        unsafe { all_tasks.release(tasks_mask) }
    );
}

pub fn enable_preemption() {
    execute_critical(|_|
        unsafe { all_tasks.is_preemptive = true }
    )
}

pub fn disable_preemption() {
    execute_critical(|_| unsafe {
        all_tasks.is_preemptive = false;
    })
}