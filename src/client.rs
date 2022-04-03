use xcb::x;

#[derive(Debug)]
pub struct Client {
    pub window: x::Window,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub fullscreen: bool,
    pub floating: bool,
    pub old_x: i16,
    pub old_y: i16,
    pub old_width: u16,
    pub old_height: u16,
    pub old_floating: bool,
}

impl Client {
    pub fn new(
        window: x::Window,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
        fullscreen: bool,
        floating: bool,
    ) -> Self {
        Client {
            window,
            x,
            y,
            width,
            height,
            fullscreen,
            floating,
            old_x: x,
            old_y: y,
            old_width: width,
            old_height: height,
            old_floating: false,
        }
    }
}
