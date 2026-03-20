#![forbid(unsafe_code)]

//! `track_caller_error` provides a `thiserror`-style wrapper that records the real call-site
//! with [`#[track_caller]`](https://doc.rust-lang.org/reference/attributes/codegen.html#the-track_caller-attribute).
//!
//! # Usage
//! ## 1) Single source type + `?` directly
//! ```rust
//! use std::fs;
//! use std::io;
//! use track_caller_error::UniversalError;
//!
//! fn read_cfg() -> Result<String, UniversalError<io::Error>> {
//!     let text = fs::read_to_string("./missing.txt")?;
//!     Ok(text)
//! }
//! ```
//!
//! ## 2) Add context to any `Result<T, E>`
//! ```rust
//! use std::fs;
//! use std::io;
//! use track_caller_error::{ResultExt, UniversalError};
//!
//! fn open_cfg() -> Result<String, UniversalError<io::Error>> {
//!     fs::read_to_string("./missing.txt").context("opening config")
//! }
//! ```
//!
//! ## 3) Multiple source types + `?` directly via `error_enum!`
//! ```rust
//! use std::fs;
//! use std::io;
//! use std::num::ParseIntError;
//!
//! track_caller_error::error_enum! {
//!     enum AppError {
//!         Io(io::Error),
//!         Parse(ParseIntError),
//!     }
//! }
//!
//! fn parse_file(path: &std::path::Path) -> Result<i32, AppError> {
//!     let content = fs::read_to_string(path)?;
//!     let value: i32 = content.trim().parse()?;
//!     Ok(value)
//! }
//! ```

mod error;

pub use error::{NoSource, Result, ResultExt, UniversalError};
