use crate::task_manager::{preempt, IS_PREEMPTIVE};
use cortex_m::interrupt::free as execute_critical;
use cortex_m_semihosting::hprintln;
use crate::event_manager::{sweep_event_table, EventTimeType};

static mut M_SEC: u32 = 0;
static mut SEC: u32 = 0;
static mut MIN: u32 = 0;

// SysTick Exception handler
#[no_mangle]
pub extern "C" fn SysTick() {
    execute_critical(|_| {
        if unsafe { IS_PREEMPTIVE } {
            preempt();
        }
        let mut m_sec = unsafe { &mut M_SEC };
        let mut sec = unsafe { &mut SEC };
        let mut min = unsafe { &mut MIN };

        *m_sec += 1;

        let mut m_sec_flag = true;
        let mut sec_flag = false;
        let mut min_flag = false;
        let mut hour_flag = false;

        if *m_sec >= 1000 {
            *sec += 1;
            *m_sec = 0;
            sec_flag = true;
        }

        if *sec >= 60 {
            *min += 1;
            *sec = 0;
            min_flag = true;
        }

        if *min >= 60 {
            *min = 0;
            hour_flag = true;
        }

        if m_sec_flag {
            sweep_event_table(EventTimeType::MSec);
        }
        if sec_flag {
            sweep_event_table(EventTimeType::Sec);
        }
        if min_flag {
            sweep_event_table(EventTimeType::Min);
        }
        if hour_flag {
            sweep_event_table(EventTimeType::Hour);
        }
    });
}
