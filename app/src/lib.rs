#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::panic::Location;

pub use app_error_macro::flat_error_enum;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageError(String);

impl MessageError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl Display for MessageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for MessageError {}

#[derive(Debug)]
pub struct UniversalError<E> {
    message: String,
    source: E,
    location: &'static Location<'static>,
}

impl<E> UniversalError<E> {
    #[track_caller]
    pub fn with_source(message: impl Into<String>, source: E) -> Self {
        Self {
            message: message.into(),
            source,
            location: Location::caller(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn source_ref(&self) -> &E {
        &self.source
    }

    pub fn into_source(self) -> E {
        self.source
    }

    pub fn location(&self) -> &'static Location<'static> {
        self.location
    }
}

impl<E> UniversalError<E>
where
    E: Error,
{
    #[track_caller]
    pub fn wrap(source: E) -> Self {
        Self {
            message: source.to_string(),
            source,
            location: Location::caller(),
        }
    }
}

impl<E> Display for UniversalError<E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (at {}:{}:{})",
            self.message,
            self.location.file(),
            self.location.line(),
            self.location.column()
        )
    }
}

impl<E> Error for UniversalError<E>
where
    E: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

impl<E> From<E> for UniversalError<E>
where
    E: Error,
{
    #[track_caller]
    fn from(value: E) -> Self {
        Self::wrap(value)
    }
}

pub type Result<T, E> = std::result::Result<T, UniversalError<E>>;

pub trait ResultExt<T, E>
where
    E: Error,
{
    #[track_caller]
    fn context(self, context: impl Into<String>) -> Result<T, E>;

    #[track_caller]
    fn with_context<F>(self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> String;
}

impl<T, E> ResultExt<T, E> for std::result::Result<T, E>
where
    E: Error,
{
    #[track_caller]
    fn context(self, context: impl Into<String>) -> Result<T, E> {
        let location = Location::caller();
        match self {
            Ok(value) => Ok(value),
            Err(source) => Err(UniversalError {
                message: context.into(),
                source,
                location,
            }),
        }
    }

    #[track_caller]
    fn with_context<F>(self, f: F) -> Result<T, E>
    where
        F: FnOnce() -> String,
    {
        let location = Location::caller();
        match self {
            Ok(value) => Ok(value),
            Err(source) => Err(UniversalError {
                message: f(),
                source,
                location,
            }),
        }
    }
}

#[macro_export]
macro_rules! err {
    ($msg:literal $(, $arg:expr)* $(,)?) => {
        $crate::UniversalError::wrap($crate::MessageError::new(format!($msg $(, $arg)*)))
    };
    ($msg:expr) => {
        $crate::UniversalError::wrap($crate::MessageError::new($msg))
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __flat_error_enum_impl {
    (
        $vis:vis enum $name:ident {
            $(
                $variant:ident($source:ty)
            ),+ $(,)?
        }
    ) => {
        #[derive(Debug)]
        $vis enum $name {
            $(
                $variant($crate::UniversalError<$source>),
            )+
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    $(
                        Self::$variant(inner) => inner.fmt(f),
                    )+
                }
            }
        }

        impl ::std::error::Error for $name {
            fn source(&self) -> ::std::option::Option<&(dyn ::std::error::Error + 'static)> {
                match self {
                    $(
                        Self::$variant(inner) => ::std::option::Option::Some(inner),
                    )+
                }
            }
        }

        $(
            impl ::std::convert::From<$source> for $name {
                #[track_caller]
                fn from(source: $source) -> Self {
                    Self::$variant($crate::UniversalError::wrap(source))
                }
            }

            impl ::std::convert::From<$crate::UniversalError<$source>> for $name {
                #[track_caller]
                fn from(error: $crate::UniversalError<$source>) -> Self {
                    Self::$variant(error)
                }
            }
        )+
    };
}

pub mod file;
pub mod net;

flat_error_enum! {
    // NOTE: each source type must be unique in the flattened list.
    // Duplicate source types would generate conflicting `From<T> for AppError` impls.
    pub enum AppError {
        use_variants(crate::file_error_variants),
        use_variants(crate::net_error_variants),
        Utf8(std::str::Utf8Error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io;
    use std::num::ParseIntError;

    fn parse_inline() -> std::result::Result<i32, AppError> {
        let _v: i32 = "bad".parse::<i32>()?;
        Ok(1)
    }

    fn read_missing() -> std::result::Result<String, AppError> {
        let text = fs::read_to_string("./definitely-missing-flat-error.txt")?;
        Ok(text)
    }

    #[track_caller]
    fn parse_with_context() -> std::result::Result<i32, AppError> {
        let num = "x".parse::<i32>().context("parsing in context")?;
        Ok(num)
    }

    #[test]
    fn parse_errors_are_flat() {
        let err = parse_inline().unwrap_err();
        match err {
            AppError::Parse(inner) => {
                let src = inner.source_ref();
                let _: &ParseIntError = src;
            }
            _ => panic!("expected AppError::Parse"),
        }
    }

    #[test]
    fn variants_are_contributed_from_multiple_modules() {
        let io_err = read_missing().unwrap_err();
        assert!(matches!(io_err, AppError::Io(_)));

        let net_err = net::always_fails().unwrap_err();
        assert!(matches!(net_err, AppError::Net(_)));
    }

    #[track_caller]
    fn location_for_question_mark() -> AppError {
        read_missing().unwrap_err()
    }

    #[test]
    fn question_mark_records_callsite_line() {
        let expected = std::panic::Location::caller();
        let err = location_for_question_mark();
        match err {
            AppError::Io(inner) => {
                assert_eq!(inner.location().file(), expected.file());
                assert_eq!(inner.location().line(), expected.line() + 1);
            }
            _ => panic!("expected io error"),
        }
    }

    fn pass_through(err: AppError) -> std::result::Result<(), AppError> {
        Err(err)
    }

    fn propagate_existing_app_error() -> std::result::Result<(), AppError> {
        let e = read_missing().unwrap_err();
        pass_through(e)?;
        Ok(())
    }

    #[test]
    fn propagating_existing_app_error_does_not_rewrap() {
        let err = propagate_existing_app_error().unwrap_err();
        assert!(matches!(err, AppError::Io(_)));
    }

    #[test]
    fn context_reports_exact_callsite() {
        let expected = std::panic::Location::caller();
        let err = parse_with_context().unwrap_err();
        match err {
            AppError::Parse(inner) => {
                assert_eq!(inner.location().file(), expected.file());
                assert_eq!(inner.location().line(), expected.line() + 1);
                assert_eq!(inner.message(), "parsing in context");
                assert_eq!(
                    inner.source_ref().kind(),
                    &std::num::IntErrorKind::InvalidDigit
                );
            }
            _ => panic!("expected parse variant"),
        }
    }

    #[test]
    fn err_macro_uses_message_error() {
        let err = err!("hello {}", "world");
        let source: &MessageError = err.source_ref();
        assert_eq!(source.to_string(), "hello world");
    }

    #[test]
    fn with_source_preserves_real_source() {
        let source = io::Error::new(io::ErrorKind::NotFound, "missing");
        let err = UniversalError::with_source("custom", source);
        assert_eq!(err.source_ref().kind(), io::ErrorKind::NotFound);
    }
}
