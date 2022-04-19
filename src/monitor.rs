use xcb::x;

use crate::{bar::Bar, client::Client, config};

pub struct Monitor {
    x: i16,
    y: i16,
    width: u16,
    height: u16,
    bar: Bar,
    tags: Vec<Vec<Client>>,
    tag: usize,
    main_factor: f64,
}

impl Monitor {
    pub fn new(connection: &xcb::Connection, x: i16, y: i16, width: u16, height: u16) -> Self {
        let mut tags = Vec::with_capacity(config::TAGS.len());

        for _ in 0..config::TAGS.len() {
            tags.push(Vec::new())
        }

        let bar = Bar::new(connection, x, y, width);
        bar.init();

        Self {
            x,
            y,
            width,
            height,
            bar,
            tags,
            tag: 0,
            main_factor: 0.5,
        }
    }

    pub fn x(&self) -> i16 {
        self.x
    }

    pub fn y(&self) -> i16 {
        self.y
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    pub fn clients(&self) -> Vec<x::Window> {
        self.tags
            .iter()
            .flatten()
            .map(|client| client.window)
            .collect()
    }

    pub fn contains(&self, x: i16, y: i16) -> bool {
        x >= self.x
            && x < self.x + self.width as i16
            && y >= self.y
            && y < self.y + self.height as i16
    }

    pub fn update(
        &mut self,
        connection: &xcb::Connection,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
    ) {
        self.bar.update(connection, x, y, width);

        for client in &mut self
            .tags
            .iter_mut()
            .flatten()
            .filter(|client| client.floating)
        {
            client.x = client.x - self.x + x;
            client.y = client.y - self.y + y;
            client.width = (client.width as f64 * (width as f64 / self.width as f64)) as u16;
            client.height = (client.height as f64 * (height as f64 / self.height as f64)) as u16;

            let x = client.x;
            let y = client.y;
            let width = client.width;
            let height = client.height;

            resize(connection, client, x, y, width, height);
        }

        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;

        self.draw_tags();
        self.arrange(connection);
    }

    pub fn swap(&mut self, connection: &xcb::Connection, window: x::Window) {
        if let Some(position) = self.tags[self.tag]
            .iter()
            .position(|client| client.window == window)
        {
            let temp = self.tags[self.tag][0].window;
            self.tags[self.tag][0].window = self.tags[self.tag][position].window;
            self.tags[self.tag][position].window = temp;

            let client_1 = &mut self.tags[self.tag][0];
            resize(
                connection,
                client_1,
                client_1.x,
                client_1.y,
                client_1.width,
                client_1.height,
            );

            let client_2 = &mut self.tags[self.tag][position];
            resize(
                connection,
                client_2,
                client_2.x,
                client_2.y,
                client_2.width,
                client_2.height,
            );
        }
    }

    pub fn toggle_fullscreen(&mut self, connection: &xcb::Connection, window: x::Window) {
        if let Some(client) = self
            .tags
            .iter_mut()
            .flatten()
            .find(|client| client.window == window)
        {
            client.fullscreen = !client.fullscreen;

            if client.fullscreen {
                client.old_floating = client.floating;
                client.floating = true;

                connection.send_request(&x::ConfigureWindow {
                    window,
                    value_list: &[
                        x::ConfigWindow::BorderWidth(0),
                        x::ConfigWindow::StackMode(x::StackMode::Above),
                    ],
                });

                resize(connection, client, self.x, self.y, self.width, self.height);
            } else {
                client.floating = client.old_floating;

                connection.send_request(&x::ConfigureWindow {
                    window,
                    value_list: &[x::ConfigWindow::BorderWidth(config::BORDER_WIDTH as u32)],
                });

                if client.floating {
                    client.x = client.old_x;
                    client.y = client.old_y;
                    client.width = client.old_width;
                    client.height = client.old_height;

                    resize(
                        connection,
                        client,
                        client.x,
                        client.y,
                        client.width,
                        client.height,
                    );
                } else {
                    self.arrange(connection);
                }
            }
        }
    }

    pub fn set_fullscreen(&mut self, connection: &xcb::Connection, window: x::Window) {
        if let Some(client) = self
            .tags
            .iter_mut()
            .flatten()
            .find(|client| client.window == window)
        {
            if !client.fullscreen {
                client.fullscreen = true;
                client.old_floating = client.floating;
                client.floating = true;

                connection.send_request(&x::ConfigureWindow {
                    window,
                    value_list: &[
                        x::ConfigWindow::BorderWidth(0),
                        x::ConfigWindow::StackMode(x::StackMode::Above),
                    ],
                });

                resize(connection, client, self.x, self.y, self.width, self.height);
            }
        }
    }

    pub fn toggle_floating(&mut self, connection: &xcb::Connection, window: x::Window) {
        if let Some(client) = self
            .tags
            .iter_mut()
            .flatten()
            .find(|client| client.window == window)
        {
            if client.fullscreen {
                return;
            }

            client.floating = !client.floating;

            self.arrange(connection);
        }
    }

    pub fn set_floating(&mut self, connection: &xcb::Connection, window: x::Window) {
        if let Some(client) = self
            .tags
            .iter_mut()
            .flatten()
            .find(|client| client.window == window)
        {
            client.floating = true;

            self.arrange(connection);
        }
    }

    pub fn main_factor(&mut self, connection: &xcb::Connection, factor: f64) {
        self.main_factor += factor;

        if self.main_factor < 0. || self.main_factor > 1. {
            self.main_factor -= factor;
        }

        self.arrange(connection);
    }

    pub fn view(&mut self, connection: &xcb::Connection, tag: usize) -> bool {
        if tag == self.tag {
            false
        } else {
            for client in &self.tags[self.tag] {
                hide(connection, client);
            }

            self.tag = tag;

            self.arrange(connection);

            self.draw_tags();

            true
        }
    }

    pub fn tag(&mut self, connection: &xcb::Connection, tag: usize, window: x::Window) {
        if tag != self.tag {
            if let Some(position) = self.tags[self.tag]
                .iter()
                .position(|client| client.window == window)
            {
                let client = self.tags[self.tag].remove(position);

                hide(connection, &client);

                self.tags[tag].push(client);

                self.arrange(connection);

                self.draw_tags();
            }
        }
    }

    pub fn map(&mut self, connection: &xcb::Connection, mut client: Client) {
        if client.fullscreen {
            connection.send_request(&x::ConfigureWindow {
                window: client.window,
                value_list: &[x::ConfigWindow::BorderWidth(0)],
            });

            client.old_floating = client.floating;
            client.floating = true;

            resize(
                connection,
                &mut client,
                self.x,
                self.y,
                self.width,
                self.height,
            );
        } else {
            connection.send_request(&x::ConfigureWindow {
                window: client.window,
                value_list: &[x::ConfigWindow::BorderWidth(config::BORDER_WIDTH as u32)],
            });
        }

        if client.floating {
            let x = client.x;
            let y = client.y;
            let width = client.width;
            let height = client.height;

            resize(connection, &mut client, x, y, width, height);
        }

        self.tags[self.tag].push(client);
        self.arrange(connection);

        self.draw_tags();
    }

    pub fn unmap(&mut self, connection: &xcb::Connection, window: x::Window) {
        for tag in self.tags.iter_mut() {
            if let Some(position) = tag.iter().position(|client| client.window == window) {
                tag.remove(position);

                self.draw_tags();

                break;
            }
        }

        self.arrange(connection);
    }

    pub fn remove(&mut self, connection: &xcb::Connection, window: x::Window) -> Option<Client> {
        let mut client = None;

        for tag in &mut self.tags {
            if let Some(position) = tag.iter().position(|client| client.window == window) {
                client = Some(tag.remove(position));
                break;
            }
        }

        if client.is_some() {
            self.arrange(connection);
            self.draw_tags();
        }

        client
    }

    pub fn add(
        &mut self,
        connection: &xcb::Connection,
        mut client: Client,
        x: i16,
        y: i16,
        width: u16,
        height: u16,
    ) {
        if client.floating {
            client.x = client.x - x + self.x;
            client.y = client.y - y + self.y;
            client.width = (client.width as f64 * (self.width as f64 / width as f64)) as u16;
            client.height = (client.height as f64 * (self.height as f64 / height as f64)) as u16;

            let x = client.x;
            let y = client.y;
            let width = client.width;
            let height = client.height;

            resize(connection, &mut client, x, y, width, height);
        }

        self.tags[self.tag].push(client);

        self.arrange(connection);
        self.draw_tags();
    }

    pub fn drag(
        &mut self,
        connection: &xcb::Connection,
        window: x::Window,
        x_delta: i16,
        y_delta: i16,
    ) {
        if let Some(client) = self.tags[self.tag]
            .iter_mut()
            .find(|client| client.window == window)
        {
            if client.fullscreen {
                return;
            }

            client.x += x_delta;
            client.y += y_delta;

            resize(
                connection,
                client,
                client.x,
                client.y,
                client.width,
                client.height,
            );
        }
    }

    pub fn resize(
        &mut self,
        connection: &xcb::Connection,
        window: x::Window,
        x_delta: i16,
        y_delta: i16,
    ) {
        if let Some(client) = self.tags[self.tag]
            .iter_mut()
            .find(|client| client.window == window)
        {
            if client.fullscreen {
                return;
            }

            client.width += x_delta as u16;
            client.height += y_delta as u16;

            resize(
                connection,
                client,
                client.x,
                client.y,
                client.width,
                client.height,
            );
        }
    }

    pub fn transfer(self, connection: &xcb::Connection, monitor: &mut Self) {
        self.bar.clean(connection);

        for tag in self.tags {
            for mut client in tag {
                if client.floating {
                    client.x = client.x - self.x + monitor.x;
                    client.y = client.y - self.y + monitor.y;
                    client.width =
                        (client.width as f64 * (monitor.width as f64 / self.width as f64)) as u16;
                    client.height = (client.height as f64
                        * (monitor.height as f64 / self.height as f64))
                        as u16;

                    let x = client.x;
                    let y = client.y;
                    let width = client.width;
                    let height = client.height;

                    resize(connection, &mut client, x, y, width, height);
                }

                monitor.tags[monitor.tag].push(client);
            }
        }

        monitor.arrange(connection);
        monitor.draw_tags();
    }

    pub fn draw_status(&self, name: &str, status: &str) {
        self.bar.draw_status(name, status)
    }

    pub fn configure_request(
        &mut self,
        connection: &xcb::Connection,
        window: x::Window,
        value_mask: x::ConfigWindowMask,
        width: u16,
        height: u16,
    ) {
        if let Some(client) = self
            .tags
            .iter_mut()
            .flatten()
            .find(|client| client.window == window)
        {
            if client.floating {
                if value_mask.intersects(x::ConfigWindowMask::WIDTH) {
                    client.width = width;
                }
                if value_mask.intersects(x::ConfigWindowMask::HEIGHT) {
                    client.height = height;
                }

                client.x = self.x
                    + (self.width / 2 - (client.width + 2 * config::BORDER_WIDTH) / 2) as i16;
                client.y = self.y
                    + (self.height / 2 - (client.height + 2 * config::BORDER_WIDTH) / 2) as i16;

                resize(
                    connection,
                    client,
                    client.x,
                    client.y,
                    client.width,
                    client.height,
                );
            }

            configure(connection, client);
        }
    }

    fn draw_tags(&self) {
        self.bar.draw_tags(
            self.tag,
            self.tags
                .iter()
                .enumerate()
                .filter_map(|(i, clients)| if !clients.is_empty() { Some(i) } else { None })
                .collect(),
        );
    }

    fn arrange(&mut self, connection: &xcb::Connection) {
        let len = self.tags[self.tag]
            .iter()
            .filter(|client| !client.floating)
            .count();

        let main_width = (self.main_factor * self.width as f64) as u16;

        let mut i = 0;
        for client in &mut self.tags[self.tag] {
            if client.floating {
                connection.send_request(&x::ConfigureWindow {
                    window: client.window,
                    value_list: &[
                        x::ConfigWindow::X(client.x as i32),
                        x::ConfigWindow::Y(client.y as i32),
                    ],
                });
            } else {
                if len == 1 {
                    resize(
                        connection,
                        client,
                        self.x + config::MARGIN as i16 + config::BORDER_WIDTH as i16,
                        self.y
                            + config::BAR_HEIGHT as i16
                            + config::MARGIN as i16
                            + config::BORDER_WIDTH as i16,
                        self.width - 2 * config::MARGIN - 4 * config::BORDER_WIDTH,
                        self.height
                            - config::BAR_HEIGHT
                            - 2 * config::MARGIN
                            - 3 * config::BORDER_WIDTH,
                    )
                } else if i == 0 {
                    resize(
                        connection,
                        client,
                        self.x + config::MARGIN as i16 + config::BORDER_WIDTH as i16,
                        self.y
                            + config::BAR_HEIGHT as i16
                            + config::MARGIN as i16
                            + config::BORDER_WIDTH as i16,
                        main_width,
                        self.height
                            - config::BAR_HEIGHT
                            - 2 * config::MARGIN
                            - 3 * config::BORDER_WIDTH,
                    )
                } else {
                    let height = (self.height - config::BAR_HEIGHT - crate::config::MARGIN)
                        / (len - 1) as u16;
                    resize(
                        connection,
                        client,
                        self.x
                            + main_width as i16
                            + 2 * config::MARGIN as i16
                            + 3 * config::BORDER_WIDTH as i16,
                        self.y
                            + config::BAR_HEIGHT as i16
                            + (i - 1) * height as i16
                            + config::MARGIN as i16
                            + config::BORDER_WIDTH as i16,
                        self.width - main_width - 3 * config::MARGIN - 6 * config::BORDER_WIDTH,
                        height - config::MARGIN - 3 * config::BORDER_WIDTH,
                    );
                }

                i += 1;
            }
        }
    }
}

fn resize(
    connection: &xcb::Connection,
    client: &mut Client,
    x: i16,
    y: i16,
    width: u16,
    height: u16,
) {
    client.old_x = client.x;
    client.x = x;
    client.old_y = client.y;
    client.y = y;
    client.old_width = client.width;
    client.width = width;
    client.old_height = client.height;
    client.height = height;

    connection.send_request(&x::ConfigureWindow {
        window: client.window,
        value_list: &[
            x::ConfigWindow::X(client.x as i32),
            x::ConfigWindow::Y(client.y as i32),
            x::ConfigWindow::Width(client.width as u32),
            x::ConfigWindow::Height(client.height as u32),
        ],
    });

    configure(connection, client);
}

fn configure(connection: &xcb::Connection, client: &Client) {
    let bw = if client.fullscreen {
        0
    } else {
        config::BORDER_WIDTH
    };

    let configure_event = x::ConfigureNotifyEvent::new(
        client.window,
        client.window,
        x::WINDOW_NONE,
        client.x,
        client.y,
        client.width,
        client.height,
        bw,
        false,
    );

    connection.send_request(&x::SendEvent {
        propagate: false,
        destination: x::SendEventDest::Window(client.window),
        event_mask: x::EventMask::STRUCTURE_NOTIFY,
        event: &configure_event,
    });
}

fn hide(connection: &xcb::Connection, client: &Client) {
    let bw = if client.fullscreen {
        0
    } else {
        config::BORDER_WIDTH
    };

    connection.send_request(&x::ConfigureWindow {
        window: client.window,
        value_list: &[
            x::ConfigWindow::X((client.width as i32 + 2 * bw as i32) * -2),
            x::ConfigWindow::Y(client.y as i32),
        ],
    });
}
