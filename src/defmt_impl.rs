#[cfg(test)]
mod tests {
    //! This module is required in order to satisfy the requirements of defmt, while running tests.
    //! Note that this will cause all log `defmt::` log statements to be thrown away.

    #[defmt::global_logger]
    struct GlobalLogger;

    unsafe impl defmt::Logger for GlobalLogger {
        fn acquire() {}
        unsafe fn flush() {}
        unsafe fn release() {}
        unsafe fn write(_bytes: &[u8]) {}
    }

    defmt::timestamp!("");

    #[defmt::panic_handler]
    fn panic() -> ! {
        panic!()
    }
}
