use crate::x11::X11Error;

use x11rb::rust_connection::RustConnection;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    Cursor,
    ConnectionExt,
};

const XC_FLEUR: u16 = 52;
const XC_LEFT_PTR: u16 = 68;
const XC_SIZING: u16 = 120;

pub struct Cursors {
    normal: Cursor,
    move_: Cursor,
    resize: Cursor,
}

impl Cursors {
    pub fn new(conn: &RustConnection) -> Result<Self, X11Error> {
    
        let cursor_font = conn.generate_id()?;

        conn.open_font(cursor_font, b"cursor")?;

        let normal = conn.generate_id()?;
        let move_ = conn.generate_id()?;
        let resize = conn.generate_id()?;

        conn.create_glyph_cursor(
            normal,
            cursor_font,
            cursor_font,
            XC_LEFT_PTR,
            XC_LEFT_PTR + 1,
            0,
            0,
            0,
            0xffff,
            0xffff,
            0xffff,
        )?;

        conn.create_glyph_cursor(
            move_,
            cursor_font,
            cursor_font,
            XC_FLEUR,
            XC_FLEUR + 1,
            0,
            0,
            0,
            0xffff,
            0xffff,
            0xffff,
        )?;

        conn.create_glyph_cursor(
            resize,
            cursor_font,
            cursor_font,
            XC_SIZING,
            XC_SIZING + 1,
            0,
            0,
            0,
            0xffff,
            0xffff,
            0xffff,
        )?;

        conn.close_font(cursor_font)?;

        Ok(Self {
            normal,
            move_,
            resize,
        })
    }

    pub fn normal(&self) -> Cursor {
        self.normal
    }

    pub fn move_(&self) -> Cursor {
        self.move_
    }

    pub fn resize(&self) -> Cursor {
        self.resize
    }
}

