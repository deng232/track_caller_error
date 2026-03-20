#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::panic::Location;

/// A simple message-only error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageError(String);

impl MessageError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }

    pub fn message(&self) -> &str {
        &self.0
    }
}

impl Display for MessageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for MessageError {}

/// An error wrapper that keeps a message, concrete source, and call-site location.
#[derive(Debug)]
pub struct UniversalError<E> {
    message: String,
    source: E,
    location: &'static Location<'static>,
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
        Self {
            message: value.to_string(),
            source: value,
            location: Location::caller(),
        }
    }
}

impl From<String> for UniversalError<MessageError> {
    #[track_caller]
    fn from(value: String) -> Self {
        UniversalError::wrap(MessageError::new(value))
    }
}

impl From<&str> for UniversalError<MessageError> {
    #[track_caller]
    fn from(value: &str) -> Self {
        UniversalError::wrap(MessageError::new(value))
    }
}

/// Result alias for this crate.
pub type Result<T, E> = std::result::Result<T, UniversalError<E>>;

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

/// Builds a [`UniversalError`] while preserving call-site location.
#[macro_export]
macro_rules! err {
    ($msg:literal $(, $arg:expr)* $(,)?) => {
        $crate::UniversalError::wrap($crate::MessageError::new(format!($msg $(, $arg)*)))
    };
    ($msg:expr) => {
        $crate::UniversalError::wrap($crate::MessageError::new($msg))
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
