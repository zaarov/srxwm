use std::error::Error as StdError;
use std::fmt::{self, Display, Formatter};
use std::panic::Location;

use crate::wm;

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    location: &'static Location<'static>,
}

#[derive(Debug)]
pub enum ErrorKind {
    Wm(wm::Error),
}

impl Error {
    #[track_caller]
    fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            location: Location::caller(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::Wm(e) => {
                write!(f, "window manager error: {} (at {})", e, self.location)
            }
        }
    }
}

impl StdError for Error {}

impl From<wm::Error> for Error {
    #[track_caller]
    fn from(err: wm::Error) -> Self {
        Self::new(ErrorKind::Wm(err))
    }
}
