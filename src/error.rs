#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::panic::Location;

/// Marker type for errors without an underlying source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoSource;

impl Display for NoSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("no source")
    }
}

/// An error wrapper that keeps a message, concrete source, and call-site location.
#[derive(Debug)]
pub struct UniversalError<E = NoSource> {
    message: String,
    source: E,
    location: &'static Location<'static>,
}

impl UniversalError<NoSource> {
    /// Creates a tracked error with a message only.
    #[track_caller]
    pub fn msg(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            source: NoSource,
            location: Location::caller(),
        }
    }
}

impl<E> UniversalError<E> {
    /// Creates a tracked error with a message and a concrete source type.
    #[track_caller]
    pub fn with_source(message: impl Into<String>, source: E) -> Self {
        Self {
            message: message.into(),
            source,
            location: Location::caller(),
        }
    }

    /// Returns the tracked call-site.
    pub fn location(&self) -> &'static Location<'static> {
        self.location
    }

    /// Returns the human-readable message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns a shared reference to the wrapped source.
    pub fn source_ref(&self) -> &E {
        &self.source
    }

    /// Consumes self and returns the wrapped source.
    pub fn into_source(self) -> E {
        self.source
    }
}

impl<E> UniversalError<E>
where
    E: Error,
{
    /// Wraps a source error as-is while capturing the call-site.
    #[track_caller]
    pub fn wrap(source: E) -> Self {
        let message = source.to_string();
        Self {
            message,
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

trait SourceProvider {
    fn as_source(&self) -> Option<&(dyn Error + 'static)>;
}

impl SourceProvider for NoSource {
    fn as_source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl<E> SourceProvider for E
where
    E: Error + 'static,
{
    fn as_source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self)
    }
}

impl<E> Error for UniversalError<E>
where
    E: SourceProvider + fmt::Debug,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.source.as_source()
    }
}

impl<E> From<E> for UniversalError<E>
where
    E: Error,
{
    #[track_caller]
    fn from(value: E) -> Self {
        Self {
            message: value.to_string(),
            source: value,
            location: Location::caller(),
        }
    }
}

/// Result alias for this crate.
pub type Result<T, E = NoSource> = std::result::Result<T, UniversalError<E>>;

/// Extension trait adding tracked context to any `Result<T, E>`.
pub trait ResultExt<T, E>
where
    E: Error,
{
    /// Converts `Err(e)` into `UniversalError::with_source(context, e)` with tracked location.
    #[track_caller]
    fn context(self, context: impl Into<String>) -> Result<T, E>;

    /// Lazily computes context and converts `Err(e)` into tracked [`UniversalError`].
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

impl From<String> for UniversalError<NoSource> {
    #[track_caller]
    fn from(value: String) -> Self {
        UniversalError::msg(value)
    }
}

impl From<&str> for UniversalError<NoSource> {
    #[track_caller]
    fn from(value: &str) -> Self {
        UniversalError::msg(value)
    }
}

/// Builds a [`UniversalError`] while preserving call-site location.
#[macro_export]
macro_rules! err {
    ($msg:literal $(, $arg:expr)* $(,)?) => {
        $crate::UniversalError::msg(format!($msg $(, $arg)*))
    };
    ($msg:expr) => {
        $crate::UniversalError::msg($msg)
    };
}

/// Defines a typed application error enum with automatic `From<Source>` conversions.
///
/// This macro enables using `?` directly across multiple source error types.
#[macro_export]
macro_rules! error_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident ($source:ty)
            ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug)]
        $vis enum $name {
            $(
                $(#[$variant_meta])*
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
                        Self::$variant(inner) => ::std::option::Option::Some(inner as &(dyn ::std::error::Error + 'static)),
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

#[cfg(test)]
mod tests {
    use super::{NoSource, ResultExt, UniversalError};
    use std::error::Error;
    use std::fs;
    use std::io;
    use std::num::ParseIntError;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn failing_io() -> std::result::Result<(), io::Error> {
        Err(io::Error::new(io::ErrorKind::NotFound, "missing file"))
    }

    #[track_caller]
    fn create_msg_error() -> UniversalError<NoSource> {
        UniversalError::msg("boom")
    }

    #[test]
    fn records_call_site_for_message_constructor() {
        let expected = std::panic::Location::caller();
        let err = create_msg_error();
        assert_eq!(err.location().file(), expected.file());
        assert_eq!(err.location().line(), expected.line() + 1);
    }

    #[test]
    fn result_ext_adds_context_and_source() {
        let err = failing_io().context("opening config").unwrap_err();
        assert_eq!(err.message(), "opening config");
        assert!(err.source().is_some());
        assert_eq!(err.source_ref().kind(), io::ErrorKind::NotFound);
    }

    #[track_caller]
    fn create_context_error() -> UniversalError<io::Error> {
        failing_io().context("opening config").unwrap_err()
    }

    #[test]
    fn result_ext_context_records_external_callsite() {
        let expected = std::panic::Location::caller();
        let err = create_context_error();
        assert_eq!(err.location().file(), expected.file());
        assert_eq!(err.location().line(), expected.line() + 1);
    }

    #[test]
    fn display_contains_message_and_location() {
        let err = UniversalError::msg("bad things happened");
        let text = err.to_string();
        assert!(text.contains("bad things happened"));
        assert!(text.contains("at"));
    }

    #[test]
    fn question_mark_works_for_single_source_type() {
        fn one_error_type() -> std::result::Result<(), UniversalError<io::Error>> {
            fs::read_to_string("./definitely-missing-single-source.txt")?;
            Ok(())
        }

        let err = one_error_type().unwrap_err();
        assert_eq!(err.source_ref().kind(), io::ErrorKind::NotFound);
    }

    crate::error_enum! {
        enum AppError {
            Io(io::Error),
            Parse(ParseIntError),
        }
    }

    fn parse_file(path: &std::path::Path) -> std::result::Result<i32, AppError> {
        let content = fs::read_to_string(path)?;
        let value: i32 = content.trim().parse()?;
        Ok(value)
    }

    #[test]
    fn question_mark_works_for_multiple_error_types_via_macro() {
        let missing = std::path::Path::new("./definitely-missing-multi-source.txt");
        let io_err = parse_file(missing).unwrap_err();
        match io_err {
            AppError::Io(inner) => assert_eq!(inner.source_ref().kind(), io::ErrorKind::NotFound),
            AppError::Parse(_) => panic!("expected io variant"),
        }

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("track-caller-error-{stamp}.txt"));
        fs::write(&path, "not-a-number\n").expect("write fixture");

        let parse_err = parse_file(&path).unwrap_err();
        match parse_err {
            AppError::Parse(inner) => {
                assert!(inner.source_ref().to_string().contains("invalid digit"))
            }
            AppError::Io(_) => panic!("expected parse variant"),
        }

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn msg_has_concrete_nosource_marker() {
        let err = UniversalError::msg("plain message");
        let _marker: &NoSource = err.source_ref();
        assert!(err.source().is_none());
    }
}
