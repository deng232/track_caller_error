use std::fs;
use std::io;
use std::num::ParseIntError;

track_caller_error::error_enum! {
    enum AppError {
        Io(io::Error),
        Parse(ParseIntError),
    }
}

type Result<T> = std::result::Result<T, AppError>;
fn with_single_source() -> Result<String> {
    Ok(fs::read_to_string("./missing-single-source.txt")?)
}

fn with_multiple_sources(path: &std::path::Path) -> Result<i32> {
    let content = fs::read_to_string(path)?;
    let parsed = content.trim().parse::<i32>()?;
    Ok(parsed)
}

fn main() {
    if let Err(err) = with_single_source() {
        eprintln!("single-source error: {err}");
    }

    let missing = std::path::Path::new("./missing-multi-source.txt");
    if let Err(err) = with_multiple_sources(missing) {
        eprintln!("multi-source error: {err}");
    }
}
