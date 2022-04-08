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

// need to implement our own panic handler, since we don't have the one included in the standard library
use core::panic::PanicInfo;

// vga_buffer.rs
mod vga_buffer;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // function "diverges" (call stack is never allowed to return), so the return type is "!", or "never" (no type info)
    // note: "!" can be coerced into any type
    println!("{}", info);
    loop {}
}

// implement our own entry point
// ensure entry point is actually called "_start" and isn't mangled
#[no_mangle]
// use C calling convention, not Rust calling convention. Return "never" type
pub extern "C" fn _start() -> ! {
    println!("Hello world!");

    // this implements conditional compilation; the following line only executes during "cargo test"
    #[cfg(test)]
    test_main(); // we don't have to implement this function. The custom_test_frameworks feature auto-generates it
                 // due to the #![reexport_test_harness_main = "test_main"] attribute above

    loop {}
}

// exit_qemu "hack":
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
// it's a 32-bit exit code
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10, // sends 33 (success)
    Failed = 0x11, // sends failure (anything other than success)
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u16)] // port address has to be a u16 (real-mode!)
pub enum QemuPort {
    ExitPort = 0xf4,
}

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
    exit_qemu(QEMUExitCode::Success);
}

#[test_case]
fn trivial_assertion() {
    print!("trivial assertion...");
    assert_eq!(1, 1);
    println!("[ok]");
}
