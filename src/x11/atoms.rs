use x11rb::atom_manager;
use x11rb::rust_connection::RustConnection;
use x11rb::errors::ReplyError;

atom_manager! {
    pub Atoms: AtomsCookie {
        // EWMH
        _NET_SUPPORTING_WM_CHECK,
        _NET_WM_NAME,

        // X11 types
        UTF8_STRING,

        WM_PROTOCOLS,
        WM_DELETE_WINDOW,

        _NET_WM_WINDOW_TYPE,
        _NET_WM_WINDOW_TYPE_DOCK,
        _NET_WM_STRUT_PARTIAL,
    }
}

impl Atoms {
    pub fn load(conn: &RustConnection) -> Result<Self, ReplyError> {
        Ok(Self::new(conn)?.reply()?)
    }
}
