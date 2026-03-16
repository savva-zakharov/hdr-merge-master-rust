use iced::keyboard;
use iced::time;
use iced::widget::{
    button,
    center_x,
    checkbox,
    column,
    container,
    pick_list,
    progress_bar,
    radio,
    row,
    rule,
    scrollable,
    slider,
    space,
    text,
    text_input,
    toggler,
};

use iced::{ Center, Element, Fill, Shrink, Subscription, Theme };

// Header size multiplier
const HEADER_SCALE: f32 = 2.0;

// use iced::keyboard::Key::Character;

pub fn main() -> iced::Result {
    iced::application(Styling::default, Styling::update, Styling::view)
        .subscription(Styling::subscription)
        .theme(Styling::theme)
        .run()
}



impl Default for Styling {
    fn default() -> Self {
        Self {
            theme: None,
            input_value: String::default(),
            slider_value: 0.0,
            checkbox_value: false,
            toggler_1_value: false,
            toggler_2_value: false,
            toggler_3_value: false,
            toggler_fast: false,
            toggler_good: false,
            toggler_cheap: false,
            card_state: String::default(),
            selection: None,
            text_size: 16,
            progress_value: 0.0,
            progress_running: false,
        }
    }
}

struct Styling {
    theme: Option<Theme>,
    input_value: String,
    slider_value: f32,
    checkbox_value: bool,
    toggler_1_value: bool,
    toggler_2_value: bool,
    toggler_3_value: bool,
    toggler_fast: bool,
    toggler_good: bool,
    toggler_cheap: bool,
    card_state: String,
    selection: Option<Choice>,
    text_size: u32,
    progress_value: f32,
    progress_running: bool,
}

#[derive(Debug, Clone)]
enum Message {
    ThemeChanged(Theme),
    InputChanged(String),
    PrimaryButtonPressed,
    SecondaryButtonPressed,
    SuccessButtonPressed,
    WarningButtonPressed,
    DangerButtonPressed,
    SliderChanged(f32),
    CheckboxToggled(bool),
    RadioSelected(Choice),
    Toggler1Toggled(bool),
    Toggler2Toggled(bool),
    Toggler3Toggled(bool),
    TogglerFast(bool),
    TogglerGood(bool),
    TogglerCheap(bool),
    ClosePanel,
    PreviousTheme,
    NextTheme,
    ClearTheme,
    FontSizeIncreased,
    FontSizeDecreased,
    StartProgress,
    StopProgress,
    ProgressTick,
    SendNotification(String)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Choice {
    A,
    B,
    C,
    All,
}

impl Styling {
    fn update(&mut self, message: Message) {
        match message {
            Message::SendNotification(_content) => {
            }
            Message::ThemeChanged(theme) => {
                self.theme = Some(theme);
            }
            Message::InputChanged(value) => {
                self.input_value = value;
            }
            Message::PrimaryButtonPressed => {
                if self.card_state != "Primary" {
                    self.card_state = "Primary".to_string();
                } else {
                    self.card_state = "".to_string();
                }
            }
            Message::SecondaryButtonPressed => {
                if self.card_state != "Secondary" {
                    self.card_state = "Secondary".to_string();
                } else {
                    self.card_state = "".to_string();
                }
            }
            Message::SuccessButtonPressed => {
                if self.card_state != "Success" {
                    self.card_state = "Success".to_string();
                } else {
                    self.card_state = "".to_string();
                }
            }
            Message::WarningButtonPressed => {
                if self.card_state != "Warning" {
                    self.card_state = "Warning".to_string();
                } else {
                    self.card_state = "".to_string();
                }
            }
            Message::DangerButtonPressed => {
                if self.card_state != "Danger" {
                    self.card_state = "Danger".to_string();
                } else {
                    self.card_state = "".to_string();
                }
            }
            Message::ClosePanel => {
                self.card_state = "".to_string();
            }

            Message::SliderChanged(value) => {
                self.slider_value = value;
            }
            Message::CheckboxToggled(value) => {
                self.checkbox_value = value;
            }
            Message::Toggler1Toggled(value) => {
                self.toggler_1_value = value;
                self.toggler_2_value = false;
                self.toggler_3_value = false;
            }
            Message::Toggler2Toggled(value) => {
                self.toggler_2_value = value;
                self.toggler_1_value = false;
                self.toggler_3_value = false;
            }
            Message::Toggler3Toggled(value) => {
                self.toggler_3_value = value;
                self.toggler_1_value = false;
                self.toggler_2_value = false;
            }

            

            // FAST
            // GOOD
            // CHEAP
            Message::TogglerFast(value) => {
                self.toggler_fast = value;
                if self.toggler_good && self.toggler_cheap && value {
                    self.toggler_good = false;
                }
            }
            Message::TogglerGood(value) => {
                self.toggler_good = value;
                if self.toggler_fast && self.toggler_cheap && value {
                    self.toggler_cheap = false;
                }
            }
            Message::TogglerCheap(value) => {
                self.toggler_cheap = value;
                if self.toggler_fast && self.toggler_good && value {
                    self.toggler_fast = false;
                }
            }

            Message::RadioSelected(choice) => {
                self.selection = Some(choice);
            }

            Message::PreviousTheme | Message::NextTheme => {
                let current = Theme::ALL.iter().position(
                    |candidate| self.theme.as_ref() == Some(candidate)
                );

                self.theme = Some(
                    if matches!(message, Message::NextTheme) {
                        Theme::ALL[
                            current.map(|current| current + 1).unwrap_or(0) % Theme::ALL.len()
                        ].clone()
                    } else {
                        let current = current.unwrap_or(0);

                        if current == 0 {
                            Theme::ALL.last().expect("Theme::ALL must not be empty").clone()
                        } else {
                            Theme::ALL[current - 1].clone()
                        }
                    }
                );
            }
            Message::ClearTheme => {
                self.theme = None;
            }
            Message::FontSizeIncreased => {
                self.text_size = (self.text_size + 2).min(72);
            }
            Message::FontSizeDecreased => {
                self.text_size = self.text_size.saturating_sub(2).max(8);
            }
            Message::StartProgress => {
                self.progress_running = true;
                self.progress_value = 0.0;
            }
            Message::StopProgress => {
                self.progress_running = false;
                self.progress_value = 0.0;
            }
            Message::ProgressTick => {
                if self.progress_running {
                    self.progress_value = (self.progress_value + 1.0).min(100.0);
                }
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let choose_theme = column![
            row![
                text("Theme:").size(self.text_size),
                button(text("<").size(self.text_size).width(10)).on_press(Message::PreviousTheme),
                pick_list(Theme::ALL, self.theme.as_ref(), Message::ThemeChanged)
                    .width(Fill)
                    .placeholder("System"),
                button(text(">").size(self.text_size).width(10)).on_press(Message::NextTheme),
                button(text("X").size(self.text_size).width(10)).on_press(Message::ClearTheme)
            ].spacing(10)
             .align_y(Center)
        ].spacing(20);

        // let theme_next = button(text(">").width(10)).on_press(Message::NextTheme);
        // let theme_previous = button(text("<").width(10)).on_press(Message::PreviousTheme);
        // let theme_clear = button(text("X").width(10)).on_press(Message::ClearTheme);

        let text_input = text_input("Type something...", &self.input_value)
            .on_input(Message::InputChanged)
            .padding(self.text_size as f32 * 0.6)
            .size(self.text_size);

        let buttons = {
            let button_config = [
                ("Primary", button::secondary, Message::PrimaryButtonPressed),
                ("Secondary", button::secondary, Message::SecondaryButtonPressed),
                ("Success", button::secondary, Message::SuccessButtonPressed),
                ("Warning", button::secondary, Message::WarningButtonPressed),
                ("Danger", button::secondary, Message::DangerButtonPressed),
            ];

            column!(
                if self.checkbox_value {
                    row(
                        button_config.into_iter().map(|(label, _style, msg)| {
                            let is_active = self.card_state == label;
                            let btn = button(text(label).size(self.text_size).width(Fill).center()).padding(self.text_size as f32 * 0.6);
                            if is_active {
                                btn.style(button::primary).on_press(msg).into()
                            } else {
                                btn.style(button::secondary).on_press(msg).into()
                            }
                        })
                    )
                    .spacing(10)
                    .align_y(Center)
                } else {
                    row(
                        button_config.into_iter().map(|(label, style, _)| {
                            button(text(label).size(self.text_size).width(Fill).center())
                                .padding(self.text_size as f32 * 0.6)
                                .style(style)
                                .into()
                        })
                    )
                    .spacing(10)
                    .align_y(Center)
                }
            )
                .spacing(10)
        };

        let a: Element<'_, Message> = radio("A", Choice::A, self.selection, Message::RadioSelected).size(self.text_size).into();

        let b: Element<'_, Message> = radio("B", Choice::B, self.selection, Message::RadioSelected).size(self.text_size).into();

        let c: Element<'_, Message> = radio("C", Choice::C, self.selection, Message::RadioSelected).size(self.text_size).into();

        let all: Element<'_, Message> = radio("All", Choice::All, self.selection, Message::RadioSelected).size(self.text_size).into();

        let slider = || slider(0.0..=100.0, self.slider_value, Message::SliderChanged).step(1.0);

        let progress = || progress_bar(0.0..=100.0, self.slider_value);

        let scroll_me = scrollable(column![text("Scroll me!").size(self.text_size), space().height(800), text("You did it!").size(self.text_size)])
            .width(Fill)
            .height(Fill)
            .auto_scroll(true);
        let scroll_me_2 = scrollable(column![text("Scroll me!").size(self.text_size), space().height(800), text("You did it!").size(self.text_size)])
            .width(Fill)
            .height(Fill)
            .auto_scroll(true);

        let check = checkbox(self.checkbox_value)
            .label("Enable Panel")
            .on_toggle(Message::CheckboxToggled)
            .size(self.text_size);

        let check_disabled = checkbox(!self.checkbox_value).label("Disabled").size(self.text_size);

        let toggle1 = toggler(self.toggler_1_value)
            .label("Toggle me!")
            .on_toggle(Message::Toggler1Toggled)
            .spacing(self.text_size as f32 * 0.6)
            .size(self.text_size);
        let toggle2 = toggler(self.toggler_2_value)
            .label("No, toggle me!")
            .on_toggle(Message::Toggler2Toggled)
            .spacing(self.text_size as f32 * 0.6)
            .size(self.text_size);
        let toggle3 = toggler(self.toggler_3_value)
            .label("No, choose me!")
            .on_toggle(Message::Toggler3Toggled)
            .spacing(self.text_size as f32 * 0.6)
            .size(self.text_size);

        let toggle_fast = toggler(self.toggler_fast)
            .label("Fast")
            .on_toggle(Message::TogglerFast)
            .spacing(self.text_size as f32 * 0.6)
            .size(self.text_size);
        let toggle_good = toggler(self.toggler_good)
            .label("Good")
            .on_toggle(Message::TogglerGood)
            .spacing(self.text_size as f32 * 0.6)
            .size(self.text_size);
        let toggle_cheap = toggler(self.toggler_cheap)
            .label("Cheap")
            .on_toggle(Message::TogglerCheap)
            .spacing(self.text_size as f32 * 0.6)
            .size(self.text_size);

        let disabled_toggle = toggler(!self.toggler_3_value).label("Disabled").spacing(self.text_size as f32 * 0.6).size(self.text_size);

        let card_primary = {
            container(column![text("Card Example").size((self.text_size as f32 * HEADER_SCALE) as u32), slider(), progress()].spacing(self.text_size as f32 * 1.2))
                .width(Fill)
                .padding(self.text_size as f32 * 1.2)
                .style(container::bordered_box)
                .style(container::primary)
        };
        let card_secondary = {
            container(
                column![
                    text(" Secondary Card Example").size((self.text_size as f32 * HEADER_SCALE) as u32),
                    text(
                        "This is a secondary card style. It goes on to explain all the things about it. I don't want to set the world on fire, I just wawnt to start a flame in yiour heart."
                    ).size(self.text_size),
                    choose_theme
                ].spacing(self.text_size as f32 * 1.2)
            )
                .width(Fill)
                .padding(self.text_size as f32 * 1.2)
                .style(container::bordered_box)
                .style(container::secondary)
        };
        let card_success = {
            container(column![text("Success Card").size((self.text_size as f32 * HEADER_SCALE) as u32),
            button(text("Send Notification").size(self.text_size)).on_press(Message::SendNotification("This is a success notification!".into()))
            ].spacing(self.text_size as f32 * 1.2),)
                .width(Fill)
                .padding(self.text_size as f32 * 1.2)
                .style(container::bordered_box)
                .style(container::success)
        };
        let card_warning = {
            container(
                column![
                    text("Warning Card").size((self.text_size as f32 * HEADER_SCALE) as u32),
                    text(
                        "This is a warning card style. It goes on to explain all the things about it. I don't want to set the world on fire, I just wawnt to start a flame in yiour heart."
                    ).size(self.text_size),

                    slider(),
                    progress(),

                ].spacing(self.text_size as f32 * 1.2)
            )
                .width(Fill)
                .padding(self.text_size as f32 * 1.2)
                .style(container::bordered_box)
                .style(container::warning)
        };
        let card_danger = {
            container(
                column![
                    text("Danger Card").size((self.text_size as f32 * HEADER_SCALE) as u32),
                    scroll_me_2,
                    text(
                        "This is a danger card style. It goes on to explain all the things about it. I don't want to set the world on fire, I just wawnt to start a flame in yiour heart."
                    ).size(self.text_size),
                    if self.progress_running {
                        row![
                            progress_bar(0.0..=100.0, self.progress_value),
                            button(text("Stop").size(self.text_size)).on_press(Message::StopProgress)
                        ]
                        .spacing(10)
                        .align_y(Center)
                    } else {
                        row![button(text("Start 10s Progress").size(self.text_size)).on_press(Message::StartProgress)]
                            .spacing(10)
                            .align_y(Center)
                    }
                ].spacing(self.text_size as f32 * 1.2)
                .height(300)
            )
                .width(Fill)
                .padding(self.text_size as f32 * 1.2)
                .style(container::bordered_box)
                .style(container::danger)
        };
        let card_blank = {
            container(column![].spacing(20))
                .width(Fill)
                .padding(20)
        };

        let content = column![

            // choose_theme,
            // row![
            //     theme_previous,
            //     theme_next,
            //     theme_clear
            // ]
            //     .spacing(10)
            //     .align_y(Center),
            rule::horizontal(1),
            text_input,
            buttons,
            slider(),
            progress(),
            row![
                scroll_me.height(200),
                rule::vertical(1),
                column![check, check_disabled, disabled_toggle, toggle1, toggle2, toggle3].spacing(self.text_size as f32 * 0.6),
                rule::vertical(1),
                column![toggle_fast, toggle_good, toggle_cheap].spacing(10).spacing(self.text_size as f32 * 0.6),
                rule::vertical(1),
                column![a, b, c, all].spacing(self.text_size as f32 * 0.6)
            ]
                .spacing(self.text_size as f32 * 0.6)
                .height(Shrink)
                .align_y(Center),
            if self.checkbox_value {
                if self.card_state == "Success" {
                    card_success
                } else if self.card_state == "Warning" {
                    card_warning
                } else if self.card_state == "Danger" {
                    card_danger
                } else if self.card_state == "Primary" {
                    card_primary
                } else if self.card_state == "Secondary" {
                    card_secondary
                } else {
                    card_blank
                }
            } else {
                card_blank
            }
        ]
            .spacing(self.text_size as f32 * 1.2)
            .padding(self.text_size as f32 * 1.2)
            .max_width(self.text_size as f32 * 40.0);

        scrollable(center_x(content)).spacing(self.text_size as f32 * 0.6).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let keyboard = keyboard::listen().filter_map(|event| {
            let keyboard::Event::KeyPressed { key, repeat: false, .. } = event else {
                return None;
            };

            match key {
                keyboard::Key::Named(keyboard::key::Named::ArrowLeft) => {
                    Some(Message::PreviousTheme)
                }
                keyboard::Key::Named(keyboard::key::Named::ArrowRight) => {
                    Some(Message::NextTheme)
                }
                keyboard::Key::Named(keyboard::key::Named::Space) => Some(Message::ClearTheme),
                keyboard::Key::Named(keyboard::key::Named::Escape) => Some(Message::ClosePanel),
                keyboard::Key::Character(c) if c.as_str() == "=" => {
                    Some(Message::FontSizeIncreased)
                }
                keyboard::Key::Character(c) if c.as_str() == "-" => {
                    Some(Message::FontSizeDecreased)
                }
                _ => None,
            }
        });

        let progress = if self.progress_running {
            time::every(std::time::Duration::from_millis(100))
                .map(|_| Message::ProgressTick)
        } else {
            Subscription::none()
        };

        Subscription::batch([keyboard, progress])
    }

    fn theme(&self) -> Option<Theme> {
        self.theme.clone()
    }
}
