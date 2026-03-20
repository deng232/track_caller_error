use std::fs;

use app::{AppError, ResultExt};

fn with_single_source() -> Result<String, AppError> {
    Ok(fs::read_to_string("./missing-single-source.txt")?)
}

fn with_context() -> Result<i32, AppError> {
    let content = fs::read_to_string("./missing-ctx.txt").context("read context example")?;
    Ok(content.trim().parse()?)
}

fn main() {
    if let Err(err) = with_single_source() {
        eprintln!("single-source error: {err}");
    }

    if let Err(err) = with_context() {
        eprintln!("context error: {err}");
    }
}
