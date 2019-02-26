//! When serializing or deserializing CBOR goes wrong.
use core::fmt;
use core::result;
use serde::de;
use serde::ser;
#[cfg(feature = "std")]
use std::error;
#[cfg(feature = "std")]
use std::io;

/// This type represents all possible errors that can occur when serializing or deserializing CBOR
/// data.
pub struct Error(ErrorImpl);

/// Alias for a `Result` with the error type `serde_cbor::Error`.
pub type Result<T> = result::Result<T, Error>;

/// Categorizes the cause of a `serde_cbor::Error`.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Category {
    /// The error was caused by a failure to read or write bytes on an IO stream.
    Io,
    /// The error was caused by input that was not syntactically valid CBOR.
    Syntax,
    /// The error was caused by input data that was semantically incorrect.
    Data,
    /// The error was causeed by prematurely reaching the end of the input data.
    Eof,
}

impl Error {
    /// The byte offset at which the error occurred.
    pub fn offset(&self) -> u64 {
        self.0.offset
    }

    pub(crate) fn syntax(code: ErrorCode, offset: u64) -> Error {
        Error(ErrorImpl { code, offset })
    }

    #[cfg(feature = "std")]
    pub(crate) fn io(error: io::Error) -> Error {
        Error(ErrorImpl {
            code: ErrorCode::Io(error),
            offset: 0,
        })
    }

    pub(crate) fn scratch_too_small() -> Error {
        Error(ErrorImpl {
            code: ErrorCode::ScratchTooSmall,
            offset: 0,
        })
    }

    /// Categorizes the cause of this error.
    pub fn classify(&self) -> Category {
        match self.0.code {
            #[cfg(feature = "std")]
            ErrorCode::Message(_) => Category::Data,
            #[cfg(feature = "std")]
            ErrorCode::Io(_) => Category::Io,
            #[cfg(not(feature = "std"))]
            ErrorCode::Custom => Category::Io,
            ErrorCode::ScratchTooSmall => Category::Io,
            ErrorCode::EofWhileParsingValue
            | ErrorCode::EofWhileParsingArray
            | ErrorCode::EofWhileParsingMap => Category::Eof,
            ErrorCode::NumberOutOfRange
            | ErrorCode::LengthOutOfRange
            | ErrorCode::InvalidUtf8
            | ErrorCode::UnassignedCode
            | ErrorCode::UnexpectedCode
            | ErrorCode::TrailingData
            | ErrorCode::ArrayTooShort
            | ErrorCode::ArrayTooLong
            | ErrorCode::RecursionLimitExceeded => Category::Syntax,
        }
    }

    /// Returns true if this error was caused by a failure to read or write bytes on an IO stream.
    pub fn is_io(&self) -> bool {
        match self.classify() {
            Category::Io => true,
            _ => false,
        }
    }

    /// Returns true if this error was caused by input that was not syntactically valid CBOR.
    pub fn is_syntax(&self) -> bool {
        match self.classify() {
            Category::Syntax => true,
            _ => false,
        }
    }

    /// Returns true if this error was caused by data that was semantically incorrect.
    pub fn is_data(&self) -> bool {
        match self.classify() {
            Category::Data => true,
            _ => false,
        }
    }

    /// Returns true if this error was caused by prematurely reaching the end of the input data.
    pub fn is_eof(&self) -> bool {
        match self.classify() {
            Category::Eof => true,
            _ => false,
        }
    }
}

#[cfg(feature = "std")]
impl error::Error for Error {
    fn description(&self) -> &str {
        match self.0.code {
            ErrorCode::Io(ref err) => error::Error::description(err),
            _ => "CBOR error",
        }
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        match self.0.code {
            ErrorCode::Io(ref err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.offset == 0 {
            fmt::Display::fmt(&self.0.code, f)
        } else {
            write!(f, "{} at offset {}", self.0.code, self.0.offset)
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.0, fmt)
    }
}

impl de::Error for Error {
    #[cfg(feature = "std")]
    fn custom<T>(msg: T) -> Error
    where
        T: fmt::Display,
    {
        Error(ErrorImpl {
            code: ErrorCode::Message(msg.to_string()),
            offset: 0,
        })
    }

    #[cfg(not(feature = "std"))]
    fn custom<T>(_msg: T) -> Error
    where
        T: fmt::Display,
    {
        Error(ErrorImpl {
            code: ErrorCode::Custom,
            offset: 0,
        })
    }

    fn invalid_type(unexp: de::Unexpected<'_>, exp: &dyn de::Expected) -> Error {
        if let de::Unexpected::Unit = unexp {
            Error::custom(format_args!("invalid type: null, expected {}", exp))
        } else {
            Error::custom(format_args!("invalid type: {}, expected {}", unexp, exp))
        }
    }
}

#[cfg(feature = "std")]
impl ser::Error for Error {
    fn custom<T>(msg: T) -> Error
    where
        T: fmt::Display,
    {
        Error(ErrorImpl {
            code: ErrorCode::Message(msg.to_string()),
            offset: 0,
        })
    }
}

#[cfg(not(feature = "std"))]
impl ser::Error for Error {
    fn custom<T>(_msg: T) -> Error
    where
        T: fmt::Display,
    {
        // TODO propagate at least something
        Error(ErrorImpl {
            code: ErrorCode::Custom,
            offset: 0,
        })
    }
}

#[cfg(feature = "std")]
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::io(e)
    }
}

#[derive(Debug)]
struct ErrorImpl {
    code: ErrorCode,
    offset: u64,
}

#[derive(Debug)]
pub(crate) enum ErrorCode {
    #[cfg(feature = "std")]
    Message(String),
    #[cfg(feature = "std")]
    Io(io::Error),
    #[cfg(not(feature = "std"))]
    Custom,
    ScratchTooSmall,
    EofWhileParsingValue,
    EofWhileParsingArray,
    EofWhileParsingMap,
    NumberOutOfRange,
    LengthOutOfRange,
    InvalidUtf8,
    UnassignedCode,
    UnexpectedCode,
    TrailingData,
    ArrayTooShort,
    ArrayTooLong,
    RecursionLimitExceeded,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            #[cfg(feature = "std")]
            ErrorCode::Message(ref msg) => f.write_str(msg),
            #[cfg(feature = "std")]
            ErrorCode::Io(ref err) => fmt::Display::fmt(err, f),
            #[cfg(not(feature = "std"))]
            ErrorCode::Custom => f.write_str("Unknown error"),
            ErrorCode::ScratchTooSmall => f.write_str("Scratch buffer too small"),
            ErrorCode::EofWhileParsingValue => f.write_str("EOF while parsing a value"),
            ErrorCode::EofWhileParsingArray => f.write_str("EOF while parsing an array"),
            ErrorCode::EofWhileParsingMap => f.write_str("EOF while parsing a map"),
            ErrorCode::NumberOutOfRange => f.write_str("number out of range"),
            ErrorCode::LengthOutOfRange => f.write_str("length out of range"),
            ErrorCode::InvalidUtf8 => f.write_str("invalid UTF-8"),
            ErrorCode::UnassignedCode => f.write_str("unassigned type"),
            ErrorCode::UnexpectedCode => f.write_str("unexpected code"),
            ErrorCode::TrailingData => f.write_str("trailing data"),
            ErrorCode::ArrayTooShort => f.write_str("array too short"),
            ErrorCode::ArrayTooLong => f.write_str("array too long"),
            ErrorCode::RecursionLimitExceeded => f.write_str("recursion limit exceeded"),
        }
    }
}
