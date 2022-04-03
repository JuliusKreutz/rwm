use std::ptr::NonNull;

use pangocairo::pango;
use xcb::{x, Xid};

use crate::{config, ffi};

struct Tag {
    width: u16,
    text: &'static str,
}

impl Tag {
    fn new(width: u16, text: &'static str) -> Self {
        Self { width, text }
    }
}

struct Tags {
    width: u16,
    tags: Vec<Tag>,
}

impl Tags {
    fn new(tags: Vec<Tag>) -> Self {
        let width = tags.iter().map(|tag| tag.width).sum();

        Self { width, tags }
    }
}

pub struct Bar {
    window: x::Window,
    width: u16,
    context: cairo::Context,
    main_layout: pango::Layout,
    tag_layout: pango::Layout,
    main_text_middle: f64,
    tag_text_middle: f64,
    tags: Tags,
}

impl Bar {
    pub fn new(connection: &xcb::Connection, x: i16, y: i16, width: u16) -> Self {
        let screen = connection.get_setup().roots().next().unwrap();

        let window = connection.generate_id();
        connection.send_request(&x::CreateWindow {
            depth: x::COPY_FROM_PARENT as u8,
            wid: window,
            parent: screen.root(),
            x,
            y,
            width,
            height: config::BAR_HEIGHT,
            border_width: 0,
            class: x::WindowClass::InputOutput,
            visual: screen.root_visual(),
            value_list: &[],
        });
        connection.send_request(&x::MapWindow { window });

        let cairo_conn =
            cairo::XCBConnection(NonNull::new((*connection).get_raw_conn() as *mut _).unwrap());
        let cairo_window = cairo::XCBDrawable(window.resource_id());
        let visual_type = screen
            .allowed_depths()
            .flat_map(|depth| depth.visuals())
            .find(|visual| screen.root_visual() == visual.visual_id())
            .unwrap();
        let visual_ptr = &mut ffi::xcb_visualtype_t::from(visual_type) as *mut _
            as *mut cairo::ffi::xcb_visualtype_t;
        let cairo_visual = cairo::XCBVisualType(std::ptr::NonNull::new(visual_ptr).unwrap());
        let surface = cairo::XCBSurface::create(
            &cairo_conn,
            &cairo_window,
            &cairo_visual,
            screen.width_in_pixels() as i32,
            config::BAR_HEIGHT as i32,
        )
        .unwrap();
        let context = cairo::Context::new(&surface).unwrap();

        let main_layout = pangocairo::create_layout(&context).unwrap();
        main_layout.set_font_description(Some(&pango::FontDescription::from_string(
            crate::config::FONT,
        )));
        let tag_layout = pangocairo::create_layout(&context).unwrap();
        tag_layout.set_font_description(Some(&pango::FontDescription::from_string(
            crate::config::TAG_FONT,
        )));

        let main_text_middle =
            (crate::config::BAR_HEIGHT - (main_layout.size().1 / pango::SCALE) as u16) as f64 / 2.;
        let tag_text_middle =
            (crate::config::BAR_HEIGHT - (tag_layout.size().1 / pango::SCALE) as u16) as f64 / 2.;

        let tags = Tags::new(
            crate::config::TAGS
                .iter()
                .map(|tag| {
                    tag_layout.set_text(tag);
                    Tag::new(
                        (tag_layout.size().0 / pango::SCALE) as u16
                            + crate::config::TEXT_MARGIN * 2,
                        tag,
                    )
                })
                .collect(),
        );

        Bar {
            window,
            width,
            context,
            main_layout,
            tag_layout,
            main_text_middle,
            tag_text_middle,
            tags,
        }
    }

    pub fn init(&self) {
        self.draw_tags(0, Vec::new());
    }

    pub fn clean(&self, connection: &xcb::Connection) {
        connection.send_request(&x::UnmapWindow {
            window: self.window,
        });
    }

    pub fn draw_tags(&self, selected: usize, full_workspaces: Vec<usize>) {
        let mut position = 0;

        for (i, tag) in self.tags.tags.iter().enumerate() {
            let box_color;
            let text_color;

            if i == selected {
                box_color = crate::config::BAR_HL_COLOR;
                text_color = crate::config::BAR_TEXT_HL_COLOR;
            } else {
                box_color = crate::config::BAR_COLOR;
                text_color = crate::config::BAR_TEXT_COLOR;
            }

            self.draw_rectangle(position, tag.width, box_color);
            self.draw_tag_text(position, tag.text, text_color);

            if full_workspaces.contains(&i) {
                self.draw_tag_rectangle(position, text_color);
            }

            position += tag.width;
        }
    }

    pub fn draw_status(&self, name: &str, status: &str) {
        self.main_layout.set_text(name);

        let name_width = self.width
            - self.tags.width
            - ((self.main_layout.size().0 / pango::SCALE) as u16 + 2 * crate::config::TEXT_MARGIN);

        self.draw_rectangle(self.tags.width, name_width, crate::config::BAR_HL_COLOR);
        self.draw_main_text(self.tags.width, name, crate::config::BAR_TEXT_HL_COLOR);

        self.main_layout.set_text(status);
        let status_width =
            (self.main_layout.size().0 / pango::SCALE) as u16 + 2 * crate::config::TEXT_MARGIN;
        let status_position = self.width - status_width;

        self.draw_rectangle(status_position, status_width, crate::config::BAR_COLOR);
        self.draw_main_text(status_position, status, crate::config::BAR_TEXT_COLOR);
    }

    fn draw_rectangle(&self, x: u16, width: u16, color: u32) {
        self.context
            .rectangle(x as f64, 0., width as f64, crate::config::BAR_HEIGHT as f64);
        self.set_color(color);
        self.context.fill().unwrap();
    }

    fn draw_tag_rectangle(&self, x: u16, color: u32) {
        let margin = config::BAR_HEIGHT as f64 / 16.;
        let size = config::BAR_HEIGHT as f64 / 4.;
        self.context
            .rectangle(x as f64 + margin, margin, size, size);
        self.set_color(color);
        self.context.fill().unwrap();
    }

    fn draw_main_text(&self, x: u16, text: &str, color: u32) {
        self.main_layout.set_text(text);
        self.context.move_to(
            (x + crate::config::TEXT_MARGIN) as f64,
            self.main_text_middle,
        );
        self.set_color(color);
        pangocairo::show_layout(&self.context, &self.main_layout);
    }

    fn draw_tag_text(&self, x: u16, text: &str, color: u32) {
        self.tag_layout.set_text(text);
        self.context.move_to(
            (x + crate::config::TEXT_MARGIN) as f64,
            self.tag_text_middle,
        );
        self.set_color(color);
        pangocairo::show_layout(&self.context, &self.tag_layout);
    }

    fn set_color(&self, color: u32) {
        self.context.set_source_rgb(
            (color >> 16) as f64 / 255.,
            (color >> 8 & 0x0000ff) as f64 / 255.,
            (color & 0x0000ff) as f64 / 255.,
        );
    }
}
