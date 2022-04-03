use xcb::x;

#[repr(C)]
pub struct xcb_visualtype_t {
    visual_id: u32,
    _class: u8,
    bits_per_rgb_value: u8,
    colormap_entries: u16,
    red_mask: u32,
    green_mask: u32,
    blue_mask: u32,
    pad0: u8,
}

impl From<&x::Visualtype> for xcb_visualtype_t {
    fn from(visual_type: &x::Visualtype) -> Self {
        xcb_visualtype_t {
            visual_id: visual_type.visual_id(),
            _class: visual_type.class() as u8,
            bits_per_rgb_value: visual_type.bits_per_rgb_value(),
            colormap_entries: visual_type.colormap_entries(),
            red_mask: visual_type.red_mask(),
            green_mask: visual_type.green_mask(),
            blue_mask: visual_type.blue_mask(),
            pad0: 0,
        }
    }
}

#[repr(C)]
struct xcb_render_directformat_t {
    red_shift: u16,
    red_mask: u16,
    green_shift: u16,
    green_mask: u16,
    blue_shift: u16,
    blue_mask: u16,
    alpha_shift: u16,
    alpha_mask: u16,
}

#[repr(C)]
struct xcb_render_query_pict_formats_reply_t {
    response_type: u8,
    pad0: u8,
    sequence: u16,
    length: u32,
    num_formats: u32,
    num_screens: u32,
    num_depths: u32,
    num_visuals: u32,
    num_subpixel: u32,
    pad1: [u8; 4],
}

#[repr(C)]
struct xcb_render_pictforminfo_t {
    id: u32,
    type_: u8,
    depth: u8,
    pad0: [u8; 2],
    direct: xcb_render_directformat_t,
    colormap: u32,
}

#[repr(C)]
pub struct xcb_cursor_context_t {
    conn: *mut xcb::ffi::xcb_connection_t,
    root: u32,
    cursor_font: u32,
    pf_reply: *mut xcb_render_query_pict_formats_reply_t,
    pict_format: *mut xcb_render_pictforminfo_t,
    rm: [*mut i8; 3],
    size: u32,
    home: *const i8,
    path: *const i8,
    render_version: u32,
}

#[repr(C)]
pub struct xcb_screen_t {
    root: x::Window,
    default_colormap: x::Colormap,
    white_pixel: u32,
    black_pixel: u32,
    current_input_masks: u32,
    width_in_pixels: u16,
    height_in_pixels: u16,
    width_in_millimeters: u16,
    height_in_millimeters: u16,
    min_installed_maps: u16,
    max_installed_maps: u16,
    root_visual: x::Visualid,
    backing_stores: u8,
    save_unders: u8,
    root_depth: u8,
    allowed_depths_len: u8,
}

impl From<&x::Screen> for xcb_screen_t {
    fn from(screen: &x::Screen) -> Self {
        xcb_screen_t {
            root: screen.root(),
            default_colormap: screen.default_colormap(),
            white_pixel: screen.white_pixel(),
            black_pixel: screen.black_pixel(),
            current_input_masks: screen.current_input_masks().bits(),
            width_in_pixels: screen.width_in_pixels(),
            height_in_pixels: screen.height_in_pixels(),
            width_in_millimeters: screen.width_in_millimeters(),
            height_in_millimeters: screen.height_in_millimeters(),
            min_installed_maps: screen.min_installed_maps(),
            max_installed_maps: screen.max_installed_maps(),
            root_visual: screen.root_visual(),
            backing_stores: match screen.backing_stores() {
                x::BackingStore::NotUseful => 0,
                x::BackingStore::WhenMapped => 1,
                x::BackingStore::Always => 2,
            },
            save_unders: if screen.save_unders() { 1 } else { 0 },
            root_depth: screen.root_depth(),
            allowed_depths_len: screen.allowed_depths().count() as u8,
        }
    }
}

#[link(name = "xcb-cursor")]
extern "C" {
    pub fn xcb_cursor_context_new(
        conn: *mut xcb::ffi::xcb_connection_t,
        screen: *mut xcb_screen_t,
        ctx: *mut *mut xcb_cursor_context_t,
    ) -> i32;
}
#[link(name = "xcb-cursor")]
extern "C" {
    pub fn xcb_cursor_load_cursor(ctx: *mut xcb_cursor_context_t, name: *const i8) -> u32;
}
#[link(name = "xcb-cursor")]
extern "C" {
    pub fn xcb_cursor_context_free(ctx: *mut xcb_cursor_context_t);
}
