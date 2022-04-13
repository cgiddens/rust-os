// need to disable standard library in order to run bare-metal
#![no_std]
// in order to use the typical "main()" entry point, Rust executes a C runtime library called "crt0"
// ("C Runtime Zero"), which sets up the environment for a C process by creating a stack and placing the
// appropriate arguments into their registers. The C runtime then invokes the Rust runtime entry point, which
// is a language item called "start". "start" sets up things like stack overflow guards and backtrace functions.
// "start" then calls main(). We don't have crt0 (no standard library), so we need to implement our own entry point.
// we can't implement our own "start" language item, because we'd still need "crt0". So instead, we will overwrite
// "crt0" directly by implementing our own start_()
#![no_main]

// we need to use a custom test framework, since #[test] uses the standard library (and stack unwinding).
// to do this, we use the "custom_test_frameworks" feature, which uses a "test_runner" to run a test runner function
// that calls all functions annotated with #[test_case]. In our case, the function is called "run_tests" and lives in
// this crate.
#![feature(custom_test_frameworks)]
#![test_runner(crate::run_tests)]
// normally, the custom_test_frameworks feature generates its own main() function that calls the test_runner function.
// however, since we've specified #![no_main], "cargo test" uses our _start entry point. To fix this, we use a special
// attribute that re-exports the test harness entry point.
#![reexport_test_harness_main = "test_main"]

// BOOTLOADER: we need to have global (inserted assembly) to write out bootloader. 'global_asm' allows that
use core::arch::global_asm;

// BOOTLOADER: "global_asm!" macro is for inline assembly; "start.s" is the ARM bootloader for Cortex-A53
// The A53 is single-core, but we'll get to multi-core later. Right now, I have to learn how to write a
// bootloader from scratch in order to be able to support ARM in QEMU, because the "bootloader" crate through
// which I call "$ bootimage runner" has no ARM support; only x86_64. Plus, it's super opaque and I don't like
// that. This code will run be compiled into the binary, but the linker will place it in the very beginning
// of the program image and use the global "_start" entry point in start.s (which will in turn call our
// _start() rust function
global_asm!(include_str!("start.s"));

// vga_driver.rs, needed for our _start entry point to have access to the VGA interface
mod vga_driver;

use core::ptr;

// implement our own entry point
// ensure entry point is actually called "_start" and isn't mangled
#[no_mangle]
// use C calling convention, not Rust calling convention. Return "never" type
pub extern "C" fn os_entry_point() -> ! {
    println!("Hello world!");

    const UART0: *mut u8 = 0x0900_0000 as *mut u8;
    let out_str = b"AArch64 Bare Metal";
    for byte in out_str {
        unsafe {
            ptr::write_volatile(UART0, *byte);
        }
    }


    // this implements conditional compilation; the following line only executes during "cargo test"
    #[cfg(test)]
    test_main(); // we don't have to implement this function. The custom_test_frameworks feature auto-generates it
                 // due to the #![reexport_test_harness_main = "test_main"] attribute above

    loop {}
}

// need to implement our own panic handler, since we don't have the one included in the standard library
use core::panic::PanicInfo;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // function "diverges" (call stack is never allowed to return), so the return type is "!", or "never" (no type info)
    // note: "!" can be coerced into any type
    println!("{}", info);
    loop {}
}


// exit_qemu "hack":
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// it's a 32-bit exit code
#[repr(u16)]
pub enum QemuExitCode {
    Success = 0x10, // sends 33 (success)
    Failed = 0x11,  // sends failure (anything other than success)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u16)] // port address has to be a u16 (real-mode!)
pub enum QemuPort {
    ExitPort = 0xf4,
}

#[cfg(nope)]
pub fn exit_qemu(exit_code: QemuExitCode) {
    use x86_64::instructions::port::Port;

    // write to QEMU exit port (0xf4) with exit code (success or failure)
    unsafe {
        let mut port = Port::new(QemuPort::ExitPort as u16);
        port.write(exit_code as u32);
    }
}

// cfg(test) = conditional compilation for "cargo test"
// param: slice of references to Fn() trait "trait objects" (objects that implement a trait)
#[cfg(test)]
fn run_tests(tests: &[&dyn Fn()]) {
    println!("Running {} tests...", tests.len());
    for test in tests {
        test();
    }
    exit_qemu(QemuExitCode::Success);
}

#[test_case]
fn trivial_assertion() {
    print!("trivial assertion...");
    assert_eq!(1, 1);
    println!("[ok]");
}
