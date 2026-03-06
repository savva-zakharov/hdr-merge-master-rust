//! Profile manager dialog for managing PP3 profiles

use iced::widget::{button, center_x, container, rule, scrollable, text, Column, Row, space, Space, text_input};
use iced::{Element, Length};
use iced::Length::Fill;
use iced::widget::button::background;
use crate::config::Profile;

#[derive(Debug, Clone)]
pub enum ProfileMessage {
    Add,
    Delete(usize),
    Edit(usize),
    ClearAll,
    Close,
    NameEdit(String, usize),
    PathEdit(String, usize),
    TagEdit(String, usize),
}

pub struct ProfileManagerDialog {
    pub show: bool,
}

impl ProfileManagerDialog {
    pub fn new() -> Self {
        Self { show: false }
    }

    pub fn open(&mut self) {
        self.show = true;
    }

    pub fn close(&mut self) {
        self.show = false;
    }

    pub fn view<'a>(&self, profiles: &'a [String], config_profiles: &'a [Profile], uiscale: f32) -> Element<'a, ProfileMessage> {
        let mut content = Column::new().spacing(8.0 * uiscale).padding(10.0 * uiscale);

        let mut profile_rows = Column::new().spacing(4.0 * uiscale);
        //Buttons
        let add_btn = button(text("Add")
            .size(16.0 * uiscale))
            .style(button::success)
            .on_press(ProfileMessage::Add);
        let clear_all_btn = button(text("Clear All")
            .size(16.0 * uiscale))
            .style(button::danger)
            .on_press(ProfileMessage::ClearAll);
        let close_btn = button(text("Close")
            .size(16.0 * uiscale))
            .style(button::warning)
            .on_press(ProfileMessage::Close);

        let buttons = Row::new()
            .push(add_btn)
            .push(clear_all_btn)
            .push(space().width(Fill))
            .push(close_btn)
            .spacing(10.0 * uiscale);

        content = content.push(buttons);

        // Header row
        let header = Row::new()
            .push(text("Name").width(Length::FillPortion(2)))
            .push(text("File Path").width(Length::FillPortion(3)))
            .push(text("Tag").width(Length::Fill))
            .push(text("Actions").width(Length::FillPortion(2)))
            .spacing(10.0 * uiscale)
            .padding([4.0 * uiscale, 10.0 * uiscale]);
        profile_rows = profile_rows.push(header);

        // Profile rows
        for (i, name) in profiles.iter().enumerate() {
            let profile = config_profiles.get(i);
            let path = profile.map(|p| p.file_path.as_str()).unwrap_or("");
            let tag = profile.map(|p| p.tag.as_str()).unwrap_or("");

            let name_input = text_input("", name).width(Length::FillPortion(2))
                .on_input(move |value| ProfileMessage::NameEdit(value, i));
            let path_input = text_input("", path).width(Length::FillPortion(3))
                .on_input(move |value| ProfileMessage::PathEdit(value, i));
            let tag_input = text_input("", tag).width(Fill)
                .on_input(move |value| ProfileMessage::TagEdit(value, i));

            let profile_row = Row::new()
                .push(name_input)
                .push(path_input)
                .push(tag_input)
                .push(
                    Row::new()
                        .push(button(text("Edit").size(12.0 * uiscale)).style(button::secondary).on_press(ProfileMessage::Edit(i)))
                        .push(button(text("Delete").size(12.0 * uiscale)).style(button::danger).on_press(ProfileMessage::Delete(i)))
                        .spacing(5.0 * uiscale)
                        .width(Length::FillPortion(2)),
                )
                .spacing(10.0 * uiscale)
                .padding([4.0 * uiscale, 10.0 * uiscale]);
            profile_rows = profile_rows.push(horizontal_rule(2));
            profile_rows = profile_rows.push(profile_row);
        }

        let profile_scroll = container(scrollable(profile_rows).height(Length::Fixed(300.0 * uiscale))).width(Length::Fill).height(Length::Fill).style(container::bordered_box);
        content = content.push(profile_scroll);

        // content = content.push(horizontal_rule((20.0 * uiscale) as u16));

        // Buttons




        center_x(container(content)
            .height(400.0 * uiscale)
            .max_width(1000.0 * uiscale)
            .padding(10.0 * uiscale)
            .style(container::rounded_box))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn horizontal_space() -> Element<'static, ProfileMessage> {
    Space::new().into()
}

fn horizontal_rule(thickness: u16) -> Element<'static, ProfileMessage> {
    rule::horizontal(thickness as u32).into()
}
