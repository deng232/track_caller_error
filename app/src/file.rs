#[macro_export]
macro_rules! file_error_variants {
    () => {
        Parse(std::num::ParseIntError),
        Io(std::io::Error),
    };
}
