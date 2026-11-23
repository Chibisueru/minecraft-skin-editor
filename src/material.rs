//! A small Material 3 design layer for iced: generated colour schemes,
//! the type scale, and style functions for the components the editor uses.
//!
//! Every style function takes the active [`Scheme`] by value, so the same
//! widgets render correctly in light and dark mode.

use iced::border::Radius;
use iced::widget::{button, checkbox, container, rule, scrollable, text, text_input, toggler, Text};
use iced::{font, Background, Border, Color, Font, Shadow, Theme, Vector};

// ---------------------------------------------------------------- colours ---

/// The Material 3 colour roles the editor uses.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)] // a complete set of roles, not all of which are used yet
pub struct Scheme {
    pub primary: Color,
    pub on_primary: Color,
    pub primary_container: Color,
    pub on_primary_container: Color,
    pub secondary_container: Color,
    pub on_secondary_container: Color,
    pub error: Color,
    pub surface: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub outline_variant: Color,
    pub surface_container_lowest: Color,
    pub surface_container_low: Color,
    pub surface_container: Color,
    pub surface_container_high: Color,
    pub surface_container_highest: Color,
    pub inverse_surface: Color,
    pub inverse_on_surface: Color,
    pub inverse_primary: Color,
    pub shadow: Color,
}

/// The colour the whole interface is generated from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accent {
    Blue,
    Green,
    Red,
    Purple,
    Yellow,
}

impl Accent {
    pub const ALL: [Accent; 5] = [Accent::Blue, Accent::Green, Accent::Red, Accent::Purple, Accent::Yellow];

    pub fn name(self) -> &'static str {
        match self {
            Accent::Blue => "Blue",
            Accent::Green => "Green",
            Accent::Red => "Red",
            Accent::Purple => "Purple",
            Accent::Yellow => "Yellow",
        }
    }

    /// The source colour handed to the Material colour algorithm.
    fn seed(self) -> u32 {
        match self {
            Accent::Blue => 0x1A73E8,
            Accent::Green => 0x2E9D4A,
            Accent::Red => 0xD93025,
            Accent::Purple => 0x6750A4,
            Accent::Yellow => 0xF9C80E,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.name().eq_ignore_ascii_case(name))
    }
}

fn argb(c: material_colors::color::Argb) -> Color {
    Color::from_rgb8(c.red, c.green, c.blue)
}

impl Scheme {
    /// The Material 3 tonal scheme for an accent, as Google's colour
    /// utilities compute it.
    pub fn new(accent: Accent, dark: bool) -> Self {
        let theme = material_colors::theme::ThemeBuilder::with_source(
            material_colors::color::Argb::from_u32(0xFF00_0000 | accent.seed()),
        )
        .build();
        let m = if dark { &theme.schemes.dark } else { &theme.schemes.light };
        Scheme {
            primary: argb(m.primary),
            on_primary: argb(m.on_primary),
            primary_container: argb(m.primary_container),
            on_primary_container: argb(m.on_primary_container),
            secondary_container: argb(m.secondary_container),
            on_secondary_container: argb(m.on_secondary_container),
            error: argb(m.error),
            surface: argb(m.surface),
            on_surface: argb(m.on_surface),
            on_surface_variant: argb(m.on_surface_variant),
            outline: argb(m.outline),
            outline_variant: argb(m.outline_variant),
            surface_container_lowest: argb(m.surface_container_lowest),
            surface_container_low: argb(m.surface_container_low),
            surface_container: argb(m.surface_container),
            surface_container_high: argb(m.surface_container_high),
            surface_container_highest: argb(m.surface_container_highest),
            inverse_surface: argb(m.inverse_surface),
            inverse_on_surface: argb(m.inverse_on_surface),
            inverse_primary: argb(m.inverse_primary),
            shadow: argb(m.shadow),
        }
    }

    /// An iced theme that matches the scheme, for anything not styled here.
    pub fn theme(&self) -> Theme {
        Theme::custom(
            "Material 3".to_string(),
            iced::theme::Palette {
                background: self.surface,
                text: self.on_surface,
                primary: self.primary,
                success: self.primary,
                danger: self.error,
            },
        )
    }
}

pub fn alpha(color: Color, a: f32) -> Color {
    Color { a: color.a * a, ..color }
}

/// `over` laid on top of `base` at the given opacity: a Material state layer.
fn layer(base: Color, over: Color, opacity: f32) -> Color {
    let mix = |b: f32, o: f32| b + (o - b) * opacity;
    Color {
        r: mix(base.r, over.r),
        g: mix(base.g, over.g),
        b: mix(base.b, over.b),
        a: base.a,
    }
}

/// State-layer opacity for a button-like status.
fn state_opacity(status: button::Status) -> f32 {
    match status {
        button::Status::Hovered => 0.08,
        button::Status::Pressed => 0.10,
        _ => 0.0,
    }
}

fn elevation(s: &Scheme, level: u8) -> Shadow {
    let (y, blur, a) = match level {
        0 => return Shadow::default(),
        1 => (1.0, 3.0, 0.20),
        2 => (2.0, 6.0, 0.22),
        _ => (4.0, 12.0, 0.26),
    };
    Shadow {
        color: alpha(s.shadow, a),
        offset: Vector::new(0.0, y),
        blur_radius: blur,
    }
}

// ------------------------------------------------------------- typography ---

pub const MEDIUM: Font = Font {
    weight: font::Weight::Medium,
    ..Font::DEFAULT
};

pub fn headline_small<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).size(24)
}

pub fn title_large<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).size(22)
}

pub fn title_small<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).size(14).font(MEDIUM)
}

pub fn label_large<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).size(14).font(MEDIUM)
}

pub fn label_medium<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).size(12).font(MEDIUM)
}

pub fn body_medium<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).size(14)
}

pub fn body_small<'a>(content: impl text::IntoFragment<'a>) -> Text<'a> {
    text(content).size(12)
}

// ------------------------------------------------------------------ icons ---

/// Material Symbols Rounded, outlined (FILL 0) and filled (FILL 1), cut down
/// to the glyphs below. Apache-2.0, see `assets/fonts/LICENSE-MaterialSymbols`.
pub const ICONS: &[u8] = include_bytes!("../assets/fonts/MaterialSymbolsRounded.ttf");
pub const ICONS_FILLED: &[u8] = include_bytes!("../assets/fonts/MaterialSymbolsRoundedFilled.ttf");

const ICON_FONT: Font = Font::with_name("Material Symbols Rounded");
const ICON_FONT_FILLED: Font = Font::with_name("Material Symbols Rounded Filled");

#[derive(Clone, Copy, Debug)]
pub enum Icon {
    Add,
    FolderOpen,
    Save,
    SaveAs,
    Close,
    Edit,
    Eraser,
    Fill,
    Colorize,
    Undo,
    Redo,
    Check,
    LayersClear,
    RestartAlt,
    Brush,
    Warning,
    Settings,
    LightMode,
    DarkMode,
}

impl Icon {
    fn codepoint(self) -> char {
        match self {
            Icon::Add => '\u{e145}',
            Icon::FolderOpen => '\u{e2c8}',
            Icon::Save => '\u{e161}',
            Icon::SaveAs => '\u{eb60}',
            Icon::Close => '\u{e5cd}',
            Icon::Edit => '\u{f097}',
            Icon::Eraser => '\u{e6d0}',
            Icon::Fill => '\u{e23a}',
            Icon::Colorize => '\u{e3b8}',
            Icon::Undo => '\u{e166}',
            Icon::Redo => '\u{e15a}',
            Icon::Check => '\u{e668}',
            Icon::LayersClear => '\u{e53c}',
            Icon::RestartAlt => '\u{f053}',
            Icon::Brush => '\u{e3ae}',
            Icon::Warning => '\u{f083}',
            Icon::Settings => '\u{e8b8}',
            Icon::LightMode => '\u{e518}',
            Icon::DarkMode => '\u{e51c}',
        }
    }
}

fn glyph<'a>(icon: Icon, size: f32, font: Font) -> Text<'a> {
    text(icon.codepoint().to_string())
        .font(font)
        .size(size)
        .line_height(1.0)
        .width(size)
        .height(size)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
}

/// An outlined icon; it takes its colour from the surrounding widget.
pub fn icon<'a>(icon: Icon, size: f32) -> Text<'a> {
    glyph(icon, size, ICON_FONT)
}

/// A filled icon, for selected states.
pub fn icon_filled<'a>(icon: Icon, size: f32) -> Text<'a> {
    glyph(icon, size, ICON_FONT_FILLED)
}

// ---------------------------------------------------------------- buttons ---

pub const FULL: f32 = 20.0;

fn button_style(
    status: button::Status,
    s: &Scheme,
    background: Option<Color>,
    content: Color,
    outline: Option<Color>,
    radius: Radius,
    raise_on_hover: bool,
) -> button::Style {
    if status == button::Status::Disabled {
        return button::Style {
            background: background.map(|_| Background::Color(alpha(s.on_surface, 0.12))),
            text_color: alpha(s.on_surface, 0.38),
            border: Border {
                color: alpha(s.on_surface, 0.12),
                width: if outline.is_some() { 1.0 } else { 0.0 },
                radius,
            },
            shadow: Shadow::default(),
        };
    }
    let opacity = state_opacity(status);
    let fill = match background {
        Some(bg) => Some(layer(bg, content, opacity)),
        None if opacity > 0.0 => Some(alpha(content, opacity)),
        None => None,
    };
    button::Style {
        background: fill.map(Background::Color),
        text_color: content,
        border: Border {
            color: outline.unwrap_or(Color::TRANSPARENT),
            width: if outline.is_some() { 1.0 } else { 0.0 },
            radius,
        },
        shadow: if raise_on_hover && status == button::Status::Hovered {
            elevation(s, 1)
        } else {
            Shadow::default()
        },
    }
}

/// The common button emphasis levels, for helpers that take any of them.
#[derive(Clone, Copy)]
pub enum Emphasis {
    Filled,
    Tonal,
    Outlined,
    Text,
    Danger,
}

pub fn emphasis(s: Scheme, kind: Emphasis) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, st| match kind {
        Emphasis::Filled => filled(s)(theme, st),
        Emphasis::Tonal => tonal(s)(theme, st),
        Emphasis::Outlined => outlined(s)(theme, st),
        Emphasis::Text => text_button(s)(theme, st),
        Emphasis::Danger => text_danger(s)(theme, st),
    }
}

/// Filled button: the most important action on screen.
pub fn filled(s: Scheme) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, st| button_style(st, &s, Some(s.primary), s.on_primary, None, FULL.into(), true)
}

/// Filled tonal button: a secondary action that still needs weight.
pub fn tonal(s: Scheme) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, st| {
        button_style(
            st,
            &s,
            Some(s.secondary_container),
            s.on_secondary_container,
            None,
            FULL.into(),
            true,
        )
    }
}

/// Outlined button: medium emphasis.
pub fn outlined(s: Scheme) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, st| button_style(st, &s, None, s.primary, Some(s.outline_variant), FULL.into(), false)
}

/// Text button: low emphasis.
pub fn text_button(s: Scheme) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, st| button_style(st, &s, None, s.primary, None, FULL.into(), false)
}

/// Text button for a destructive choice.
pub fn text_danger(s: Scheme) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, st| button_style(st, &s, None, s.error, None, FULL.into(), false)
}

/// Standard icon button (round, no container).
pub fn icon_button(s: Scheme) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, st| button_style(st, &s, None, s.on_surface_variant, None, 999.0.into(), false)
}

/// A round colour chip in the options dialog.
pub fn color_chip(s: Scheme, color: Color, selected: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, st| button::Style {
        background: Some(layer(color, s.on_primary, state_opacity(st)).into()),
        text_color: s.on_primary,
        border: Border {
            color: if selected { s.on_surface } else { Color::TRANSPARENT },
            width: if selected { 2.0 } else { 0.0 },
            radius: 999.0.into(),
        },
        shadow: if st == button::Status::Hovered { elevation(&s, 1) } else { Shadow::default() },
    }
}

/// A navigation-drawer item: a pill that fills in when it is the active one.
pub fn nav_item(s: Scheme, active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, st| {
        if active {
            button_style(
                st,
                &s,
                Some(s.secondary_container),
                s.on_secondary_container,
                None,
                999.0.into(),
                false,
            )
        } else {
            button_style(st, &s, None, s.on_surface_variant, None, 999.0.into(), false)
        }
    }
}

/// Where a segment sits in a segmented button, which decides its rounding.
#[derive(Clone, Copy)]
pub enum Segment {
    First,
    Middle,
    Last,
}

/// One segment of a segmented button. The surrounding outline is drawn by
/// [`segmented_frame`], so segments themselves have no border.
pub fn segment(s: Scheme, selected: bool, at: Segment) -> impl Fn(&Theme, button::Status) -> button::Style {
    let r = FULL - 1.0;
    let radius = match at {
        Segment::First => Radius {
            top_left: r,
            bottom_left: r,
            top_right: 0.0,
            bottom_right: 0.0,
        },
        Segment::Middle => Radius::from(0.0),
        Segment::Last => Radius {
            top_left: 0.0,
            bottom_left: 0.0,
            top_right: r,
            bottom_right: r,
        },
    };
    move |_, st| {
        if selected {
            button_style(
                st,
                &s,
                Some(s.secondary_container),
                s.on_secondary_container,
                None,
                radius,
                false,
            )
        } else {
            button_style(st, &s, None, s.on_surface, None, radius, false)
        }
    }
}

/// A primary tab label; the active indicator is drawn underneath separately.
pub fn tab(s: Scheme, active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, st| {
        let content = if active { s.primary } else { s.on_surface_variant };
        let mut style = button_style(st, &s, None, s.on_surface, None, 0.0.into(), false);
        style.text_color = content;
        style
    }
}

// ------------------------------------------------------------- containers ---

pub fn surface(s: Scheme) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(s.surface.into()),
        text_color: Some(s.on_surface),
        ..Default::default()
    }
}

/// A filled card on the low surface container.
pub fn card(s: Scheme) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(s.surface_container_low.into()),
        text_color: Some(s.on_surface),
        border: Border {
            radius: 16.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// The stage the 3D model sits on.
pub fn stage(s: Scheme) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(s.surface_container_lowest.into()),
        border: Border {
            radius: 28.0.into(),
            color: s.outline_variant,
            width: 1.0,
        },
        ..Default::default()
    }
}

/// The outline around a segmented button.
pub fn segmented_frame(s: Scheme) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        border: Border {
            radius: FULL.into(),
            color: s.outline,
            width: 1.0,
        },
        ..Default::default()
    }
}

/// A solid strip of one colour: dividers, tab indicators.
pub fn fill(color: Color, radius: Radius) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(color.into()),
        border: Border {
            radius,
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn dialog(s: Scheme) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(s.surface_container_high.into()),
        text_color: Some(s.on_surface),
        border: Border {
            radius: 28.0.into(),
            ..Default::default()
        },
        shadow: elevation(&s, 3),
    }
}

pub fn scrim(_s: Scheme) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Color { a: 0.32, ..Color::BLACK }.into()),
        ..Default::default()
    }
}

/// A plain tooltip.
pub fn tooltip(s: Scheme) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(s.inverse_surface.into()),
        text_color: Some(s.inverse_on_surface),
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn snackbar(s: Scheme) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(s.inverse_surface.into()),
        text_color: Some(s.inverse_on_surface),
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        shadow: elevation(&s, 3),
    }
}

// ----------------------------------------------------------------- inputs ---

/// Outlined text field.
pub fn text_field(s: Scheme) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_, st| {
        let (color, width) = match st {
            text_input::Status::Focused => (s.primary, 2.0),
            text_input::Status::Hovered => (s.on_surface, 1.0),
            text_input::Status::Disabled => (alpha(s.on_surface, 0.12), 1.0),
            _ => (s.outline, 1.0),
        };
        text_input::Style {
            background: Background::Color(Color::TRANSPARENT),
            border: Border {
                color,
                width,
                radius: 4.0.into(),
            },
            icon: s.on_surface_variant,
            placeholder: s.on_surface_variant,
            value: s.on_surface,
            selection: alpha(s.primary, 0.4),
        }
    }
}

pub fn checkbox_style(s: Scheme) -> impl Fn(&Theme, checkbox::Status) -> checkbox::Style {
    move |_, st| {
        let (checked, hovered, disabled) = match st {
            checkbox::Status::Active { is_checked } => (is_checked, false, false),
            checkbox::Status::Hovered { is_checked } => (is_checked, true, false),
            checkbox::Status::Disabled { is_checked } => (is_checked, false, true),
        };
        let base = if disabled { alpha(s.on_surface, 0.38) } else if checked { s.primary } else { s.on_surface_variant };
        let base = if hovered && checked { layer(base, s.on_primary, 0.08) } else { base };
        checkbox::Style {
            background: Background::Color(if checked { base } else if hovered { alpha(s.on_surface, 0.08) } else { Color::TRANSPARENT }),
            icon_color: s.on_primary,
            border: Border {
                color: base,
                width: if checked { 0.0 } else { 2.0 },
                radius: 2.0.into(),
            },
            text_color: Some(s.on_surface),
        }
    }
}

/// The Material switch.
pub fn switch(s: Scheme) -> impl Fn(&Theme, toggler::Status) -> toggler::Style {
    move |_, st| {
        #[allow(unreachable_patterns)]
        let (on, hovered) = match st {
            toggler::Status::Active { is_toggled } => (is_toggled, false),
            toggler::Status::Hovered { is_toggled } => (is_toggled, true),
            _ => (false, false),
        };
        if on {
            toggler::Style {
                background: if hovered { layer(s.primary, s.on_primary, 0.08) } else { s.primary },
                background_border_width: 0.0,
                background_border_color: Color::TRANSPARENT,
                foreground: if hovered { s.primary_container } else { s.on_primary },
                foreground_border_width: 0.0,
                foreground_border_color: Color::TRANSPARENT,
            }
        } else {
            toggler::Style {
                background: if hovered {
                    layer(s.surface_container_highest, s.on_surface, 0.08)
                } else {
                    s.surface_container_highest
                },
                background_border_width: 2.0,
                background_border_color: s.outline,
                foreground: if hovered { s.on_surface_variant } else { s.outline },
                foreground_border_width: 0.0,
                foreground_border_color: Color::TRANSPARENT,
            }
        }
    }
}

pub fn divider(s: Scheme) -> impl Fn(&Theme) -> rule::Style {
    move |_| rule::Style {
        color: s.outline_variant,
        width: 1,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
    }
}

pub fn scroll(s: Scheme) -> impl Fn(&Theme, scrollable::Status) -> scrollable::Style {
    move |theme, st| {
        let mut style = scrollable::default(theme, st);
        let thumb = match st {
            scrollable::Status::Active => alpha(s.on_surface_variant, 0.35),
            _ => alpha(s.on_surface_variant, 0.6),
        };
        for rail in [&mut style.vertical_rail, &mut style.horizontal_rail] {
            rail.background = None;
            rail.border = Border::default();
            rail.scroller.color = thumb;
            rail.scroller.border = Border {
                radius: 999.0.into(),
                ..Default::default()
            };
        }
        style
    }
}
