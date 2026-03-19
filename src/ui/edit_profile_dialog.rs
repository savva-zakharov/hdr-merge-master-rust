//! Edit profile dialog for editing individual profile details

use iced::widget::{button, container, rule, text, text_input, Column, Row, Space};
use iced::{Element, Length};

#[derive(Debug, Clone)]
pub enum EditProfileMessage {
    NameChanged(String),
    FilePathChanged(String),
    TagChanged(String),
    Browse,
    Save,
    Cancel,
}

pub struct EditProfileDialog {
    pub show: bool,
    pub editing_index: Option<usize>,
    pub name: String,
    pub file_path: String,
    pub tag: String,
}

impl EditProfileDialog {
    pub fn new() -> Self {
        Self {
            show: false,
            editing_index: None,
            name: String::new(),
            file_path: String::new(),
            tag: String::new(),
        }
    }

    pub fn open(&mut self, index: usize, name: &str, file_path: &str, tag: &str) {
        self.editing_index = Some(index);
        self.name = name.to_string();
        self.file_path = file_path.to_string();
        self.tag = tag.to_string();
        self.show = true;
    }

    pub fn close(&mut self) {
        self.editing_index = None;
        self.show = false;
    }

    pub fn get_result(&self) -> (Option<usize>, String, String, String) {
        (
            self.editing_index,
            self.name.clone(),
            self.file_path.clone(),
            self.tag.clone(),
        )
    }

    pub fn update(&mut self, message: EditProfileMessage) {
        match message {
            EditProfileMessage::NameChanged(value) => {
                self.name = value;
            }
            EditProfileMessage::FilePathChanged(value) => {
                self.file_path = value;
            }
            EditProfileMessage::TagChanged(value) => {
                self.tag = value;
            }
            EditProfileMessage::Browse | EditProfileMessage::Save | EditProfileMessage::Cancel => {}
        }
    }

    pub fn view(&self, uiscale: f32) -> Element<'_, EditProfileMessage> {
        let mut content = Column::new().spacing(8.0 * uiscale).padding(10.0 * uiscale);

        // Title
        let title = text("Edit Profile").size(16.0 * uiscale);
        content = content.push(title);
        content = content.push(horizontal_rule((2.0 * uiscale) as u16));

        // Name
        let name_row = Row::new()
            .push(text("Name:").width(Length::Fixed(80.0 * uiscale)))
            .push(
                text_input("Name", &self.name)
                    .on_input(EditProfileMessage::NameChanged)
                    .width(Length::Fixed(250.0 * uiscale)),
            )
            .spacing(10.0 * uiscale);
        content = content.push(name_row);

        // File Path
        let browse_btn = button(text("Browse...").size(12.0 * uiscale)).on_press(EditProfileMessage::Browse);
        let path_row = Row::new()
            .push(text("File Path:").width(Length::Fixed(80.0 * uiscale)))
            .push(
                text_input("Path", &self.file_path)
                    .on_input(EditProfileMessage::FilePathChanged)
                    .width(Length::Fixed(250.0 * uiscale)),
            )
            .push(browse_btn)
            .spacing(10.0 * uiscale);
        content = content.push(path_row);

        // Tag
        let tag_row = Row::new()
            .push(text("Tag:").width(Length::Fixed(80.0 * uiscale)))
            .push(
                text_input("Tag", &self.tag)
                    .on_input(EditProfileMessage::TagChanged)
                    .width(Length::Fixed(250.0 * uiscale)),
            )
            .spacing(10.0 * uiscale);
        content = content.push(tag_row);

        content = content.push(horizontal_rule((2.0 * uiscale) as u16));

        // Buttons
        let save_btn = button(text("Save").size(12.0 * uiscale)).on_press(EditProfileMessage::Save);
        let cancel_btn = button(text("Cancel").size(12.0 * uiscale)).on_press(EditProfileMessage::Cancel);

        let buttons = Row::new()
            .push(horizontal_space())
            .push(cancel_btn)
            .push(save_btn)
            .spacing(10.0 * uiscale);

        content = content.push(buttons);

        // Return as a side panel container
        container(content)
            .width(Length::Fixed(400.0 * uiscale))
            .height(Length::Fill)
            .style(container::bordered_box)
            .into()
    }
}

fn horizontal_space() -> Element<'static, EditProfileMessage> {
    Space::new().into()
}

fn horizontal_rule(thickness: u16) -> Element<'static, EditProfileMessage> {
    rule::horizontal(thickness as u32).into()
}
