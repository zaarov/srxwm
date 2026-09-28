use std::error::Error;
use std::panic::Location;
use std::fmt::{
    Display,
    Result,
    Formatter,
};

use crate::x11;

#[derive(Debug)]
pub struct WmError {
    kind: ErrorKind,
    location: &'static Location<'static>,
}

#[derive(Debug)]
pub enum ErrorKind {
    X11(x11::X11Error),
    Io(std::io::Error),
}

impl WmError {
    #[track_caller]
    fn new(kind: ErrorKind) -> Self {
        WmError {
            kind,
            location: Location::caller(),
        }
    }
}

impl Display for WmError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match &self.kind {
            ErrorKind::X11(e) => write!(
                f,
                "X11 error: {} (at {})",
                e,
                self.location,
            ),
            ErrorKind::Io(error) => write!(
                f,
                "I/O error: {} (at {})",
                error,
                self.location,
            ),
        }
    }
}

impl Error for WmError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            ErrorKind::X11(error) => Some(error),
            ErrorKind::Io(error) => Some(error),
        }
    }
}

impl From<x11::X11Error> for WmError {
    #[track_caller]
    fn from(err: x11::X11Error) -> Self {
        Self::new(ErrorKind::X11(err))
    }
}

impl From<std::io::Error> for WmError {
    #[track_caller]
    fn from(error: std::io::Error) -> Self {
        Self::new(ErrorKind::Io(error))
    }
}
