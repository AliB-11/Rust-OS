use alloc::{boxed::Box, vec::Vec};
use core::sync::atomic::{AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

const STACK_SIZE: usize = 16 * 1024;

#[repr(C)]
struct TaskContext {
    rsp: u64,
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    rbx: u64,
    rbp: u64,
}

impl TaskContext {
    const fn empty() -> Self {
        Self {
            rsp: 0,
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbx: 0,
            rbp: 0,
        }
    }
}

struct Task {
    _stack: Box<[u8; STACK_SIZE]>,
    context: TaskContext,
    name: &'static str,
    priority: u8,
    remaining_quanta: u8,
}

impl Task {
    fn new(entry: extern "C" fn() -> !, name: &'static str, priority: u8) -> Self {
        let stack = Box::new([0; STACK_SIZE]);
        let stack_end = stack.as_ptr() as usize + STACK_SIZE;
        let stack_top = (stack_end & !0xf).saturating_sub(8);

        unsafe {
            (stack_top as *mut u64).write(entry as usize as u64);
        }

        Self {
            _stack: stack,
            context: TaskContext {
                rsp: stack_top as u64,
                ..TaskContext::empty()
            },
            name,
            priority,
            remaining_quanta: priority,
        }
    }
}

struct Scheduler {
    tasks: Vec<Task>,
    current: Option<usize>,
    kernel_context: TaskContext,
    initialized: bool,
}

impl Scheduler {
    fn new() -> Self {
        Self {
            tasks: Vec::new(),
            current: None,
            kernel_context: TaskContext::empty(),
            initialized: false,
        }
    }

    fn choose_next(&mut self) -> usize {
        if self.tasks.iter().all(|task| task.remaining_quanta == 0) {
            for task in &mut self.tasks {
                task.remaining_quanta = task.priority;
            }
        }

        self.tasks
            .iter()
            .enumerate()
            .filter(|(index, task)| {
                task.remaining_quanta > 0 && Some(*index) != self.current
            })
            .max_by_key(|(_, task)| task.remaining_quanta)
            .map(|(index, _)| index)
            .unwrap_or_else(|| self.current.unwrap_or(0))
    }
}

lazy_static! {
    static ref SCHEDULER: Mutex<Scheduler> = Mutex::new(Scheduler::new());
}

static TICKS: AtomicU64 = AtomicU64::new(0);

core::arch::global_asm!(r#"
    .global task_switch
    task_switch:
        mov [rdi + 0], rsp
        mov [rdi + 8], r15
        mov [rdi + 16], r14
        mov [rdi + 24], r13
        mov [rdi + 32], r12
        mov [rdi + 40], rbx
        mov [rdi + 48], rbp

        mov rsp, [rsi + 0]
        mov r15, [rsi + 8]
        mov r14, [rsi + 16]
        mov r13, [rsi + 24]
        mov r12, [rsi + 32]
        mov rbx, [rsi + 40]
        mov rbp, [rsi + 48]
        ret
"#);

unsafe extern "C" {
    fn task_switch(current: *mut TaskContext, next: *const TaskContext);
}

pub fn init() {
    let mut scheduler = SCHEDULER.lock();
    if scheduler.initialized {
        return;
    }

    scheduler.tasks.push(Task::new(task_alpha, "alpha", 3));
    scheduler.tasks.push(Task::new(task_beta, "beta", 2));
    scheduler.tasks.push(Task::new(task_gamma, "gamma", 1));
    scheduler.initialized = true;
    crate::serial_println!("scheduler: 3 tasks ready (priorities 3, 2, 1)");
}

pub fn timer_tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);

    let (current_context, next_context, current_name, next_name) = {
        let mut scheduler = SCHEDULER.lock();
        if !scheduler.initialized {
            return;
        }

        let next = scheduler.choose_next();
        scheduler.tasks[next].remaining_quanta -= 1;
        let current_context = match scheduler.current {
            Some(index) => &mut scheduler.tasks[index].context as *mut TaskContext,
            None => &mut scheduler.kernel_context as *mut TaskContext,
        };
        let current_name = scheduler
            .current
            .map(|index| scheduler.tasks[index].name)
            .unwrap_or("kernel");
        let next_name = scheduler.tasks[next].name;
        let next_context = &scheduler.tasks[next].context as *const TaskContext;
        scheduler.current = Some(next);
        (current_context, next_context, current_name, next_name)
    };

    crate::serial_println!(
        "[hardware IRQ: timer] tick={} context switch {} -> {}",
        tick_count(),
        current_name,
        next_name
    );

    unsafe {
        task_switch(current_context, next_context);
    }
}

pub fn tick_count() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

fn start_task() {
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack, preserves_flags));
    }
}

extern "C" fn task_alpha() -> ! {
    let mut counter = 0;
    start_task();
    loop {
        crate::serial_println!("[task alpha] priority=3 tick={} counter={}", tick_count(), counter);
        counter += 1;
        for _ in 0..50_000 {
            core::hint::spin_loop();
        }
    }
}

extern "C" fn task_beta() -> ! {
    let mut counter = 0;
    start_task();
    loop {
        crate::serial_println!("[task beta ] priority=2 tick={} counter={}", tick_count(), counter);
        counter += 1;
        for _ in 0..50_000 {
            core::hint::spin_loop();
        }
    }
}

extern "C" fn task_gamma() -> ! {
    let mut counter = 0;
    start_task();
    loop {
        crate::serial_println!("[task gamma] priority=1 tick={} counter={}", tick_count(), counter);
        counter += 1;
        for _ in 0..50_000 {
            core::hint::spin_loop();
        }
    }
}