#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod dial;
mod language;
mod placement;
mod settings_file;

use std::{
    ops::RangeInclusive,
    time::{Duration, Instant},
};

use chrono::{Local, Timelike, Utc};
use chrono_tz::{TZ_VARIANTS, Tz};
use eframe::egui::{self, Color32, ViewportBuilder, ViewportCommand};
use serde::{Deserialize, Serialize};

use dial::{
    Appearance, ClockFace, ClockTime, Design, FONT_BYTES, HandAppearance, HandStyle,
    NumeralAppearance, NumeralFont,
};
use language::Language;
use placement::{Drag, Placement};
use settings_file::SettingsFile;

const APP_NAME: &str = "Simple Analog Clock for PC";
const DEFAULT_DIAMETER: u16 = 116;
const MIN_DIAMETER: u16 = 116;
const MAX_DIAMETER: u16 = 400;
const MIN_RIM_WIDTH: u8 = 2;
const DEFAULT_RIM_WIDTH: u8 = 4;
const MAX_RIM_WIDTH: u8 = 14;
const MIN_HAND_WIDTH: u8 = 50;
const MAX_HAND_WIDTH: u8 = 200;
const MIN_NUMERAL_SIZE: u8 = 70;
const MAX_NUMERAL_SIZE: u8 = 125;
const UI_FONT_SIZE: f32 = 11.0 * 96.0 / 72.0;
const MENU_FONT_SIZE: f32 = 12.0 * 96.0 / 72.0;
const MENU_SIZE: egui::Vec2 = egui::vec2(150.0, 162.0);

fn options_size(language: Language) -> egui::Vec2 {
    let width = match language {
        Language::Korean => 374.0,
        Language::English => 424.0,
    };
    egui::vec2(width, 564.0)
}

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
enum Opacity {
    Twenty,
    Forty,
    Sixty,
    Eighty,
    Full,
}

impl Opacity {
    const VALUES: [(Self, &str); 5] = [
        (Self::Twenty, "20%"),
        (Self::Forty, "40%"),
        (Self::Sixty, "60%"),
        (Self::Eighty, "80%"),
        (Self::Full, "100%"),
    ];

    fn alpha(self) -> u8 {
        match self {
            Self::Twenty => 51,
            Self::Forty => 102,
            Self::Sixty => 153,
            Self::Eighty => 204,
            Self::Full => 255,
        }
    }
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(default)]
struct ClockOptions {
    time_zone: Option<Tz>,
    design: Design,
    diameter: u16,
    hands: HandStyle,
    snap_to_edges: bool,
    rim_width: Option<u8>,
    numeral_font: NumeralFont,
    hand_width: u8,
    numeral_size: u8,
    numeral_bold: bool,
    numeral_visible: bool,
}

impl Default for ClockOptions {
    fn default() -> Self {
        Self {
            time_zone: None,
            design: Design::default(),
            diameter: DEFAULT_DIAMETER,
            hands: HandStyle::default(),
            snap_to_edges: true,
            rim_width: Some(DEFAULT_RIM_WIDTH),
            numeral_font: NumeralFont::default(),
            hand_width: HandAppearance::default().width_percent,
            numeral_size: NumeralAppearance::default().size_percent,
            numeral_bold: NumeralAppearance::default().bold,
            numeral_visible: NumeralAppearance::default().visible,
        }
    }
}

impl ClockOptions {
    fn normalize(&mut self) {
        self.diameter = self.diameter.clamp(MIN_DIAMETER, MAX_DIAMETER);
        self.hand_width = self.hand_width.clamp(MIN_HAND_WIDTH, MAX_HAND_WIDTH);
        self.numeral_size = self.numeral_size.clamp(MIN_NUMERAL_SIZE, MAX_NUMERAL_SIZE);
        self.rim_width = self
            .rim_width
            .map(|width| width.clamp(MIN_RIM_WIDTH, MAX_RIM_WIDTH));
        if self.design == Design::LegacySteel {
            self.design = Design::Charcoal;
            self.hands = HandStyle::Pointed;
        }
    }

    fn appearance(self) -> Appearance {
        Appearance {
            design: self.design,
            hands: HandAppearance {
                style: self.hands,
                width_percent: self.hand_width,
            },
            numerals: NumeralAppearance {
                font: self.numeral_font,
                size_percent: self.numeral_size,
                bold: self.numeral_bold,
                visible: self.numeral_visible,
            },
            diameter: f32::from(self.diameter),
            rim_width: self.rim_width.map(f32::from),
        }
    }

    fn window_size(self) -> egui::Vec2 {
        egui::Vec2::splat(self.appearance().window_side())
    }

    fn time(&self) -> (ClockTime, Duration) {
        let now = Utc::now();
        let time = match self.time_zone {
            Some(zone) => now.with_timezone(&zone).time(),
            None => now.with_timezone(&Local).time(),
        };
        let next_tick = Duration::from_nanos(u64::from(1_000_000_000 - now.nanosecond()));
        (
            (time.hour(), time.minute(), time.second()).into(),
            next_tick,
        )
    }
}

#[derive(Clone, Copy)]
struct Settings {
    clock: ClockOptions,
    language: Language,
    opacity: Opacity,
    position: Option<egui::Pos2>,
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    always_on_top: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            clock: ClockOptions::default(),
            language: Language::default(),
            opacity: Opacity::Full,
            position: None,
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            always_on_top: true,
        }
    }
}

struct OptionsDialog {
    draft: ClockOptions,
    zone_filter: String,
    focus_on_open: bool,
    position: Option<egui::Pos2>,
    snap_available: bool,
    save_failed: bool,
}

fn option_slider<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    value: &mut T,
    range: RangeInclusive<T>,
    suffix: &str,
) -> egui::Response {
    let before = *value;
    let mut response = ui.add(
        egui::Slider::new(value, range.clone())
            .step_by(1.0)
            .suffix(suffix),
    );
    if !response.enabled() {
        return response;
    }
    let mut steps = 0.0;
    if response.has_focus() {
        ui.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                response.id,
                egui::EventFilter {
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    ..Default::default()
                },
            )
        });
        // egui already handles left/right on the track and arrows in the numeric value.
        if *value == before {
            steps += ui.input(|input| {
                input.num_presses(egui::Key::ArrowUp) as f32
                    - input.num_presses(egui::Key::ArrowDown) as f32
            });
        }
    }
    if response.hovered() {
        steps += ui.input_mut(|input| {
            let mut steps = 0.0;
            input.events.retain(|event| {
                if let egui::Event::MouseWheel { unit, delta, .. } = event {
                    if delta.y != 0.0 {
                        steps += match unit {
                            egui::MouseWheelUnit::Line => delta.y,
                            _ => delta.y.signum(),
                        };
                        return false;
                    }
                }
                true
            });
            steps
        });
    }
    if steps != 0.0 {
        let step = f64::from(steps.abs().round().max(1.0).copysign(steps));
        let next = T::from_f64(
            (value.to_f64() + step).clamp(range.start().to_f64(), range.end().to_f64()),
        );
        if next != *value {
            *value = next;
            response.mark_changed();
            ui.ctx().request_repaint();
        }
    }
    response
}

impl OptionsDialog {
    fn show(
        &mut self,
        ctx: &egui::Context,
        viewport: ViewportBuilder,
        preview: &mut ClockFace,
        language: Language,
    ) -> Option<bool> {
        let mut result = None;
        let title = language.text("옵션", "Options");
        let viewport = viewport
            .with_title(format!("{APP_NAME} — {title}"))
            .with_inner_size(options_size(language))
            .with_resizable(false);
        let viewport = if let Some(position) = self.position {
            viewport.with_position(position)
        } else {
            viewport
        };
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("options"),
            viewport,
            |ctx, class| {
                if class == egui::ViewportClass::Embedded {
                    egui::Window::new(title)
                        .id(egui::Id::new("options"))
                        .show(ctx, |ui| {
                            result = self.contents(ui, preview, language);
                        });
                    return;
                }
                if self.focus_on_open {
                    self.focus_on_open = false;
                    ctx.send_viewport_cmd(ViewportCommand::Focus);
                }
                egui::CentralPanel::default()
                    .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(16.0))
                    .show(ctx, |ui| result = self.contents(ui, preview, language));
                if ctx.input(|input| {
                    input.viewport().close_requested() || input.key_pressed(egui::Key::Escape)
                }) {
                    result = Some(false);
                }
            },
        );
        result
    }

    fn contents(
        &mut self,
        ui: &mut egui::Ui,
        preview: &mut ClockFace,
        language: Language,
    ) -> Option<bool> {
        let enter_pressed =
            ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        ui.spacing_mut().item_spacing.y = 10.0;
        ui.vertical_centered(|ui| {
            let (time, next_tick) = self.draft.time();
            ui.ctx().request_repaint_after(next_tick);
            let (area, _) = ui.allocate_exact_size(egui::Vec2::splat(160.0), egui::Sense::hover());
            let mut appearance = self.draft.appearance();
            let scale = 130.0 / appearance.diameter;
            appearance.diameter *= scale;
            appearance.rim_width = appearance.rim_width.map(|width| width * scale);
            let rect = egui::Rect::from_center_size(
                area.center(),
                egui::Vec2::splat(appearance.window_side()),
            );
            preview.paint(ui.painter(), rect, appearance, time, 255);
        });
        ui.separator();

        egui::Grid::new("options_grid")
            .num_columns(2)
            .spacing(egui::vec2(12.0, 13.0))
            .show(ui, |ui| {
                ui.label(language.text("시계 크기", "Clock Size"));
                option_slider(
                    ui,
                    &mut self.draft.diameter,
                    MIN_DIAMETER..=MAX_DIAMETER,
                    " px",
                );
                ui.end_row();

                ui.label(language.text("디자인", "Design"));
                egui::ComboBox::from_id_salt("clock_design")
                    .width(230.0)
                    .selected_text(self.draft.design.label(language))
                    .show_ui(ui, |ui| {
                        for design in Design::ALL {
                            ui.selectable_value(
                                &mut self.draft.design,
                                design,
                                design.label(language),
                            );
                        }
                    });
                ui.end_row();

                ui.label(language.text("바늘 두께", "Hand Thickness"));
                option_slider(
                    ui,
                    &mut self.draft.hand_width,
                    MIN_HAND_WIDTH..=MAX_HAND_WIDTH,
                    "%",
                );
                ui.end_row();

                ui.label(language.text("숫자 표시", "Show Numerals"));
                ui.checkbox(
                    &mut self.draft.numeral_visible,
                    language.text("표시", "Show"),
                );
                ui.end_row();

                ui.label(language.text("숫자 글꼴", "Numeral Font"));
                ui.add_enabled_ui(self.draft.numeral_visible, |ui| {
                    egui::ComboBox::from_id_salt("numeral_font")
                        .width(230.0)
                        .selected_text(self.draft.numeral_font.label())
                        .show_ui(ui, |ui| {
                            for font in NumeralFont::ALL {
                                ui.selectable_value(
                                    &mut self.draft.numeral_font,
                                    font,
                                    font.label(),
                                );
                            }
                        });
                });
                ui.end_row();

                ui.label(language.text("글꼴 크기", "Font Size"));
                ui.add_enabled_ui(self.draft.numeral_visible, |ui| {
                    ui.horizontal(|ui| {
                        option_slider(
                            ui,
                            &mut self.draft.numeral_size,
                            MIN_NUMERAL_SIZE..=MAX_NUMERAL_SIZE,
                            "%",
                        );
                        ui.toggle_value(
                            &mut self.draft.numeral_bold,
                            language.text("굵게", "Bold"),
                        );
                    });
                });
                ui.end_row();

                ui.label(language.text("바늘 모양", "Hand Shape"));
                ui.horizontal(|ui| {
                    for style in HandStyle::ALL {
                        ui.selectable_value(&mut self.draft.hands, style, style.label(language));
                    }
                });
                ui.end_row();

                ui.label(language.text("테두리 두께", "Border Thickness"));
                ui.horizontal(|ui| {
                    let automatic_width =
                        dial::automatic_rim_width(f32::from(self.draft.diameter)).round() as u8;
                    let mut width = self.draft.rim_width.unwrap_or(automatic_width);
                    if option_slider(ui, &mut width, MIN_RIM_WIDTH..=MAX_RIM_WIDTH, " px").changed()
                    {
                        self.draft.rim_width = Some(width);
                    }
                    let automatic = self.draft.rim_width.is_none();
                    if ui
                        .selectable_label(automatic, language.text("자동", "Auto"))
                        .clicked()
                    {
                        self.draft.rim_width = automatic.then_some(width);
                    }
                });
                ui.end_row();

                ui.label(language.text("시간 기준", "Time Source"));
                let local_time = language.text("현재 컴퓨터 시간", "Current computer time");
                let zone_label = self.draft.time_zone.map_or(local_time, |zone| zone.name());
                egui::ComboBox::from_id_salt("time_zone")
                    .width(230.0)
                    .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                    .selected_text(zone_label)
                    .show_ui(ui, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.zone_filter)
                                .hint_text(language.text("시간대 검색", "Search time zones")),
                        );
                        if ui
                            .selectable_label(self.draft.time_zone.is_none(), local_time)
                            .clicked()
                        {
                            self.draft.time_zone = None;
                            self.zone_filter.clear();
                            ui.close();
                        }
                        let filter = self.zone_filter.trim().to_ascii_lowercase();
                        egui::ScrollArea::vertical()
                            .max_height(116.0)
                            .show(ui, |ui| {
                                for zone in TZ_VARIANTS.iter().copied() {
                                    if !zone.name().to_ascii_lowercase().contains(&filter) {
                                        continue;
                                    }
                                    if ui
                                        .selectable_label(
                                            self.draft.time_zone == Some(zone),
                                            zone.name(),
                                        )
                                        .clicked()
                                    {
                                        self.draft.time_zone = Some(zone);
                                        self.zone_filter.clear();
                                        ui.close();
                                    }
                                }
                            });
                    });
                ui.end_row();

                if self.snap_available {
                    ui.label(language.text("이동", "Movement"));
                    ui.checkbox(
                        &mut self.draft.snap_to_edges,
                        language.text("가장자리 붙이기", "Snap to Edges"),
                    );
                    ui.end_row();
                }
            });
        if self.save_failed {
            ui.colored_label(
                ui.visuals().error_fg_color,
                language.text(
                    "INI 설정 파일을 저장할 수 없습니다.",
                    "Could not save the INI settings file.",
                ),
            );
        }
        let button_height = ui.spacing().interact_size.y.max(
            ui.text_style_height(&egui::TextStyle::Button) + 2.0 * ui.spacing().button_padding.y,
        );
        ui.add_space((ui.available_height() - button_height).max(10.0));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = ui
                .painter()
                .layout_no_wrap(
                    "확확확".to_owned(),
                    egui::TextStyle::Button.resolve(ui.style()),
                    Color32::PLACEHOLDER,
                )
                .size()
                .x;
            let cancel = ui.button(language.text("취소", "Cancel"));
            let confirm = ui.button(language.text("확인(Enter)", "OK (Enter)"));
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                if ui.button(language.text("초기화", "Reset")).clicked() {
                    self.draft = ClockOptions::default();
                    self.zone_filter.clear();
                    ui.ctx().request_repaint();
                }
            });
            if confirm.clicked() || enter_pressed {
                Some(true)
            } else if cancel.clicked() {
                Some(false)
            } else {
                None
            }
        })
        .inner
    }
}

enum MenuAction {
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    ToggleTop,
    SetOpacity(Opacity),
    SetLanguage(Language),
    OpenOptions,
    CloseClock,
    Dismiss,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum Submenu {
    Language,
    Opacity,
}

impl Submenu {
    fn label(self, language: Language) -> &'static str {
        match self {
            Self::Language => language.text("언어", "Language"),
            Self::Opacity => language.text("불투명도", "Opacity"),
        }
    }

    fn size(self) -> egui::Vec2 {
        match self {
            Self::Language => egui::vec2(120.0, 66.0),
            Self::Opacity => egui::vec2(100.0, 146.0),
        }
    }

    fn contents(self, ui: &mut egui::Ui, settings: &Settings) -> Option<MenuAction> {
        menu_font(ui);
        match self {
            Self::Language => menu_choices(
                ui,
                settings.language,
                Language::ALL.map(|language| (language, language.label())),
            )
            .map(MenuAction::SetLanguage),
            Self::Opacity => {
                menu_choices(ui, settings.opacity, Opacity::VALUES).map(MenuAction::SetOpacity)
            }
        }
    }
}

struct ClockMenu {
    placement: Placement,
    pointer: egui::Pos2,
    focus_lost_at: Option<Instant>,
    was_focused: bool,
    submenu: Option<(Submenu, f32)>,
}

impl ClockMenu {
    fn show(&mut self, ctx: &egui::Context, settings: &Settings) -> Option<MenuAction> {
        let position = self.placement.menu_position(self.pointer, MENU_SIZE);
        let parent = ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("clock_menu"),
            menu_window(
                settings.language.text("시계 메뉴", "Clock Menu"),
                MENU_SIZE,
                position,
            ),
            |ctx, _| {
                let mut rows = Vec::with_capacity(2);
                let action = menu_panel(ctx, |ui| {
                    clock_menu(ui, settings, |ui, submenu| {
                        let row = ui.add(
                            egui::Button::new(submenu.label(settings.language)).right_text("⏵"),
                        );
                        rows.push((submenu, row));
                        None
                    })
                });
                (menu_state(ctx, action), rows, ctx.pointer_hover_pos())
            },
        );
        let ((mut focused, action), rows, pointer) = parent;
        if action.is_some() {
            return action;
        }
        self.update_submenu_hover(&rows, pointer);
        if let Some((submenu, top)) = self.submenu {
            let size = submenu.size();
            let position = self
                .placement
                .submenu_position(position, MENU_SIZE, top, size);
            let (child_focused, action) = ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of(("clock_submenu", submenu)),
                menu_window(submenu.label(settings.language), size, position).with_active(false),
                |ctx, _| menu_state(ctx, menu_panel(ctx, |ui| submenu.contents(ui, settings))),
            );
            if action.is_some() {
                return action;
            }
            focused |= child_focused;
        }
        if focused {
            self.was_focused = true;
            self.focus_lost_at = None;
        } else if self.was_focused {
            let elapsed = self
                .focus_lost_at
                .get_or_insert_with(Instant::now)
                .elapsed();
            let grace = Duration::from_millis(100);
            if elapsed >= grace {
                return Some(MenuAction::Dismiss);
            }
            ctx.request_repaint_after(grace - elapsed);
        }
        None
    }

    fn update_submenu_hover(
        &mut self,
        rows: &[(Submenu, egui::Response)],
        pointer: Option<egui::Pos2>,
    ) {
        if let Some((submenu, row)) = rows.iter().find(|(_, row)| row.hovered() || row.clicked()) {
            self.submenu = Some((*submenu, row.rect.top()));
        } else if let (Some((active, _)), Some(pointer)) = (self.submenu, pointer) {
            let near_row = rows.iter().any(|(submenu, row)| {
                *submenu == active
                    && (row.rect.top() - 4.0..=row.rect.bottom() + 4.0).contains(&pointer.y)
            });
            if !near_row {
                self.submenu = None;
            }
        }
    }
}

fn menu_window(title: &str, size: egui::Vec2, position: egui::Pos2) -> ViewportBuilder {
    let viewport = ViewportBuilder::default()
        .with_title(title)
        .with_inner_size(size)
        .with_position(position)
        .with_decorations(false)
        .with_resizable(false)
        .with_taskbar(false);
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    let viewport = viewport.with_always_on_top();
    viewport
}

fn menu_panel(
    ctx: &egui::Context,
    content: impl FnOnce(&mut egui::Ui) -> Option<MenuAction>,
) -> Option<MenuAction> {
    egui::CentralPanel::default()
        .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(8.0))
        .show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), content)
                .inner
        })
        .inner
}

fn menu_state(ctx: &egui::Context, action: Option<MenuAction>) -> (bool, Option<MenuAction>) {
    ctx.input(|input| {
        let viewport = input.viewport();
        let dismiss = viewport.close_requested() || input.key_pressed(egui::Key::Escape);
        (
            viewport.focused == Some(true),
            if dismiss {
                Some(MenuAction::Dismiss)
            } else {
                action
            },
        )
    })
}

fn menu_font(ui: &mut egui::Ui) {
    ui.spacing_mut().item_spacing.y = 7.0;
    for name in [egui::TextStyle::Body, egui::TextStyle::Button] {
        if let Some(font) = ui.style_mut().text_styles.get_mut(&name) {
            font.size = MENU_FONT_SIZE;
        }
    }
}

fn clock_menu(
    ui: &mut egui::Ui,
    settings: &Settings,
    mut show_submenu: impl FnMut(&mut egui::Ui, Submenu) -> Option<MenuAction>,
) -> Option<MenuAction> {
    menu_font(ui);
    let language = settings.language;
    if let Some(action) = show_submenu(ui, Submenu::Language) {
        return Some(action);
    }
    #[cfg(any(target_os = "windows", target_os = "macos"))]
    {
        let mut always_on_top = settings.always_on_top;
        if ui
            .checkbox(
                &mut always_on_top,
                language.text("항상 위", "Always on Top"),
            )
            .changed()
        {
            return Some(MenuAction::ToggleTop);
        }
    }
    if let Some(action) = show_submenu(ui, Submenu::Opacity) {
        return Some(action);
    }
    if ui.button(language.text("옵션", "Options")).clicked() {
        return Some(MenuAction::OpenOptions);
    }
    ui.separator();
    ui.button(language.text("닫기", "Close"))
        .clicked()
        .then_some(MenuAction::CloseClock)
}

fn menu_choices<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    current: T,
    choices: impl IntoIterator<Item = (T, &'static str)>,
) -> Option<T> {
    let mut action = None;
    for (value, label) in choices {
        let mut selected = current == value;
        if ui.checkbox(&mut selected, label).changed() {
            action = Some(value);
        }
    }
    action
}

struct ClockApp {
    settings: Settings,
    settings_file: Option<SettingsFile>,
    save_failed: bool,
    face: ClockFace,
    preview: ClockFace,
    dialog: Option<OptionsDialog>,
    menu: Option<ClockMenu>,
    drag: Option<Drag>,
}

impl ClockApp {
    fn new(
        cc: &eframe::CreationContext<'_>,
        settings: Settings,
        settings_file: Option<SettingsFile>,
    ) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "korean".into(),
            egui::FontData::from_static(FONT_BYTES).into(),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push("korean".into());
        cc.egui_ctx.set_fonts(fonts);
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            cc.egui_ctx.style_mut_of(theme, |style| {
                for name in [
                    egui::TextStyle::Body,
                    egui::TextStyle::Button,
                    egui::TextStyle::Small,
                    egui::TextStyle::Monospace,
                ] {
                    if let Some(font) = style.text_styles.get_mut(&name) {
                        font.size = UI_FONT_SIZE;
                    }
                }
            });
        }
        #[cfg(any(target_os = "windows", target_os = "macos"))]
        set_window_level(&cc.egui_ctx, settings.always_on_top);

        Self {
            settings,
            settings_file,
            save_failed: false,
            face: ClockFace::new(&cc.egui_ctx, "dial"),
            preview: ClockFace::new(&cc.egui_ctx, "preview"),
            dialog: None,
            menu: None,
            drag: None,
        }
    }

    fn persist(&mut self, explicit: bool) {
        self.save_failed = self
            .settings_file
            .as_mut()
            .is_none_or(|file| file.save(&self.settings, explicit).is_err());
    }

    fn handle_menu(
        &mut self,
        ctx: &egui::Context,
        action: MenuAction,
        placement: Option<Placement>,
    ) {
        match action {
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            MenuAction::ToggleTop => {
                self.settings.always_on_top = !self.settings.always_on_top;
                set_window_level(ctx, self.settings.always_on_top);
                self.persist(true);
            }
            MenuAction::SetOpacity(value) => {
                self.settings.opacity = value;
                self.persist(true);
            }
            MenuAction::SetLanguage(language) => {
                self.settings.language = language;
                if let (Some(dialog), Some(placement)) = (&mut self.dialog, placement) {
                    let size = options_size(language);
                    dialog.position =
                        Some(placement.options_position(size + egui::vec2(16.0, 44.0)));
                }
                self.persist(true);
            }
            MenuAction::OpenOptions => {
                let size = options_size(self.settings.language);
                let position = placement.map(|p| p.options_position(size + egui::vec2(16.0, 44.0)));
                let dialog = self.dialog.get_or_insert(OptionsDialog {
                    draft: self.settings.clock,
                    zone_filter: String::new(),
                    focus_on_open: true,
                    position,
                    snap_available: placement.is_some_and(Placement::can_snap),
                    save_failed: self.save_failed,
                });
                dialog.position = position;
                dialog.focus_on_open = true;
            }
            MenuAction::CloseClock => ctx.send_viewport_cmd(ViewportCommand::Close),
            MenuAction::Dismiss => {}
        }
        self.menu = None;
        ctx.request_repaint();
    }
}

impl eframe::App for ClockApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let (time, next_tick) = self.settings.clock.time();
        ctx.request_repaint_after(next_tick);
        #[cfg(windows)]
        if self.drag.is_none() {
            placement::keep_clock_on_screen(frame);
        }
        let placement = Placement::current(ctx, frame);
        if let Some(placement) = placement {
            self.settings.position = Some(placement.clock.min);
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                let (rect, response) =
                    ui.allocate_exact_size(ui.available_size(), egui::Sense::click_and_drag());
                self.face.paint(
                    ui.painter(),
                    rect,
                    self.settings.clock.appearance(),
                    time,
                    self.settings.opacity.alpha(),
                );
                if response.drag_started_by(egui::PointerButton::Primary) {
                    self.menu = None;
                    self.drag = Some(Drag::begin(ctx, frame, placement));
                }
                if let Some(placement) = placement.filter(|_| !ctx.embed_viewports()) {
                    if response.secondary_clicked() {
                        if let Some(pointer) = response.interact_pointer_pos() {
                            let origin = ctx.input(|input| {
                                input.viewport().inner_rect.unwrap_or(placement.clock).min
                            });
                            self.menu = Some(ClockMenu {
                                placement,
                                pointer: origin + pointer.to_vec2(),
                                focus_lost_at: None,
                                was_focused: false,
                                submenu: None,
                            });
                        }
                    } else if response.clicked() {
                        self.menu = None;
                    }
                } else {
                    response.context_menu(|ui| {
                        if let Some(action) = clock_menu(ui, &self.settings, |ui, submenu| {
                            ui.menu_button(submenu.label(self.settings.language), |ui| {
                                submenu.contents(ui, &self.settings)
                            })
                            .inner
                            .flatten()
                        }) {
                            ui.close();
                            self.handle_menu(ctx, action, placement);
                        }
                    });
                }
            });

        if self
            .drag
            .as_ref()
            .is_some_and(|drag| !drag.update(ctx, frame, self.settings.clock.snap_to_edges))
        {
            self.drag = None;
        }

        if let Some(menu) = &mut self.menu {
            if let Some(action) = menu.show(ctx, &self.settings) {
                self.handle_menu(ctx, action, placement);
            }
        }

        if let Some(dialog) = &mut self.dialog {
            let viewport = ViewportBuilder::default();
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            let viewport = viewport.with_window_level(window_level(self.settings.always_on_top));
            if let Some(apply) =
                dialog.show(ctx, viewport, &mut self.preview, self.settings.language)
            {
                if apply {
                    let previous = self.settings;
                    let draft = dialog.draft;
                    let size = draft.window_size();
                    let was_resized = previous.clock.window_size() != size;
                    let position = placement
                        .filter(|_| was_resized && previous.clock.diameter == draft.diameter)
                        .map(|placement| placement.resized_clock_position(size));
                    self.settings.clock = draft;
                    self.settings.position = position.or(previous.position);
                    self.persist(true);
                    if self.save_failed {
                        self.settings = previous;
                        if let Some(dialog) = &mut self.dialog {
                            dialog.save_failed = true;
                        }
                        return;
                    }
                    if was_resized {
                        ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
                    }
                    if let Some(position) = position {
                        ctx.send_viewport_cmd(ViewportCommand::OuterPosition(position));
                    }
                    ctx.request_repaint();
                }
                self.dialog = None;
            }
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Color32::TRANSPARENT.to_normalized_gamma_f32()
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.persist(false);
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn set_window_level(ctx: &egui::Context, always_on_top: bool) {
    ctx.send_viewport_cmd_to(
        egui::ViewportId::ROOT,
        ViewportCommand::WindowLevel(window_level(always_on_top)),
    );
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn window_level(always_on_top: bool) -> egui::WindowLevel {
    if always_on_top {
        egui::WindowLevel::AlwaysOnTop
    } else {
        egui::WindowLevel::Normal
    }
}

fn main() -> eframe::Result {
    let settings_file = SettingsFile::beside_executable().ok();
    let settings = settings_file
        .as_ref()
        .and_then(|file| file.load().ok())
        .unwrap_or_default();
    let mut viewport = ViewportBuilder::default()
        .with_app_id("simple-analog-clock-for-pc")
        .with_inner_size(settings.clock.window_size())
        .with_resizable(false)
        .with_decorations(false)
        .with_transparent(true)
        .with_icon(dial::icon());
    #[cfg(windows)]
    {
        viewport = viewport.with_taskbar(false);
    }
    if let Some(position) = settings.position {
        viewport = viewport.with_position(position);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        APP_NAME,
        options,
        Box::new(move |cc| Ok(Box::new(ClockApp::new(cc, settings, settings_file)))),
    )
}
