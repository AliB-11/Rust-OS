# MiniOS: x86 Kernel with Preemptive Task Scheduling
 
A small x86 operating system, built from scratch in Rust, extended with a custom preemptive, priority-weighted task scheduler.
 
## Overview
 
This project is based on the implementation of Philipp Oppermann's *["Writing an OS in Rust"](https://os.phil-opp.com/)* series and adds a **preemptive task scheduler** which runs multiple independent tasks, forcibly switching between them on a hardware timer interrupt, with each task's share of CPU time weighted by an assigned priority.

## Implementation Details

## Foundation
The kernel boots into a freestanding x86_64 environment with VGA text output (memory-mapped IO) and serial output over an emulated UART (port-mapped IO), used for all logging. A full interrupt descriptor table handles CPU exceptions and faults, with spin locks protecting shared state, extended to hardware interrupts for the timer and keyboard. Paging provides a working virtual memory setup, and two custom heap allocators (bump and fixed-size block) enable dynamic memory (`Vec`, `Box`, etc.) at runtime. A custom test framework, built on Rust's `custom_test_frameworks` feature, runs unit and integration tests with automated pass/fail reporting via QEMU's `isa-debug-exit` device and serial output.
<img width="724" height="457" alt="image" src="https://github.com/user-attachments/assets/be6d334e-df60-4462-9af8-0b88f6a80d8a" />
VGA output dynamically allocating heap memory 

## Scheduler
Each task has its own dedicated stack and a saved `TaskContext` of callee-saved registers. A hand-written assembly routine (`task_switch`) saves the current task's register state and restores the next task's, using `ret` to jump into it — the same mechanism bootstraps new tasks, since their stacks are pre-loaded with their entry point address. On every hardware timer tick, the scheduler forcibly switches tasks with no cooperation required from the running task.

<img width="547" height="471" alt="TaskScheduler" src="https://github.com/user-attachments/assets/84032a1f-0c65-4e50-bd94-389c99072902" />
Running Task Scheduler

## File Structure

mini_os/
├── .cargo/
│   └── config.toml        
├── src/
│   ├── allocator/
│   │   ├── bump.rs            
│   │   └── fixed_size_block.rs    
│   ├── allocator.rs            
│   ├── gdt.rs               
│   ├── interrupts.rs            
│   ├── memory.rs             
│   ├── scheduler.rs            
│   ├── serial.rs                
│   ├── vga_buffer.rs            
│   ├── lib.rs                   
│   └── main.rs                  
├── tests/
│   ├── basic_boot.rs           
│   ├── heap_allocation.rs      
│   ├── should_panic.rs        
│   └── stack_overflow.rs                                
├── .gitignore
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
├── x86_64-mini_os.json          
└── README.md
