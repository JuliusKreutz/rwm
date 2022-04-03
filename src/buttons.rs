use xcb::x;

#[derive(Eq, PartialEq, Hash)]
pub struct ButtonCombo {
    mask: x::KeyButMask,
    button: u8,
}

impl ButtonCombo {
    pub const fn new(mask: x::KeyButMask, button: u8) -> Self {
        Self { mask, button }
    }

    pub fn button(&self) -> u8 {
        self.button
    }

    pub fn mask(&self) -> x::KeyButMask {
        self.mask
    }
}
