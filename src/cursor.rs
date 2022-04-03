use xcb::{x, XidNew};

use crate::ffi;

pub struct Cursors {
    left_ptr: x::Cursor,
    sizing: x::Cursor,
    fleur: x::Cursor,
}

impl Cursors {
    pub fn new(connection: &xcb::Connection, screen: &x::Screen) -> Self {
        unsafe {
            let screen: *mut _ = &mut ffi::xcb_screen_t::from(screen);

            let mut ctx: *mut ffi::xcb_cursor_context_t = std::ptr::null_mut();
            ffi::xcb_cursor_context_new(connection.get_raw_conn(), screen, &mut ctx as *mut _);

            let c_str = std::ffi::CString::new("left_ptr").unwrap();
            let left_ptr = ffi::xcb_cursor_load_cursor(ctx, c_str.as_ptr());

            let c_str = std::ffi::CString::new("sizing").unwrap();
            let sizing = ffi::xcb_cursor_load_cursor(ctx, c_str.as_ptr());

            let c_str = std::ffi::CString::new("fleur").unwrap();
            let fleur = ffi::xcb_cursor_load_cursor(ctx, c_str.as_ptr());

            ffi::xcb_cursor_context_free(ctx);

            Cursors {
                left_ptr: x::Cursor::new(left_ptr),
                sizing: x::Cursor::new(sizing),
                fleur: x::Cursor::new(fleur),
            }
        }
    }

    pub fn left_ptr(&self) -> x::Cursor {
        self.left_ptr
    }

    pub fn sizing(&self) -> x::Cursor {
        self.sizing
    }

    pub fn fleur(&self) -> x::Cursor {
        self.fleur
    }
}
