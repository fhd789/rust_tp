use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;

lazy_static! {
    static ref SERIAL1: Mutex<SerialPort> = Mutex::new(unsafe {
        // SAFETY: The kernel expects COM1 at 0x3F8 in the QEMU setup.
        init_serial_port()
    });
}

/// # Safety
/// The caller must ensure the UART at 0x3F8 exists and is safe to access.
unsafe fn init_serial_port() -> SerialPort {
    let mut serial_port = SerialPort::new(0x3F8);
    serial_port.init();
    serial_port
}

pub fn _print(args: core::fmt::Arguments) {
    use core::fmt::Write;
    SERIAL1.lock().write_fmt(args).expect("serial write failed");
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! printk {
    ($($arg:tt)*) => {
        $crate::serial_print!($($arg)*);
    };
}
