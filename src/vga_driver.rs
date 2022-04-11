#[allow(dead_code)] // it's a library
#[derive(Debug, Clone, Copy, PartialEq, Eq)] // enable copy semantics, make enum printable & comparable
#[repr(u8)] // store each enum variant as a u8
pub enum Color {
    Black = 0x0,
    Blue = 0x1,
    Green = 0x2,
    Cyan = 0x3,
    Red = 0x4,
    Magenta = 0x5,
    Brown = 0x6,
    LightGray = 0x7,
    DarkGray = 0x8,
    LightBlue = 0x9,
    LightGreen = 0xa,
    LightCyan = 0xb,
    LightRed = 0xc,
    Pink = 0xd,
    Yellow = 0xe,
    White = 0xf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)] // enable copy semantics, make enum printable & comparable
#[repr(transparent)] // explicitly represent struct exactly as its underlying type (otherwise there might be optimizations)
struct ColorCode(u8);

impl ColorCode {
    fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)] // field ordering is undefined in Rust; this tells Rust to use C-standard layout
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

use volatile::Volatile;

#[repr(transparent)] // struct should have exact same layout as its underlying type [[ScreenChar; 25]; 80]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT], // [[ScreenChar; 25]; 80] -> 25*80 of ScreenChars
}

pub struct Writer {
    column_position: usize,      // keeps track of current position (in last row)
    color_code: ColorCode,       // color code of current foreground / background colors
    buffer: &'static mut Buffer, // 'static "lifetime" tells compiler that reference is valid for whole program run time
}

impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(), // if we send '\n', call new_line()
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1; // always writes to bottom line; shifts lines up when full
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            }
        }
    }

    fn new_line(&mut self) {
        // iterate through all characters on screen
        for row in 1..BUFFER_HEIGHT {
            // exclusive of upper bound
            for column in 0..BUFFER_WIDTH {
                // exclusive of upper bound
                // set each character to the character on the line below it
                let character: ScreenChar = self.buffer.chars[row][column].read();
                self.buffer.chars[row - 1][column].write(character);
            }
        }
        // clear the final row and reset cursor to 0
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        /* TODO */
        for column in 0..BUFFER_WIDTH {
            self.buffer.chars[row][column].write(ScreenChar {
                ascii_character: b' ',
                color_code: self.color_code,
            });
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline (space to ~) on en.wikipedia.org/wiki/Code_page_437
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe), // prints ■
            }
        }
    }
}

// implement core::fmt::Write trait for Writer so that we can support Rust's formatting macros
use core::fmt;

impl fmt::Write for Writer {
    // core::fmt::Write 'trait'
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(()) // expr evaluates to fmt::Result::Ok containing the unit () type (empty tuple). This is the return value
    }
}

// want to create a global interface, so need to make it static. This requires initializing a Writer at compile-time. It
// is not currently possible to convert a *mut (raw pointer) into a &mut Buffer (mutable reference) at compile-time due
// to limitations of rustc's "const evaluator". Apparently, const evaluator limitations are a common problem in Rust, so
// there is a "lazy_static" crate that only initializes a reference to the underlying type when it is first accessed
use lazy_static::lazy_static;
// we need to provide thread safety for global interfaces. However, std::Mutex is only available in the std library. We
// don't have OS functions to rely on for concurrency, so we need to use the simplest possible concurrency modality --
// spinlocking. This can be implemented without a std library. For this, we use the "spin" crate
use spin::Mutex;
// lazily initialize global mutable static interface (yuck -- is there a better way to do this with higher-level Rust???)
lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

// implement a print! macro. Don't worry about the syntax; learn that later
// by using $crate::, it ensures that it can be called here as print! and
// expands to std::print in other crates (macro_export elevates to "crate
// root", which means we import it with use std::println, not std::macros::println
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_driver::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

// implement _print() for this crate for the above print!() macro
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}
