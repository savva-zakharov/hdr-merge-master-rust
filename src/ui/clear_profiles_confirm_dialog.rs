//! Confirmation dialog for clearing all profiles

use iced::widget::{button, container, rule, text, Column, Row, Space};
use iced::{Element, Length};

#[derive(Debug, Clone)]
pub enum ClearProfilesMessage {
    Confirm,
    Cancel,
}

pub struct ClearProfilesConfirmDialog {
    pub show: bool,
}

impl ClearProfilesConfirmDialog {
    pub fn new() -> Self {
        Self { show: false }
    }

    pub fn open(&mut self) {
        self.show = true;
    }

    pub fn close(&mut self) {
        self.show = false;
    }

    pub fn view(&self, uiscale: f32) -> Element<'_, ClearProfilesMessage> {
        let mut content = Column::new().spacing(8.0 * uiscale).padding(20.0 * uiscale);

        // Title
        let title = text("Clear All Profiles").size(18.0 * uiscale);
        content = content.push(title);
        content = content.push(horizontal_rule((2.0 * uiscale) as u16));

        content = content.push(text("Are you sure you want to delete all profiles?").size(14.0 * uiscale));
        content = content.push(text("This action cannot be undone.").size(12.0 * uiscale));

        content = content.push(horizontal_rule((2.0 * uiscale) as u16));

        // Buttons
        let confirm_btn = button(text("Yes, Clear All").size(12.0 * uiscale)).on_press(ClearProfilesMessage::Confirm);
        let cancel_btn = button(text("Cancel").size(12.0 * uiscale)).on_press(ClearProfilesMessage::Cancel);

        let buttons = Row::new()
            .push(horizontal_space())
            .push(cancel_btn)
            .push(confirm_btn)
            .spacing(10.0 * uiscale);

        content = content.push(buttons);

        // Return as a side panel container
        container(content)
            .width(Length::Fixed(400.0 * uiscale))
            .height(Length::Fill)
            .style(container::danger)
            .into()
    }
}

fn horizontal_space() -> Element<'static, ClearProfilesMessage> {
    Space::new().into()
}

fn horizontal_rule(thickness: u16) -> Element<'static, ClearProfilesMessage> {
    rule::horizontal(thickness as u32).into()
}
