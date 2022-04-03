use rwm::Rwm;

#[macro_use]
mod macros;
mod bar;
mod buttons;
mod client;
mod config;
mod cursor;
mod ffi;
mod keys;
mod monitor;
mod rwm;

fn main() {
    let mut rwm = Rwm::new();
    rwm.setup();
    rwm.run();
}
