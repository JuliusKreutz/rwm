use std::{
    collections::HashMap,
    process::{exit, Child, Command},
};

use xcb::{x, xinerama, Xid};

use crate::{
    buttons::ButtonCombo,
    client::Client,
    config,
    cursor::Cursors,
    keys::{KeyCombo, Keymap},
    monitor::Monitor,
};

atoms!(
    UTF8_STRING,
    WM_PROTOCOLS,
    WM_DELETE_WINDOW,
    _NET_WM_WINDOW_TYPE,
    _NET_WM_WINDOW_TYPE_DIALOG,
    _NET_WM_STATE,
    _NET_WM_STATE_FULLSCREEN,
);

#[derive(Debug)]
enum State {
    Dragging(x::Window, i16, i16, u32),
    Resizing(x::Window, i16, i16, u32),
    None,
}

pub struct Rwm {
    root: x::Window,
    connection: xcb::Connection,
    atoms: [x::Atom; ATOMS.len()],
    keymap: Keymap,
    monitors: Vec<Monitor>,
    monitor: usize,
    focused: Option<x::Window>,
    keys: HashMap<KeyCombo, fn(&mut Self)>,
    buttons: HashMap<ButtonCombo, fn(&mut Self)>,
    state: State,
    border_color: u32,
    border_hl_color: u32,
    cursors: Cursors,
    children: HashMap<u32, Child>,
}

impl Rwm {
    pub fn new() -> Self {
        let (connection, _) = xcb::Connection::connect(None).unwrap();

        let setup = connection.get_setup();

        let screen = setup.roots().next().unwrap();
        let root = screen.root();

        let intern_atom_cookies = ATOMS.map(|atom| {
            connection.send_request(&x::InternAtom {
                only_if_exists: false,
                name: atom.as_bytes(),
            })
        });
        let keyboard_mapping_cookie = connection.send_request(&x::GetKeyboardMapping {
            first_keycode: setup.min_keycode(),
            count: setup.max_keycode() - setup.min_keycode() + 1,
        });
        let border_color_cookie = connection.send_request(&x::AllocColor {
            cmap: screen.default_colormap(),
            red: (config::BORDER_COLOR >> 16) as u16 * 257,
            green: (config::BORDER_COLOR >> 8 & 0x0000ff) as u16 * 257,
            blue: (config::BORDER_COLOR & 0x0000ff) as u16 * 257,
        });
        let border_hl_color_cookie = connection.send_request(&x::AllocColor {
            cmap: screen.default_colormap(),
            red: (config::BORDER_HL_COLOR >> 16) as u16 * 257,
            green: (config::BORDER_HL_COLOR >> 8 & 0x0000ff) as u16 * 257,
            blue: (config::BORDER_HL_COLOR & 0x0000ff) as u16 * 257,
        });

        let atoms =
            intern_atom_cookies.map(|cookie| connection.wait_for_reply(cookie).unwrap().atom());

        let keyboard_mapping = connection.wait_for_reply(keyboard_mapping_cookie).unwrap();
        let keymap = Keymap::new(
            keyboard_mapping.keysyms().to_vec(),
            setup.min_keycode(),
            keyboard_mapping.keysyms_per_keycode(),
        );

        let border_color = connection
            .wait_for_reply(border_color_cookie)
            .unwrap()
            .pixel()
            | 0xff << 24;
        let border_hl_color = connection
            .wait_for_reply(border_hl_color_cookie)
            .unwrap()
            .pixel()
            | 0xff << 24;

        let cursors = Cursors::new(&connection, screen);

        Rwm {
            root,
            connection,
            atoms,
            keymap,
            monitors: Vec::new(),
            monitor: 0,
            focused: None,
            keys: HashMap::from_iter(config::KEYS),
            buttons: HashMap::from_iter(config::BUTTONS),
            state: State::None,
            border_color,
            border_hl_color,
            cursors,
            children: HashMap::new(),
        }
    }

    pub fn spawn(&mut self, command: &str, args: &[&str]) {
        let child = Command::new(command)
            .args(args)
            .spawn()
            .unwrap_or_else(|_| panic!("Command {} with args {:?} couldn't be run", command, args));
        self.children.insert(child.id(), child);
    }

    pub fn kill(&mut self) {
        if let Some(window) = self.focused {
            if self
                .get_atom_property(window, self.atoms[WM_PROTOCOLS])
                .contains(&self.atoms[WM_DELETE_WINDOW])
            {
                let delete_event = x::ClientMessageEvent::new(
                    window,
                    self.atoms[WM_PROTOCOLS],
                    x::ClientMessageData::Data32([
                        self.atoms[WM_DELETE_WINDOW].resource_id(),
                        x::CURRENT_TIME,
                        0,
                        0,
                        0,
                    ]),
                );
                self.connection.send_request(&x::SendEvent {
                    propagate: false,
                    destination: x::SendEventDest::Window(window),
                    event_mask: x::EventMask::NO_EVENT,
                    event: &delete_event,
                });
            } else {
                self.connection.send_request(&x::KillClient {
                    resource: window.resource_id(),
                });
            }

            self.children
                .retain(|_, child| child.try_wait().map_or(true, |ret| ret.is_none()));
        }
    }

    pub fn swap(&mut self) {
        if let Some(window) = self.focused {
            self.monitors[self.monitor].swap(&self.connection, window);
        }
    }

    pub fn toggle_fullscreen(&mut self) {
        if let Some(window) = self.focused {
            self.monitors[self.monitor].toggle_fullscreen(&self.connection, window);
        }
    }

    pub fn toggle_floating(&mut self) {
        if let Some(window) = self.focused {
            self.monitors[self.monitor].toggle_floating(&self.connection, window);
        }
    }

    pub fn main_factor(&mut self, factor: f64) {
        self.monitors[self.monitor].main_factor(&self.connection, factor);
    }

    pub fn view(&mut self, tag: usize) {
        if self.monitors[self.monitor].view(&self.connection, tag) {
            self.focus(None);
            self.draw_status();
        }
    }

    pub fn tag(&mut self, tag: usize) {
        if let Some(window) = self.focused {
            self.focus(None);
            self.draw_status();

            self.monitors[self.monitor].tag(&self.connection, tag, window);
        }
    }

    pub fn tagmon(&mut self) {
        if self.monitors.len() == 1 {
            return;
        }

        if let Some(window) = self.focused {
            if let Some(client) = self.monitors[self.monitor].remove(&self.connection, window) {
                self.focus(None);

                let next_mon = (self.monitor + 1) % self.monitors.len();
                let x = self.monitors[self.monitor].x();
                let y = self.monitors[self.monitor].y();
                let width = self.monitors[self.monitor].width();
                let height = self.monitors[self.monitor].height();
                self.monitors[next_mon].add(&self.connection, client, x, y, width, height);
            }
        }
    }

    pub fn quit(&mut self) {
        for child in self.children.values_mut() {
            let _ = child.kill();
        }

        exit(0);
    }

    pub fn drag(&mut self) {
        if let Some(window) = self.focused {
            if let Ok(pointer_reply) = self.connection.wait_for_reply(
                self.connection
                    .send_request(&x::QueryPointer { window: self.root }),
            ) {
                self.connection.send_request(&x::GrabPointer {
                    owner_events: false,
                    grab_window: self.root,
                    event_mask: x::EventMask::BUTTON_PRESS
                        | x::EventMask::BUTTON_RELEASE
                        | x::EventMask::POINTER_MOTION,
                    pointer_mode: x::GrabMode::Async,
                    keyboard_mode: x::GrabMode::Async,
                    confine_to: x::WINDOW_NONE,
                    cursor: self.cursors.fleur(),
                    time: x::CURRENT_TIME,
                });

                self.monitors[self.monitor].set_floating(&self.connection, window);
                self.state =
                    State::Dragging(window, pointer_reply.root_x(), pointer_reply.root_y(), 0);
            }
        }
    }

    pub fn resize(&mut self) {
        if let Some(window) = self.focused {
            if let Ok(pointer_reply) = self.connection.wait_for_reply(
                self.connection
                    .send_request(&x::QueryPointer { window: self.root }),
            ) {
                self.connection.send_request(&x::GrabPointer {
                    owner_events: false,
                    grab_window: self.root,
                    event_mask: x::EventMask::BUTTON_PRESS
                        | x::EventMask::BUTTON_RELEASE
                        | x::EventMask::POINTER_MOTION,
                    pointer_mode: x::GrabMode::Async,
                    keyboard_mode: x::GrabMode::Async,
                    confine_to: x::WINDOW_NONE,
                    cursor: self.cursors.sizing(),
                    time: x::CURRENT_TIME,
                });

                self.monitors[self.monitor].set_floating(&self.connection, window);
                self.state =
                    State::Resizing(window, pointer_reply.root_x(), pointer_reply.root_y(), 0);
            }
        }
    }

    pub fn setup(&mut self) {
        self.update_monitors();

        for key_combo in self.keys.keys() {
            self.connection.send_request(&x::GrabKey {
                owner_events: true,
                grab_window: self.root,
                modifiers: x::ModMask::from_bits_truncate(key_combo.mask().bits()),
                key: self.keymap.get_keycode(key_combo.key()),
                pointer_mode: x::GrabMode::Async,
                keyboard_mode: x::GrabMode::Async,
            });
        }

        for button_combo in self.buttons.keys() {
            self.connection.send_request(&x::GrabButton {
                owner_events: false,
                grab_window: self.root,
                event_mask: x::EventMask::NO_EVENT,
                pointer_mode: x::GrabMode::Async,
                keyboard_mode: x::GrabMode::Async,
                confine_to: x::WINDOW_NONE,
                cursor: x::CURSOR_NONE,
                button: if button_combo.button() == 1 {
                    x::ButtonIndex::N1
                } else {
                    x::ButtonIndex::N3
                },
                modifiers: x::ModMask::from_bits_truncate(button_combo.mask().bits()),
            });
        }

        self.connection.send_request(&x::ChangeWindowAttributes {
            window: self.root,
            value_list: &[
                x::Cw::EventMask(
                    x::EventMask::BUTTON_PRESS
                        | x::EventMask::BUTTON_RELEASE
                        | x::EventMask::KEY_PRESS
                        | x::EventMask::POINTER_MOTION
                        | x::EventMask::PROPERTY_CHANGE
                        | x::EventMask::STRUCTURE_NOTIFY
                        | x::EventMask::SUBSTRUCTURE_REDIRECT
                        | x::EventMask::SUBSTRUCTURE_NOTIFY,
                ),
                x::Cw::Cursor(self.cursors.left_ptr()),
            ],
        });

        self.connection.send_request(&x::SetCloseDownMode {
            mode: x::CloseDown::DestroyAll,
        });

        self.connection.flush().unwrap();
    }

    pub fn run(&mut self) {
        loop {
            match self.connection.wait_for_event() {
                Ok(event) => match event {
                    xcb::Event::X(x::Event::KeyPress(event)) => self.key_press(event),
                    xcb::Event::X(x::Event::ButtonPress(event)) => self.button_press(event),
                    xcb::Event::X(x::Event::ButtonRelease(_)) => self.button_release(),
                    xcb::Event::X(x::Event::MapRequest(event)) => self.map_request(event),
                    xcb::Event::X(x::Event::UnmapNotify(event)) => self.unmap_notify(event),
                    xcb::Event::X(x::Event::DestroyNotify(event)) => self.destroy_notify(event),
                    xcb::Event::X(x::Event::ConfigureRequest(event)) => {
                        self.configure_request(event)
                    }
                    xcb::Event::X(x::Event::ConfigureNotify(event)) => self.configure_notify(event),
                    xcb::Event::X(x::Event::EnterNotify(event)) => self.enter_notify(event),
                    xcb::Event::X(x::Event::MotionNotify(event)) => self.motion_notify(event),
                    xcb::Event::X(x::Event::PropertyNotify(event)) => self.property_notify(event),
                    xcb::Event::X(x::Event::ClientMessage(event)) => self.client_message(event),
                    _ => {}
                },
                Err(err) => println!("{:?}", err),
            }

            let _ = self.connection.flush();
        }
    }

    fn key_press(&mut self, event: x::KeyPressEvent) {
        let key_combo = KeyCombo::new(event.state(), self.keymap.get_keysym(event.detail()));

        if let Some(command) = self.keys.get(&key_combo) {
            self.connection.send_request(&x::GrabKeyboard {
                owner_events: false,
                grab_window: self.root,
                time: x::CURRENT_TIME,
                pointer_mode: x::GrabMode::Async,
                keyboard_mode: x::GrabMode::Async,
            });
            command(self);
            self.connection.send_request(&x::UngrabKeyboard {
                time: x::CURRENT_TIME,
            });
        }
    }

    fn button_press(&mut self, event: x::ButtonPressEvent) {
        let key_combo = ButtonCombo::new(event.state(), event.detail());

        if let Some(command) = self.buttons.get(&key_combo) {
            command(self);
        }
    }

    fn button_release(&mut self) {
        self.connection.send_request(&x::UngrabPointer {
            time: x::CURRENT_TIME,
        });

        self.state = State::None
    }

    fn map_request(&mut self, event: x::MapRequestEvent) {
        self.connection.send_request(&x::ChangeWindowAttributes {
            window: event.window(),
            value_list: &[
                x::Cw::BorderPixel(self.border_color),
                x::Cw::EventMask(x::EventMask::ENTER_WINDOW | x::EventMask::PROPERTY_CHANGE),
            ],
        });

        let property = self
            .get_property(
                event.window(),
                x::ATOM_WM_NORMAL_HINTS,
                x::ATOM_WM_SIZE_HINTS,
            )
            .unwrap();

        let data: &[u32] = property.value();
        let x = data[1] as i16;
        let y = data[2] as i16;
        let min_width = data[5] as u16;
        let min_height = data[6] as u16;
        let max_width = data[7] as u16;
        let max_height = data[8] as u16;

        let fixed =
            min_width > 0 && min_height > 0 && min_width == max_width && min_height == max_height;

        if fixed {
            self.monitors[self.monitor].map(
                &self.connection,
                Client::new(event.window(), x, y, min_width, min_height, false, true),
            );
        } else {
            let geometry_cookie = self.connection.send_request(&x::GetGeometry {
                drawable: x::Drawable::Window(event.window()),
            });

            let geometry = self.connection.wait_for_reply(geometry_cookie).unwrap();

            let fullscreen = self
                .get_atom_property(event.window(), self.atoms[_NET_WM_STATE])
                .contains(&self.atoms[_NET_WM_STATE_FULLSCREEN]);
            let floating = self
                .get_atom_property(event.window(), self.atoms[_NET_WM_WINDOW_TYPE])
                .contains(&self.atoms[_NET_WM_WINDOW_TYPE_DIALOG]);

            self.monitors[self.monitor].map(
                &self.connection,
                Client::new(
                    event.window(),
                    geometry.x(),
                    geometry.y(),
                    geometry.width(),
                    geometry.height(),
                    fullscreen,
                    floating,
                ),
            );
        }

        self.connection.send_request(&x::MapWindow {
            window: event.window(),
        });
    }

    fn unmap_notify(&mut self, event: x::UnmapNotifyEvent) {
        if Some(event.window()) == self.focused {
            self.focused = None;
            self.draw_status();
        }

        for monitor in &mut self.monitors {
            monitor.unmap(&self.connection, event.window());
        }
    }

    fn destroy_notify(&mut self, event: x::DestroyNotifyEvent) {
        if Some(event.window()) == self.focused {
            self.focused = None;
            self.draw_status();
        }

        for monitor in &mut self.monitors {
            monitor.unmap(&self.connection, event.window());
        }
    }

    fn configure_request(&mut self, event: x::ConfigureRequestEvent) {
        for monitor in &mut self.monitors {
            monitor.configure_request(
                &self.connection,
                event.window(),
                event.value_mask(),
                event.width(),
                event.height(),
            );
        }
    }

    fn configure_notify(&mut self, event: x::ConfigureNotifyEvent) {
        if event.window() == self.root {
            self.update_monitors();
        }
    }

    fn enter_notify(&mut self, event: x::EnterNotifyEvent) {
        for (i, monitor) in self.monitors.iter().enumerate() {
            if monitor.contains(event.root_x(), event.root_y()) {
                if self.monitor != i {
                    self.monitor = i;
                    self.focus(None);
                }

                break;
            }
        }

        self.focus(Some(event.event()));
        self.draw_status();
    }

    fn motion_notify(&mut self, event: x::MotionNotifyEvent) {
        match self.state {
            State::Dragging(window, x, y, time) => {
                if time == 0 || event.time() - time > (1000 / 60) {
                    let x_delta = event.root_x() - x;
                    let y_delta = event.root_y() - y;

                    for monitor in &mut self.monitors {
                        monitor.drag(&self.connection, window, x_delta, y_delta);
                    }

                    self.state =
                        State::Dragging(window, event.root_x(), event.root_y(), event.time());
                }
            }
            State::Resizing(window, x, y, time)
                if time == 0 || event.time() - time > (1000 / 60) =>
            {
                let x_delta = event.root_x() - x;
                let y_delta = event.root_y() - y;

                for monitor in &mut self.monitors {
                    monitor.resize(&self.connection, window, x_delta, y_delta);
                }

                self.state = State::Resizing(window, event.root_x(), event.root_y(), event.time());
            }
            _ => {
                for (i, monitor) in self.monitors.iter().enumerate() {
                    if monitor.contains(event.root_x(), event.root_y()) {
                        if self.monitor != i {
                            self.monitor = i;
                            self.focus(None);
                        }

                        break;
                    }
                }
            }
        }
    }

    fn property_notify(&mut self, event: x::PropertyNotifyEvent) {
        if event.atom() == self.atoms[_NET_WM_WINDOW_TYPE]
            && self
                .get_atom_property(event.window(), self.atoms[_NET_WM_WINDOW_TYPE])
                .contains(&self.atoms[_NET_WM_WINDOW_TYPE_DIALOG])
        {
            for monitor in &mut self.monitors {
                monitor.set_floating(&self.connection, event.window());
            }
        } else {
            self.draw_status();
        }
    }

    fn client_message(&mut self, event: x::ClientMessageEvent) {
        if event.r#type() == self.atoms[_NET_WM_STATE] {
            if let x::ClientMessageData::Data32(data) = event.data() {
                if data[1] == self.atoms[_NET_WM_STATE_FULLSCREEN].resource_id() {
                    for monitor in &mut self.monitors {
                        if data[0] == 1 {
                            monitor.set_fullscreen(&self.connection, event.window());
                        } else {
                            monitor.toggle_fullscreen(&self.connection, event.window())
                        }
                    }
                }
            }
        }
    }

    fn focus(&mut self, focused: Option<x::Window>) {
        if focused == self.focused {
            return;
        }

        if let Some(window) = self.focused {
            self.connection.send_request(&x::ChangeWindowAttributes {
                window,
                value_list: &[x::Cw::BorderPixel(self.border_color)],
            });
        }

        if let Some(window) = focused {
            self.connection.send_request(&x::ChangeWindowAttributes {
                window,
                value_list: &[x::Cw::BorderPixel(self.border_hl_color)],
            });
            self.connection.send_request(&x::SetInputFocus {
                revert_to: x::InputFocus::PointerRoot,
                focus: window,
                time: x::CURRENT_TIME,
            });
        }

        self.focused = focused;
    }

    fn update_monitors(&mut self) {
        let mut dirty = false;

        let is_active_cookie = self.connection.send_request(&xinerama::IsActive {});
        let query_screens_cookie = self.connection.send_request(&xinerama::QueryScreens {});

        if self
            .connection
            .wait_for_reply(is_active_cookie)
            .unwrap()
            .state()
            > 0
        {
            let query_screens = self
                .connection
                .wait_for_reply(query_screens_cookie)
                .unwrap();
            let screen_infos = query_screens.screen_info();

            let monitors_len = self.monitors.len();
            let infos_len = screen_infos.len();

            if monitors_len <= infos_len {
                for (i, screen_info) in screen_infos.iter().enumerate() {
                    if i >= self.monitors.len() {
                        dirty = true;

                        self.monitors.push(Monitor::new(
                            &self.connection,
                            screen_info.x_org,
                            screen_info.y_org,
                            screen_info.width,
                            screen_info.height,
                        ));
                    } else if self.monitors[i].x() != screen_info.x_org
                        || self.monitors[i].y() != screen_info.y_org
                        || self.monitors[i].width() != screen_info.width
                        || self.monitors[i].height() != screen_info.height
                    {
                        dirty = true;

                        self.monitors[i].update(
                            &self.connection,
                            screen_info.x_org,
                            screen_info.y_org,
                            screen_info.width,
                            screen_info.height,
                        );
                    }
                }
            } else {
                for _ in infos_len..monitors_len {
                    dirty = true;

                    self.monitors
                        .pop()
                        .unwrap()
                        .transfer(&self.connection, &mut self.monitors[monitors_len - 1]);
                }
            }
        } else {
            self.die("Xinerama not active");
        }

        if dirty {
            self.monitor = 0;
        }
    }

    fn get_property(
        &self,
        window: x::Window,
        property: x::Atom,
        r#type: x::Atom,
    ) -> xcb::Result<x::GetPropertyReply> {
        self.connection
            .wait_for_reply(self.connection.send_request(&x::GetProperty {
                delete: false,
                window,
                property,
                r#type,
                long_offset: 0,
                long_length: 1024,
            }))
    }

    fn get_atom_property(&self, window: x::Window, property: x::Atom) -> Vec<x::Atom> {
        if let Ok(property_reply) = self.get_property(window, property, x::ATOM_ATOM) {
            property_reply.value().to_vec()
        } else {
            vec![]
        }
    }

    fn draw_status(&self) {
        let name = self
            .focused
            .and_then(|window| {
                self.get_property(window, x::ATOM_WM_NAME, self.atoms[UTF8_STRING])
                    .ok()
                    .and_then(|reply| {
                        std::str::from_utf8(reply.value())
                            .ok()
                            .map(|s| s.to_string())
                    })
            })
            .unwrap_or_else(|| "".to_string());

        let status = self
            .get_property(self.root, x::ATOM_WM_NAME, x::ATOM_STRING)
            .ok()
            .and_then(|reply| {
                std::str::from_utf8(reply.value())
                    .ok()
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| "".to_string());

        for (i, monitor) in self.monitors.iter().enumerate() {
            if i == self.monitor {
                monitor.draw_status(&name, &status);
            } else {
                monitor.draw_status("", &status);
            }
        }
    }

    fn die(&mut self, message: &str) {
        for child in self.children.values_mut() {
            let _ = child.kill();
        }

        eprintln!("{}", message);

        exit(1);
    }
}
