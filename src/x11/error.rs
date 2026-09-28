use std::error::Error;
use std::panic::Location;
use std::fmt::{
    Display,
    Result,
    Formatter,
};

use x11rb::errors::{
    ConnectError, ConnectionError, ReplyError, ReplyOrIdError,
};

#[derive(Debug)]
pub struct X11Error {
    kind: ErrorKind,
    location: &'static Location<'static>,
}

#[derive(Debug)]
pub enum ErrorKind {
    Connect(ConnectError),
    Connection(ConnectionError),
    Reply(ReplyError),
    ReplyOrId(ReplyOrIdError),
}

impl X11Error {
    #[track_caller]
    fn new(kind: ErrorKind) -> Self {
        X11Error {
            kind,
            location: Location::caller(),
        }
    }
}

impl Display for X11Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match &self.kind {
            ErrorKind::Connect(e) => write!(
                f,
                "failed to connect to X server: {} (at {})",
                e,
                self.location,
            ),
            ErrorKind::Connection(e) => write!(
                f,
                "X11 connection error: {} (at {})",
                e,
                self.location,
            ),
            ErrorKind::Reply(e) => write!(
                f,
                "X11 reply error: {} (at {})",
                e,
                self.location,
            ),
            ErrorKind::ReplyOrId(e) => write!(
                f,
                "reply/id error: {} (at {})",
                e,
                self.location,
            ),
        }
    }
}

impl Error for X11Error {
    fn source(&self) ->  Option <&(dyn Error + 'static)> {
        match &self.kind {
            ErrorKind::Connect(error) => Some(error),
            ErrorKind::Connection(error) => Some(error),
            ErrorKind::Reply(error) => Some(error),
            ErrorKind::ReplyOrId(error) => Some(error),
        }
    }
}

impl From<ConnectError> for X11Error {
    #[track_caller]
    fn from(err: ConnectError) -> Self {
        Self::new(ErrorKind::Connect(err))
    }
}

impl From<ConnectionError> for X11Error {
    #[track_caller]
    fn from(err: ConnectionError) -> Self {
        Self::new(ErrorKind::Connection(err))
    }
}

impl From<ReplyError> for X11Error {
    #[track_caller]
    fn from(err: ReplyError) -> Self {
        Self::new(ErrorKind::Reply(err))
    }
}

impl From<ReplyOrIdError> for X11Error {
    #[track_caller]
    fn from(err: ReplyOrIdError) -> Self {
        Self::new(ErrorKind::ReplyOrId(err))
    }
}
