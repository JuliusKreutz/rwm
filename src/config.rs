use xcb::x::KeyButMask;

use crate::{
    buttons::ButtonCombo,
    keys::{self, KeyCombo},
    rwm::Rwm,
};

tags!("", "", "", "", "", "", "", "", "");

pub const FONT: &str = "JetBrains Mono 13";
pub const TAG_FONT: &str =
    "Font Awesome 6 Free Solid, Font Awesome 6 Free, Font Awesome 6 Brands 15";

pub const MARGIN: u16 = 10;
pub const TEXT_MARGIN: u16 = 12;

pub const BORDER_WIDTH: u16 = 2;
pub const BORDER_COLOR: u32 = 0xabb2bf;
pub const BORDER_HL_COLOR: u32 = 0x61afef;

pub const BAR_HEIGHT: u16 = 32;
pub const BAR_COLOR: u32 = 0x282d34;
pub const BAR_HL_COLOR: u32 = 0x61afef;
pub const BAR_TEXT_COLOR: u32 = 0xcccccc;
pub const BAR_TEXT_HL_COLOR: u32 = 0xeeeeee;

const MOD: KeyButMask = KeyButMask::MOD4;
const MODSHIFT: KeyButMask = MOD.union(KeyButMask::SHIFT);

keys!(
    (MODSHIFT, keys::XK_Return, spawn!("st")),
    (MOD, keys::XK_p, spawn!("rmenu_run")),
    (MOD, keys::XK_e, spawn!("microsoft-edge-stable")),
    (MOD, keys::XK_s, spawn!("shot")),
    (MODSHIFT, keys::XK_c, kill!()),
    (MOD, keys::XK_Return, swap!()),
    (MOD, keys::XK_f, toggle_fullscreen!()),
    (MOD, keys::XK_space, toggle_floating!()),
    (MOD, keys::XK_Left, main_factor!(-0.05)),
    (MOD, keys::XK_Right, main_factor!(0.05)),
    (MOD, keys::XK_1, view!(0)),
    (MOD, keys::XK_2, view!(1)),
    (MOD, keys::XK_3, view!(2)),
    (MOD, keys::XK_4, view!(3)),
    (MOD, keys::XK_5, view!(4)),
    (MOD, keys::XK_6, view!(5)),
    (MOD, keys::XK_7, view!(6)),
    (MOD, keys::XK_8, view!(7)),
    (MOD, keys::XK_9, view!(8)),
    (MODSHIFT, keys::XK_1, tag!(0)),
    (MODSHIFT, keys::XK_2, tag!(1)),
    (MODSHIFT, keys::XK_3, tag!(2)),
    (MODSHIFT, keys::XK_4, tag!(3)),
    (MODSHIFT, keys::XK_5, tag!(4)),
    (MODSHIFT, keys::XK_6, tag!(5)),
    (MODSHIFT, keys::XK_7, tag!(6)),
    (MODSHIFT, keys::XK_8, tag!(7)),
    (MODSHIFT, keys::XK_9, tag!(8)),
    (MODSHIFT, keys::XK_period, tagmon!()),
    (MODSHIFT, keys::XK_q, quit!()),
);

buttons!((MOD, 1, drag!()), (MOD, 3, resize!()));
