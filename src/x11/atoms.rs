use crate::x11::X11Error;

use x11rb::atom_manager;
use x11rb::rust_connection::RustConnection;
use x11rb::protocol::xproto::Atom;

atom_manager! {
    pub Atoms: AtomsCookie {
        // ICCCM
        WmProtocols: b"WM_PROTOCOLS" as &[u8],
        WmDeleteWindow: b"WM_DELETE_WINDOW",

        NetSupported: b"_NET_SUPPORTED",
        NetWmName: b"_NET_WM_NAME",
        NetSupportingWmCheck: b"_NET_SUPPORTING_WM_CHECK",

        NetWmWindowType: b"_NET_WM_WINDOW_TYPE",
        NetWmWindowTypeDialog: b"_NET_WM_WINDOW_TYPE_DIALOG",
        NetWmWindowTypeSplash: b"_NET_WM_WINDOW_TYPE_SPLASH",
        NetWmWindowTypeUtility: b"_NET_WM_WINDOW_TYPE_UTILITY",
        NetWmWindowTypeToolbar: b"_NET_WM_WINDOW_TYPE_TOOLBAR",

        NetWmWindowTypeDock: b"_NET_WM_WINDOW_TYPE_DOCK",
        NetWmStrutPartial: b"_NET_WM_STRUT_PARTIAL",

        NetWmDesktop: b"_NET_WM_DESKTOP",
        NetNumberOfDesktops: b"_NET_NUMBER_OF_DESKTOPS",
        NetCurrentDesktop: b"_NET_CURRENT_DESKTOP",
        NetDesktopNames: b"_NET_DESKTOP_NAMES",
        
        UTF8String: b"UTF8_STRING",
    }
}

impl Atoms {
    pub fn load(conn: &RustConnection) -> Result<Self, X11Error> {
        Ok(Self::new(conn)?.reply()?)
    }

    pub fn net_supported(&self) -> Vec<Atom> {
        vec![
            self.NetWmName,
            self.NetWmWindowType,
            self.NetWmWindowTypeDialog,
            self.NetWmWindowTypeSplash,
            self.NetWmWindowTypeUtility,
            self.NetWmWindowTypeToolbar,
            self.NetWmWindowTypeDock,
            self.NetWmStrutPartial,
            self.NetNumberOfDesktops,
            self.NetCurrentDesktop,
            self.NetDesktopNames,
            self.NetSupportingWmCheck,
        ]
    }
}
