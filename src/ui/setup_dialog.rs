//! Setup dialog for configuring application settings

use iced::Length::Fill;
use iced::widget::{Column, Row, button, center_x, column, container, pick_list, row, rule, scrollable, space, text, text_input, toggler, slider, radio};
use iced::{Alignment, Element, Length, Theme};

use crate::config::Config;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum DialogMessage {
    ThreadsChanged(String),
    ToggleRecursive(bool),
    ToggleCleanup(bool),
    ToggleAlign(bool),
    ToggleOpenCvAlign(bool),
    ToggleAlignImageStack(bool),
    ToggleBlenderMerge(bool),
    ToggleOpenCvDebevec(bool),
    ToggleOpenCvMergeRobertson(bool),
    ToggleRustMerge(bool),
    ToggleRustMergeDebug(bool),
    ToggleOpenCvTonemap(bool),
    ToggleLuminanceTonemap(bool),
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
    ThemeChanged(String),
    PreviousTheme,
    NextTheme,
    ClearTheme,
    UIScaleIncreased,
    UIScaleDecreased,
    UIScaleSliderChanged(f32),
    UIScaleSliderReleased(f32),
    TonemapOperatorSelected(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToneMapChoice {
    Reinhard,
    Drago,
    Durand,
    Mantiuk,
}

impl ToneMapChoice {
    fn from_operator(op: &str) -> Self {
        match op {
            "Drago" => ToneMapChoice::Drago,
            "Durand" => ToneMapChoice::Durand,
            "Mantiuk" => ToneMapChoice::Mantiuk,
            _ => ToneMapChoice::Reinhard,
        }
    }
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
    setup_page: String,
    uiscale_slider_value: f32,  // Temporary value while dragging slider
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
            setup_page: "process_page".to_string(),
            uiscale_slider_value: 1.0,
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
        self.uiscale_slider_value = config.gui_settings.uiscale;
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
                config.gui_settings.use_align_image_stack = !value;
            }
            DialogMessage::ToggleAlignImageStack(value) => {
                config.gui_settings.use_align_image_stack = value;
                config.gui_settings.use_opencv_align = !value;
            }
            DialogMessage::ToggleBlenderMerge(value) => {
                // Mutually exclusive: only one merge method can be active
                config.gui_settings.use_blender_merge = value;
                config.gui_settings.use_opencv_debevec = false;
                config.gui_settings.use_opencv_merge_robertson = false;
                config.gui_settings.use_rust_merge = false;
            }
            DialogMessage::ToggleOpenCvDebevec(value) => {
                // Mutually exclusive: only one merge method can be active
                config.gui_settings.use_opencv_debevec = value;
                config.gui_settings.use_blender_merge = false;
                config.gui_settings.use_opencv_merge_robertson = false;
                config.gui_settings.use_rust_merge = false;
            }
            DialogMessage::ToggleOpenCvMergeRobertson(value) => {
                // Mutually exclusive: only one merge method can be active
                config.gui_settings.use_opencv_merge_robertson = value;
                config.gui_settings.use_blender_merge = false;
                config.gui_settings.use_opencv_debevec = false;
                config.gui_settings.use_rust_merge = false;
            }
            DialogMessage::ToggleRustMerge(value) => {
                // Mutually exclusive: only one merge method can be active
                config.gui_settings.use_rust_merge = value;
                config.gui_settings.use_blender_merge = false;
                config.gui_settings.use_opencv_debevec = false;
                config.gui_settings.use_opencv_merge_robertson = false;
            }
            DialogMessage::ToggleRustMergeDebug(value) => {
                config.gui_settings.rust_merge_debug_export = value;
            }
            DialogMessage::ToggleOpenCvTonemap(value) => {
                config.gui_settings.use_opencv_tonemap = value;
                config.gui_settings.use_luminance_tonemap = !value;
            }
            DialogMessage::ToggleLuminanceTonemap(value) => {
                config.gui_settings.use_luminance_tonemap = value;
                config.gui_settings.use_opencv_tonemap = !value;
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
            DialogMessage::ThemeChanged(_)
            | DialogMessage::PreviousTheme
            | DialogMessage::NextTheme
            | DialogMessage::ClearTheme => {}
            DialogMessage::UIScaleIncreased
            | DialogMessage::UIScaleDecreased => {}
            DialogMessage::UIScaleSliderChanged(value) => {
                // Track slider value locally while dragging (don't forward to parent yet)
                self.uiscale_slider_value = value;
            }
            DialogMessage::UIScaleSliderReleased(_) => {}  // Forward to parent
            DialogMessage::TonemapOperatorSelected(operator) => {
                self.tonemap_operator = operator;
            }
        }
    }

    pub fn view(&self, config: &Config, uiscale: f32) -> Element<'_, DialogMessage> {

        let main_buttons = self.main_buttons(uiscale);
        let process_settings = self.view_process_page(config, uiscale);
        let folder_settings = self.view_folder_page(config, uiscale);
        let gui_settings = self.view_gui_page(config, uiscale);
        let view_exe_paths = self.view_exe_paths(config, uiscale);

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
            },)
            .padding(10.0 * uiscale)
            .style(iced::widget::container::bordered_box)
            // .width(1000.0 * uiscale),
        ]
        .spacing(10.0 * uiscale)
        .padding(10.0 * uiscale);


        center_x(container(content)
                // .height(500.0 * uiscale)
                // .max_width(1000.0 * uiscale)
                .padding(10.0 * uiscale)
                .style(container::rounded_box))
        .into()
    }

    fn main_buttons(&self, uiscale: f32) -> Element<'_, DialogMessage> {
        let button_save = button(text("Save").size(16.0 * uiscale).center())
            .on_press(DialogMessage::Save)
            .style(button::success);
        let button_cancel = button(text("Cancel").size(16.0 * uiscale).center())
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
            let btn = button(text(label).size(16.0 * uiscale).center());
            if is_active {
                btn.style(button::primary).on_press(msg).into()
            } else {
                btn.style(style).on_press(msg).into()
            }
        }))
        .spacing(10.0 * uiscale);

        row![
            buttons,
            space().width(Fill),
            button_save,
            button_cancel
        ]
        .spacing(10.0 * uiscale)
        .into()
    }

    fn view_process_page(&self, config: &Config, uiscale: f32) -> Element<'_, DialogMessage> {

        let title = text("HDR Process Settings").size(16.0 * uiscale);

        // Threads
        let threads_row = Row::new()
            .push(text("Threads:").size(16.0 * uiscale))
            .push(
                text_input("Threads", &self.threads_str)
                    .on_input(DialogMessage::ThreadsChanged)
                    .width(Length::Fixed(60.0 * uiscale)),
            )
            .spacing(10.0 * uiscale);


        // Checkboxes

        // Check if external executables are configured
        let align_image_stack_configured = !config.exe_paths.align_image_stack_exe.is_empty();
        let luminance_cli_configured = !config.exe_paths.luminance_cli_exe.is_empty();
        let blender_configured = !config.exe_paths.blender_exe.is_empty();

        // Align Images master toggle
        let toggle_align = toggler(config.gui_settings.do_align)
            .label("Enable Image Alignment")
            .on_toggle(DialogMessage::ToggleAlign)
            .size(uiscale * 16.0);

        // Align method toggles (mutually exclusive)
        // If align_image_stack is not configured, both toggles are disabled but show current state
        let toggle_align_image_stack = toggler(config.gui_settings.use_align_image_stack)
            .label(format!("Use align_image_stack{}", if !align_image_stack_configured { " (not configured)" } else { "" }))
            .size(uiscale * 16.0);
        let toggle_align_image_stack = if align_image_stack_configured {
            toggle_align_image_stack.on_toggle(DialogMessage::ToggleAlignImageStack)
        } else {
            toggle_align_image_stack // Disabled but keeps showing current state
        };

        // OpenCV Align toggle - disabled if align_image_stack exe not configured
        let toggle_align_opencv = toggler(config.gui_settings.use_opencv_align)
            .label("Use OpenCV Align (AlignMTB)")
            .size(uiscale * 16.0);
        let toggle_align_opencv = if align_image_stack_configured {
            toggle_align_opencv.on_toggle(DialogMessage::ToggleOpenCvAlign)
        } else {
            toggle_align_opencv // Disabled but keeps showing current state
        };

        // Blender toggle - disabled if not configured
        let toggle_blender_merge = toggler(config.gui_settings.use_blender_merge)
            .label(format!("Use Blender Merge (Zaal){}", if !blender_configured { " (not configured)" } else { "" }))
            .size(uiscale * 16.0);
        let toggle_blender_merge = if blender_configured {
            toggle_blender_merge.on_toggle(DialogMessage::ToggleBlenderMerge)
        } else {
            toggle_blender_merge // Disabled
        };

        let toggle_opencv_debevec = toggler(config.gui_settings.use_opencv_debevec)
                .label("Use OpenCV Merge (Debevec)")
                .on_toggle(DialogMessage::ToggleOpenCvDebevec)
                .size(uiscale * 16.0);
        let toggle_robertson = toggler(config.gui_settings.use_opencv_merge_robertson)
                .label("Use OpenCV Merge (Robertson)")
                .on_toggle(DialogMessage::ToggleOpenCvMergeRobertson)
                .size(uiscale * 16.0);
        let toggle_rust_merge = toggler(config.gui_settings.use_rust_merge)
                .label("Use Rust Merge (Zaal)")
                .on_toggle(DialogMessage::ToggleRustMerge)
                .size(uiscale * 16.0);
        let toggle_rust_merge_debug = toggler(config.gui_settings.rust_merge_debug_export)
                .label("Export Debug EXR Files (Rust Merge)")
                .on_toggle(DialogMessage::ToggleRustMergeDebug)
                .size(uiscale * 16.0);

        // Tone mapping method toggles (mutually exclusive)
        // If Luminance CLI is not configured, both toggles are disabled but show current state
        let toggle_luminance_tonemap = toggler(config.gui_settings.use_luminance_tonemap)
            .label(format!("Use Luminance CLI{}", if !luminance_cli_configured { " (not configured)" } else { "" }))
            .size(uiscale * 16.0);
        let toggle_luminance_tonemap = if luminance_cli_configured {
            toggle_luminance_tonemap.on_toggle(DialogMessage::ToggleLuminanceTonemap)
        } else {
            toggle_luminance_tonemap // Disabled but keeps showing current state
        };

        // OpenCV Tone Map toggle - disabled if Luminance CLI exe not configured
        let toggle_opencv_tonemap = toggler(config.gui_settings.use_opencv_tonemap)
            .label("Use OpenCV Tone Mapping")
            .size(uiscale * 16.0);
        let toggle_opencv_tonemap = if luminance_cli_configured {
            toggle_opencv_tonemap.on_toggle(DialogMessage::ToggleOpenCvTonemap)
        } else {
            toggle_opencv_tonemap // Disabled but keeps showing current state
        };

        let process_column = column![
            title,
            horizontal_rule((2.0 * uiscale) as u16),
            threads_row,
            horizontal_rule((1.0 * uiscale) as u16),
            text("Alignment:").size(14.0 * uiscale),
            toggle_align,
            row![
                space().width(20.0 * uiscale),
                column![
                    toggle_align_image_stack,
                    toggle_align_opencv,
                ].spacing(5.0 * uiscale)
            ].spacing(10.0 * uiscale),
            horizontal_rule((1.0 * uiscale) as u16),
            text("HDR Merge Method:").size(14.0 * uiscale),
            toggle_blender_merge,
            toggle_opencv_debevec,
            toggle_robertson,
            toggle_rust_merge,
            toggle_rust_merge_debug,
            horizontal_rule((1.0 * uiscale) as u16),
            text("Tone Mapping:").size(14.0 * uiscale),
            row![
                space().width(20.0 * uiscale),
                column![
                    toggle_luminance_tonemap,
                    toggle_opencv_tonemap,
                ].spacing(5.0 * uiscale)
            ].spacing(10.0 * uiscale),
        ]
            .spacing(10.0 * uiscale)
            .padding(10.0 * uiscale);


        // Tone Mapping Operator
        let current_choice = ToneMapChoice::from_operator(&self.tonemap_operator);
        
        let tonemap_column = column![
            text("Tone Mapping Operator:").size(14.0 * uiscale),
            radio("Reinhard", ToneMapChoice::Reinhard, Some(current_choice), |_| DialogMessage::TonemapOperatorSelected("Reinhard".to_string())).size(14.0 * uiscale),
            radio("Drago", ToneMapChoice::Drago, Some(current_choice), |_| DialogMessage::TonemapOperatorSelected("Drago".to_string())).size(14.0 * uiscale),
            radio("Durand", ToneMapChoice::Durand, Some(current_choice), |_| DialogMessage::TonemapOperatorSelected("Durand".to_string())).size(14.0 * uiscale),
            radio("Mantiuk", ToneMapChoice::Mantiuk, Some(current_choice), |_| DialogMessage::TonemapOperatorSelected("Mantiuk".to_string())).size(14.0 * uiscale),
        ].spacing(10.0 * uiscale);

        // Intensity
        let intensity_row = Row::new()
            .push(text("Intensity:").size(16.0 * uiscale).center())
            .push(
                text_input("Intensity", &self.tonemap_intensity_str)
                    .on_input(DialogMessage::TonemapIntensityChanged)
                    .width(Length::Fixed(60.0 * uiscale)),
            )
            .spacing(10.0 * uiscale);

        // Contrast
        let contrast_row = Row::new()
            .push(text("Contrast:").size(16.0 * uiscale))
            .push(
                text_input("Contrast", &self.tonemap_contrast_str)
                    .on_input(DialogMessage::TonemapContrastChanged)
                    .width(Length::Fixed(60.0 * uiscale)),
            )
            .spacing(10.0 * uiscale);

        // Saturation
        let saturation_row = Row::new()
            .push(text("Saturation:").size(16.0 * uiscale))
            .push(
                text_input("Saturation", &self.tonemap_saturation_str)
                    .on_input(DialogMessage::TonemapSaturationChanged)
                    .width(Length::Fixed(60.0 * uiscale)),
            )
            .spacing(10.0 * uiscale);

        let tonemap_params_column = column![
            text("Tonemaping parametres:").center().size(14.0 * uiscale),
            intensity_row,
            contrast_row,
            saturation_row,
            tonemap_column
            
        ].spacing(10.0 * uiscale);


        row![
            process_column,
            rule::vertical(2),
            tonemap_params_column,
        ].spacing(10.0 * uiscale).padding(10.0 * uiscale).into()
    }

    fn view_gui_page(&self, config: &Config, uiscale: f32) -> Element<'_, DialogMessage> {
        let mut group = Column::new().spacing(10.0 * uiscale).padding(10.0 * uiscale);

        let title = text("GUI Settings").size(16.0 * uiscale);
        group = group.push(title.center());
        group = group.push(horizontal_rule((uiscale * 2.0) as u16));

        // Theme selector
        let theme_label = text("Theme:").size(16.0 * uiscale);

        // Find current theme by name
        let current_theme = Theme::ALL.iter()
            .find(|t| format!("{:?}", t) == config.gui_settings.theme_name);

        let theme_row = Row::new()
            .push(theme_label)
            .push(
                button(text("<").size(16.0 * uiscale).width(30.0 * uiscale))
                    .on_press(DialogMessage::PreviousTheme),
            )
            .push(
                pick_list(
                    Theme::ALL,
                    current_theme,
                    |theme| DialogMessage::ThemeChanged(format!("{:?}", theme)),
                )
                .width(Length::Fixed(250.0 * uiscale))
                .placeholder("System"),
            )
            .push(
                button(text(">").size(16.0 * uiscale).width(30.0 * uiscale))
                    .on_press(DialogMessage::NextTheme),
            )
            .push(
                button(text("X").size(16.0 * uiscale).width(30.0 * uiscale))
                    .on_press(DialogMessage::ClearTheme),
            )
            .spacing(10.0 * uiscale)
            .align_y(Alignment::Center);
        group = group.push(theme_row);

        // UI Scale
        let scale_label = text("UI Scale:").size(16.0 * uiscale);
        // Show live preview of slider value while dragging
        let scale_value = text(format!("{:.0}%", self.uiscale_slider_value * 100.0)).size(16.0 * uiscale);
        let uiscale_value = text(format!("{:.0}%", uiscale * 100.0)).size(16.0 * uiscale);

        let scale_controls = Row::new()
            .push(scale_label)
            .push(
                button(text("-").size(16.0 * uiscale).width(16.0 * uiscale))
                    .on_press(DialogMessage::UIScaleDecreased),
            )
            .push(
                slider(0.5..=3.0, self.uiscale_slider_value, DialogMessage::UIScaleSliderChanged)
                    .step(0.1)
                    .width(Length::Fixed(200.0 * uiscale))
                    // .on_release(DialogMessage::UIScaleSliderReleased(self.uiscale_slider_value)),
            )
            .push(
                button(text("+").size(16.0 * uiscale).width(16.0 * uiscale))
                    .on_press(DialogMessage::UIScaleIncreased),
            )
            .push(scale_value)
            .push(uiscale_value)
            .spacing(10.0 * uiscale)
            .align_y(Alignment::Center);
        group = group.push(scale_controls);

        column![group].into()
    }

    fn view_folder_page(&self, config: &Config, uiscale: f32) -> Element<'_, DialogMessage> {
        let mut group = Column::new().spacing(10.0 * uiscale).padding(10.0 * uiscale);

        group = group.push(
            toggler(config.gui_settings.do_recursive)
                .label("Recursive Processing")
                .on_toggle(DialogMessage::ToggleRecursive)
                .size(uiscale * 16.0),
        );
        group = group.push(
            toggler(config.gui_settings.do_cleanup)
                .label("Cleanup Temporary Files")
                .on_toggle(DialogMessage::ToggleCleanup)
                .size(uiscale * 16.0),
        );

        // Recursive Max Depth
        let max_depth_row = Row::new()
            .push(text("Recursive Max Depth:").size(16.0 * uiscale))
            .push(
                text_input("Depth", &self.recursive_max_depth_str)
                    .on_input(DialogMessage::RecursiveMaxDepthChanged)
                    .width(Length::Fixed(60.0 * uiscale)),
            )
            .spacing(10.0 * uiscale);

        group = group.push(max_depth_row);

        group = group.push(horizontal_rule((uiscale * 2.0) as u16));

        // Recursive Ignore Folders
        group = group.push(text("Recursive Ignore Folders (comma-separated):").size(16.0 * uiscale));
        group = group.push(
            text_input("Folders", &self.ignore_folders_str)
                .on_input(DialogMessage::RecursiveIgnoreFoldersChanged)
                .width(Length::Fixed(300.0 * uiscale)),
        );

        // Processed Extensions
        group = group.push(text("Processed Extensions (comma-separated):").size(16.0 * uiscale));
        group = group.push(
            text_input("Extensions", &self.processed_exts_str)
                .on_input(DialogMessage::ProcessedExtensionsChanged)
                .width(Length::Fixed(300.0 * uiscale)),
        );

        // Raw Extensions
        group = group.push(text("Raw Extensions (comma-separated):").size(16.0 * uiscale));
        group = group.push(
            text_input("Extensions", &self.raw_exts_str)
                .on_input(DialogMessage::RawExtensionsChanged)
                .width(Length::Fixed(300.0 * uiscale)),
        );
        column![group].into()
    }

    fn view_exe_paths(&self, _config: &Config, uiscale: f32) -> Element<'_, DialogMessage> {
        let title = text("Executable Paths").size(16.0 * uiscale);

        // Align Image Stack
        let align_row = Row::new()
            .push(text("Align Image Stack (Optional):").size(16.0 * uiscale).width(Length::Fixed(250.0 * uiscale)))
            .push(
                text_input("Path", &self.align_image_stack_exe)
                    .on_input(DialogMessage::AlignImageStackPathChanged)
                    .width(Length::Fill),
            )
            .spacing(10.0 * uiscale);

        let align_download = Row::new()
            .push(text("Download:").size(12.0 * uiscale).width(Length::Fixed(250.0 * uiscale)))
            .push(text("https://hugin.sourceforge.io/download/").size(12.0 * uiscale))
            .spacing(10.0 * uiscale);

        // Blender
        let blender_row = Row::new()
            .push(text("Blender:").size(16.0 * uiscale).width(Length::Fixed(250.0 * uiscale)))
            .push(
                text_input("Path", &self.blender_exe)
                    .on_input(DialogMessage::BlenderPathChanged)
                    .width(Length::Fill),
            )
            .spacing(10.0 * uiscale);

        // Luminance CLI
        let luminance_row = Row::new()
            .push(text("Luminance CLI (Optional):").size(16.0 * uiscale).width(Length::Fixed(250.0 * uiscale)))
            .push(
                text_input("Path", &self.luminance_cli_exe)
                    .on_input(DialogMessage::LuminancePathChanged)
                    .width(Length::Fill),
            )
            .spacing(10.0 * uiscale);

        // Rawtherapee CLI
        let rawtherapee_row = Row::new()
            .push(text("Rawtherapee CLI (Optional):").size(16.0 * uiscale).width(Length::Fixed(250.0 * uiscale)))
            .push(
                text_input("Path", &self.rawtherapee_cli_exe)
                    .on_input(DialogMessage::RawtherapeePathChanged)
                    .width(Length::Fill),
            )
            .spacing(10.0 * uiscale);
        column![
            title.center(),
            horizontal_rule((uiscale * 2.0) as u16),
            align_row,
            align_download,
            blender_row,
            luminance_row,
            rawtherapee_row,
        ]
        .max_width(1000.0 * uiscale)
        .spacing(10.0 * uiscale)
        .padding(10.0 * uiscale)
        .into()
    }
}

fn horizontal_rule(thickness: u16) -> Element<'static, DialogMessage> {
    rule::horizontal(thickness as u32).into()
}
