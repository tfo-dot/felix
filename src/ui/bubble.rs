use gtk4::prelude::*;

pub struct SpeechBubble {
    pub container: gtk4::Box,
    label: gtk4::Label,
}

impl SpeechBubble {
    pub fn new() -> Self {
        let container = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Vertical)
            .halign(gtk4::Align::Center)
            .valign(gtk4::Align::End)
            .visible(false) // hidden by default
            .build();

        let body = gtk4::Box::builder()
            .css_classes(vec!["bubble-body".to_string()])
            .build();

        let label = gtk4::Label::builder()
            .wrap(true)
            .max_width_chars(25)
            .justify(gtk4::Justification::Center)
            .build();

        body.append(&label);
        container.append(&body);

        let tail = gtk4::DrawingArea::builder()
            .content_width(16)
            .content_height(8)
            .halign(gtk4::Align::Center)
            .build();

        tail.set_draw_func(move |_area, cr, _width, _height| {
            cr.set_source_rgba(30.0 / 255.0, 30.0 / 255.0, 40.0 / 255.0, 0.95);
            cr.move_to(0.0, 0.0);
            cr.line_to(16.0, 0.0);
            cr.line_to(8.0, 8.0);
            cr.close_path();
            let _ = cr.fill();
        });

        container.append(&tail);

        Self { container, label }
    }

    pub fn set_text(&self, text: &str) {
        self.label.set_text(text);
        self.container.set_visible(!text.is_empty());
    }

    pub fn hide(&self) {
        self.container.set_visible(false);
    }
}
