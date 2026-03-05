//! Setup dialog for configuring application settings

use iced::Length::Fill;
use iced::widget::{
    button, center_x, column, container, row, rule, scrollable, space,
    text, text_input, toggler, Column, Row,
};
use iced::{Element, Length};

use crate::config::Config;

#[derive(Debug, Clone)]
pub enum DialogMessage {
    ThreadsChanged(String),
    ToggleRecursive(bool),
    ToggleCleanup(bool),
    ToggleAlign(bool),
    ToggleOpenCvAlign(bool),
    ToggleOpenCvMerge(bool),
    ToggleOpenCvMergeRobertson(bool),
    ToggleOpenCvTonemap(bool),
    RecursiveMaxDepthChanged(String),
    TonemapOperatorChanged(String),
    TonemapIntensityChanged(String),
    TonemapContrastChanged(String),
    TonemapSaturationChanged(String),
    ProcessedExtensionsChanged(String),
    RawExtensionsChanged(String),
    RecursiveIgnoreFoldersChanged(String),
    AlignImageStackPathChanged(String),
    BlenderPathChanged(String),
    LuminancePathChanged(String),
    RawtherapeePathChanged(String),
    ProcessPage,
    FolderPage,
    GuiPage,
    ExePage,
    Save,
    Cancel,
}

pub struct SetupDialog {
    pub show: bool,
    // Temporary storage for editing
    threads_str: String,
    processed_exts_str: String,
    raw_exts_str: String,
    ignore_folders_str: String,
    tonemap_operator: String,
    tonemap_intensity_str: String,
    tonemap_contrast_str: String,
    tonemap_saturation_str: String,
    recursive_max_depth_str: String,
    align_image_stack_exe: String,
    blender_exe: String,
    luminance_cli_exe: String,
    rawtherapee_cli_exe: String,
    uiscale: f32,
    setup_page: String,
}

impl SetupDialog {
    pub fn new() -> Self {
        Self {
            show: false,
            threads_str: String::new(),
            processed_exts_str: String::new(),
            raw_exts_str: String::new(),
            ignore_folders_str: String::new(),
            tonemap_operator: "Reinhard".to_string(),
            tonemap_intensity_str: String::new(),
            tonemap_contrast_str: String::new(),
            tonemap_saturation_str: String::new(),
            recursive_max_depth_str: String::new(),
            align_image_stack_exe: String::new(),
            blender_exe: String::new(),
            luminance_cli_exe: String::new(),
            rawtherapee_cli_exe: String::new(),
            uiscale: 1.0,
            setup_page: "process_page".to_string(),
        }
    }

    pub fn open(&mut self, config: &Config) {
        self.threads_str = config.gui_settings.threads.to_string();
        self.processed_exts_str = config.gui_settings.processed_extensions.join(",");
        self.raw_exts_str = config.gui_settings.raw_extensions.join(",");
        self.ignore_folders_str = config.gui_settings.recursive_ignore_folders.join(",");
        self.tonemap_operator = config.gui_settings.tonemap_operator.clone();
        self.tonemap_intensity_str = config.gui_settings.tonemap_intensity.to_string();
        self.tonemap_contrast_str = config.gui_settings.tonemap_contrast.to_string();
        self.tonemap_saturation_str = config.gui_settings.tonemap_saturation.to_string();
        self.recursive_max_depth_str = config.gui_settings.recursive_max_depth.to_string();
        self.align_image_stack_exe = config.exe_paths.align_image_stack_exe.clone();
        self.blender_exe = config.exe_paths.blender_exe.clone();
        self.luminance_cli_exe = config.exe_paths.luminance_cli_exe.clone();
        self.rawtherapee_cli_exe = config.exe_paths.rawtherapee_cli_exe.clone();
        self.show = true;
    }

    pub fn save(&mut self, config: &mut Config) {
        // Update config with current values
        config.gui_settings.threads = self.threads_str.parse().unwrap_or(6);
        config.gui_settings.processed_extensions = self
            .processed_exts_str
            .split(',')
            .map(|s: &str| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        config.gui_settings.raw_extensions = self
            .raw_exts_str
            .split(',')
            .map(|s: &str| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        config.gui_settings.recursive_ignore_folders = self
            .ignore_folders_str
            .split(',')
            .map(|s: &str| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        config.gui_settings.tonemap_operator = self.tonemap_operator.clone();
        config.gui_settings.tonemap_intensity = self.tonemap_intensity_str.parse().unwrap_or(1.0);
        config.gui_settings.tonemap_contrast = self.tonemap_contrast_str.parse().unwrap_or(1.0);
        config.gui_settings.tonemap_saturation = self.tonemap_saturation_str.parse().unwrap_or(1.0);
        config.gui_settings.recursive_max_depth = self.recursive_max_depth_str.parse().unwrap_or(1);
        config.exe_paths.align_image_stack_exe = self.align_image_stack_exe.clone();
        config.exe_paths.blender_exe = self.blender_exe.clone();
        config.exe_paths.luminance_cli_exe = self.luminance_cli_exe.clone();
        config.exe_paths.rawtherapee_cli_exe = self.rawtherapee_cli_exe.clone();
    }

    pub fn cancel(&mut self) {
        self.show = false;
    }

    pub fn update(&mut self, message: DialogMessage, config: &mut Config) {
        match message {
            DialogMessage::ThreadsChanged(value) => {
                self.threads_str = value;
            }
            DialogMessage::ToggleRecursive(value) => {
                config.gui_settings.do_recursive = value;
            }
            DialogMessage::ToggleCleanup(value) => {
                config.gui_settings.do_cleanup = value;
            }
            DialogMessage::ToggleAlign(value) => {
                config.gui_settings.do_align = value;
            }
            DialogMessage::ToggleOpenCvAlign(value) => {
                config.gui_settings.use_opencv_align = value;
            }
            DialogMessage::ToggleOpenCvMerge(value) => {
                config.gui_settings.use_opencv_merge = value;
                config.gui_settings.use_opencv_merge_robertson = false; // Ensure Robertson is disabled if Merge is enabled
            }
            DialogMessage::ToggleOpenCvMergeRobertson(value) => {
                config.gui_settings.use_opencv_merge_robertson = value;
                config.gui_settings.use_opencv_merge = false; // Ensure Debevec is also disabled if Robertson is enabled
            }
            DialogMessage::ToggleOpenCvTonemap(value) => {
                config.gui_settings.use_opencv_tonemap = value;
            }
            DialogMessage::RecursiveMaxDepthChanged(value) => {
                self.recursive_max_depth_str = value;
            }
            DialogMessage::TonemapOperatorChanged(value) => {
                self.tonemap_operator = value;
            }
            DialogMessage::TonemapIntensityChanged(value) => {
                self.tonemap_intensity_str = value;
            }
            DialogMessage::TonemapContrastChanged(value) => {
                self.tonemap_contrast_str = value;
            }
            DialogMessage::TonemapSaturationChanged(value) => {
                self.tonemap_saturation_str = value;
            }
            DialogMessage::ProcessedExtensionsChanged(value) => {
                self.processed_exts_str = value;
            }
            DialogMessage::RawExtensionsChanged(value) => {
                self.raw_exts_str = value;
            }
            DialogMessage::RecursiveIgnoreFoldersChanged(value) => {
                self.ignore_folders_str = value;
            }
            DialogMessage::AlignImageStackPathChanged(value) => {
                self.align_image_stack_exe = value;
            }
            DialogMessage::BlenderPathChanged(value) => {
                self.blender_exe = value;
            }
            DialogMessage::LuminancePathChanged(value) => {
                self.luminance_cli_exe = value;
            }
            DialogMessage::RawtherapeePathChanged(value) => {
                self.rawtherapee_cli_exe = value;
            }
            DialogMessage::Save | DialogMessage::Cancel => {}
            DialogMessage::ProcessPage => self.setup_page = "process_page".to_string(),
            DialogMessage::FolderPage => self.setup_page = "folder_page".to_string(),
            DialogMessage::GuiPage => self.setup_page = "gui_page".to_string(),
            DialogMessage::ExePage => {
                self.setup_page = "exe_page".to_string();
            }
        }
    }

    pub fn view(&self, config: &Config) -> Element<'_, DialogMessage> {
        // let buttons = row![
        //     button(text("Process").width(Fill).center()).on_press(DialogMessage::ProcessPage).style(button::secondary),
        //     button(text("Folder Setup").width(Fill).center()).on_press(DialogMessage::FolderPage).style(button::secondary),
        //     button(text("Gui Setup").width(Fill).center()).on_press(DialogMessage::GuiPage).style(button::secondary),
        //     button(text("Exe Paths").width(Fill).center()).on_press(DialogMessage::ExePage).style(button::secondary),
        //     button(text("Save").width(Fill).center()).on_press(DialogMessage::Save).style(button::success),
        //     button(text("Cancel").width(Fill).center()).on_press(DialogMessage::Cancel).style(button::warning),
        // ];

        let main_buttons = self.main_buttons();
        let process_settings = self.view_process_page(config);
        let folder_settings = text("Folder Settings Placeholder");
        let gui_settings = text("GUI Settings Placeholder");
        let view_exe_paths = self.view_exe_paths(config);

        let content = column![
            main_buttons,
            container(if self.setup_page == "process_page" {
                scrollable(process_settings)
            } else if self.setup_page == "folder_page" {
                scrollable(folder_settings)
            } else if self.setup_page == "gui_page" {
                scrollable(gui_settings)
            } else if self.setup_page == "exe_page" {
                scrollable(view_exe_paths)
            } else {
                scrollable(process_settings)
            },
            ).padding(10.0).style(iced::widget::container::bordered_box).width(1000.0 * self.uiscale),
        ]
        .spacing(10.0)
        .padding(10.0);

        center_x(
            container(content)
                .height(500.0 * self.uiscale)
                .max_width(1000.0 * self.uiscale)
                .padding(10.0 * self.uiscale)
                .style(container::rounded_box),
        )
        .into()
    }

    fn main_buttons(&self) -> Element<'_, DialogMessage> {
        let button_save = button(text("Save").center())
            .on_press(DialogMessage::Save)
            .style(button::success);
        let button_cancel = button(text("Cancel").center())
            .on_press(DialogMessage::Cancel)
            .style(button::warning);

        let button_config = [
            (
                "Process",
                "process_page",
                button::secondary,
                DialogMessage::ProcessPage,
            ),
            (
                "Folder Setup",
                "folder_page",
                button::secondary,
                DialogMessage::FolderPage,
            ),
            (
                "Gui Setup",
                "gui_page",
                button::secondary,
                DialogMessage::GuiPage,
            ),
            (
                "Exe Paths",
                "exe_page",
                button::secondary,
                DialogMessage::ExePage,
            ),
        ];

        let buttons = row(button_config.into_iter().map(|(label, code, style, msg)| {
            let is_active = self.setup_page == code;
            let btn = button(text(label).center());
            if is_active {
                btn.style(button::primary).on_press(msg).into()
            } else {
                btn.style(style).on_press(msg).into()
            }
        }))
        .spacing(10);

        row![buttons,
            text("[DEBUG] uiscale:"),
            text(self.uiscale.to_string()),
            space().width(Fill),
            button_save,
            button_cancel
            ]
            .spacing(10.0)
            .into()
    }

    fn view_process_page(&self, config: &Config) -> Element<'_, DialogMessage> {
        let mut group = Column::new().spacing(8);

        let title = text("GUI Settings").size(16);
        group = group.push(title);
        group = group.push(horizontal_rule(2));

        // Threads
        let threads_row = Row::new()
            .push(text("Threads:"))
            .push(
                text_input("Threads", &self.threads_str)
                    .on_input(DialogMessage::ThreadsChanged)
                    .width(Length::Fixed(60.0)),
            )
            .spacing(10);
        group = group.push(threads_row);

        // Checkboxes
        group = group.push(
            toggler(config.gui_settings.do_recursive)
                .label("Recursive Processing")
                .on_toggle(DialogMessage::ToggleRecursive),
        );
        group = group.push(
            toggler(config.gui_settings.do_cleanup)
                .label("Cleanup Temporary Files")
                .on_toggle(DialogMessage::ToggleCleanup),
        );
        group = group.push(
            toggler(config.gui_settings.do_align)
                .label("Do Align")
                .on_toggle(DialogMessage::ToggleAlign),
        );
        group = group.push(
            toggler(config.gui_settings.use_opencv_align)
                .label("Use OpenCV Align(AlignMTB)")
                .on_toggle(DialogMessage::ToggleOpenCvAlign),
        );
        group = group.push(
            toggler(config.gui_settings.use_opencv_merge)
                .label("Use OpenCV Merge (Debevec)")
                .on_toggle(DialogMessage::ToggleOpenCvMerge),
        );
        group = group.push(
            toggler(config.gui_settings.use_opencv_merge_robertson)
                .label("Use OpenCV Merge (Robertson)")
                .on_toggle(DialogMessage::ToggleOpenCvMergeRobertson),
        );
        group = group.push(
            toggler(config.gui_settings.use_opencv_tonemap)
                .label("Use OpenCV Tone Mapping")
                .on_toggle(DialogMessage::ToggleOpenCvTonemap),
        );

        // Recursive Max Depth
        let max_depth_row = Row::new()
            .push(text("Recursive Max Depth:"))
            .push(
                text_input("Depth", &self.recursive_max_depth_str)
                    .on_input(DialogMessage::RecursiveMaxDepthChanged)
                    .width(Length::Fixed(60.0)),
            )
            .spacing(10);
        group = group.push(max_depth_row);

        group = group.push(horizontal_rule(2));

        // Tone Mapping Operator
        group = group.push(text("Tone Mapping Operator:"));
        let tonemap_row = Row::new()
            .push(
                button(text(if self.tonemap_operator == "Reinhard" {
                    "● Reinhard"
                } else {
                    "○ Reinhard"
                }))
                .on_press(DialogMessage::TonemapOperatorChanged(
                    "Reinhard".to_string(),
                )),
            )
            .push(
                button(text(if self.tonemap_operator == "Drago" {
                    "● Drago"
                } else {
                    "○ Drago"
                }))
                .on_press(DialogMessage::TonemapOperatorChanged("Drago".to_string())),
            )
            .push(
                button(text(if self.tonemap_operator == "Durand" {
                    "● Durand"
                } else {
                    "○ Durand"
                }))
                .on_press(DialogMessage::TonemapOperatorChanged("Durand".to_string())),
            )
            .push(
                button(text(if self.tonemap_operator == "Mantiuk" {
                    "● Mantiuk"
                } else {
                    "○ Mantiuk"
                }))
                .on_press(DialogMessage::TonemapOperatorChanged("Mantiuk".to_string())),
            )
            .spacing(10);
        group = group.push(tonemap_row);

        // Intensity
        let intensity_row = Row::new()
            .push(text("Intensity:"))
            .push(
                text_input("Intensity", &self.tonemap_intensity_str)
                    .on_input(DialogMessage::TonemapIntensityChanged)
                    .width(Length::Fixed(60.0)),
            )
            .spacing(10);
        group = group.push(intensity_row);

        // Contrast
        let contrast_row = Row::new()
            .push(text("Contrast:"))
            .push(
                text_input("Contrast", &self.tonemap_contrast_str)
                    .on_input(DialogMessage::TonemapContrastChanged)
                    .width(Length::Fixed(60.0)),
            )
            .spacing(10);
        group = group.push(contrast_row);

        // Saturation
        let saturation_row = Row::new()
            .push(text("Saturation:"))
            .push(
                text_input("Saturation", &self.tonemap_saturation_str)
                    .on_input(DialogMessage::TonemapSaturationChanged)
                    .width(Length::Fixed(60.0)),
            )
            .spacing(10);
        group = group.push(saturation_row);

        group = group.push(horizontal_rule((self.uiscale * 2.0) as u16));

        // Processed Extensions
        group = group.push(text("Processed Extensions (comma-separated):"));
        group = group.push(
            text_input("Extensions", &self.processed_exts_str)
                .on_input(DialogMessage::ProcessedExtensionsChanged)
                .width(Length::Fixed(300.0)),
        );

        group = group.push(horizontal_rule(2));

        // Raw Extensions
        group = group.push(text("Raw Extensions (comma-separated):"));
        group = group.push(
            text_input("Extensions", &self.raw_exts_str)
                .on_input(DialogMessage::RawExtensionsChanged)
                .width(Length::Fixed(300.0)),
        );

        group = group.push(horizontal_rule(2));

        // Recursive Ignore Folders
        group = group.push(text("Recursive Ignore Folders (comma-separated):"));
        group = group.push(
            text_input("Folders", &self.ignore_folders_str)
                .on_input(DialogMessage::RecursiveIgnoreFoldersChanged)
                .width(Length::Fixed(300.0)),
        );

        column![group].into()
    }
    fn view_exe_paths(&self, config: &Config) -> Element<'_, DialogMessage> {
        let title = text("Executable Paths").size(16);

        // Align Image Stack
        let align_row = Row::new()
            .push(text("Align Image Stack:").width(Length::Fixed(150.0)))
            .push(
                text_input("Path", &self.align_image_stack_exe)
                    .on_input(DialogMessage::AlignImageStackPathChanged)
                    .width(Length::Fill),
            )
            .spacing(10);

        // Blender
        let blender_row = Row::new()
            .push(text("Blender:").width(Length::Fixed(150.0)))
            .push(
                text_input("Path", &self.blender_exe)
                    .on_input(DialogMessage::BlenderPathChanged)
                    .width(Length::Fill),
            )
            .spacing(10);

        // Luminance CLI
        let luminance_row = Row::new()
            .push(text("Luminance CLI:").width(Length::Fixed(150.0)))
            .push(
                text_input("Path", &self.luminance_cli_exe)
                    .on_input(DialogMessage::LuminancePathChanged)
                    .width(Length::Fill),
            )
            .spacing(10.0);

        // Rawtherapee CLI
        let rawtherapee_row = Row::new()
            .push(text("Rawtherapee CLI:").width(Length::Fixed(150.0)))
            .push(
                text_input("Path", &self.rawtherapee_cli_exe)
                    .on_input(DialogMessage::RawtherapeePathChanged)
                    .width(Length::Fill),
            )
            .spacing(10.0);
        column![
            title,
            horizontal_rule(2),
            align_row,
            blender_row,
            luminance_row,
            rawtherapee_row,
        ]
        .max_width(1000 as f32)
        .spacing(self.uiscale as f32 * 10.0).into()
    }
}

fn horizontal_rule(thickness: u16) -> Element<'static, DialogMessage> {
    rule::horizontal(thickness as u32).into()
}
