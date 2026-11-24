use std::f32::consts::TAU;

use ab_glyph::{Font, FontRef, PxScale, ScaleFont, VariableFont, point};
use eframe::egui::{self, Color32, ColorImage, Painter, Rect, TextureHandle, TextureOptions};
use serde::{Deserialize, Serialize};
use tiny_skia::{
    Color, FillRule, LineCap, Paint, PathBuilder, Pixmap, PixmapPaint, Point, Stroke, Transform,
};

pub const FONT_BYTES: &[u8] = include_bytes!("../assets/ClockSansKR.ttf");
const DIAL_FONT_BYTES: &[u8] = include_bytes!("../assets/ClockSansKR-Bold.ttf");
const OSWALD_FONT_BYTES: &[u8] = include_bytes!("../assets/Oswald.ttf");
const ROBOTO_CONDENSED_BYTES: &[u8] = include_bytes!("../assets/RobotoCondensed.ttf");
const NOTO_SERIF_BYTES: &[u8] = include_bytes!("../assets/NotoSerif.ttf");
const PLEX_MONO_REGULAR_BYTES: &[u8] = include_bytes!("../assets/IBMPlexMono-Regular.ttf");
const PLEX_MONO_BOLD_BYTES: &[u8] = include_bytes!("../assets/IBMPlexMono-Bold.ttf");
const DESIGN_SIZE: f32 = 512.0;
const DESIGN_RADIUS: f32 = DESIGN_SIZE / 2.0 - 8.0;
const DESIGN_RIM: f32 = 16.0;

pub fn automatic_rim_width(diameter: f32) -> f32 {
    diameter / 2.0 * DESIGN_RIM / DESIGN_RADIUS
}

#[derive(Clone, Copy, PartialEq)]
struct DialGeometry {
    size: u32,
    radius: f32,
    design_radius: f32,
    rim_width: f32,
}

impl DialGeometry {
    fn new(size: u32, diameter: f32, rim_width: Option<f32>) -> Self {
        let design_radius = diameter / 2.0;
        let automatic = automatic_rim_width(diameter);
        let rim_width = rim_width.unwrap_or(automatic);
        Self {
            size,
            radius: design_radius + rim_width - automatic,
            design_radius,
            rim_width,
        }
    }

    fn details(self) -> Self {
        Self {
            radius: self.design_radius,
            ..self
        }
    }

    fn center(self) -> Point {
        let half = self.size as f32 / 2.0;
        Point::from_xy(half, half)
    }

    fn scale(self) -> f32 {
        self.radius / DESIGN_RADIUS
    }

    fn polar(self, turn: f32, length: f32) -> Point {
        let center = self.center();
        let (sin, cos) = (turn * TAU).sin_cos();
        Point::from_xy(
            center.x + self.radius * length * sin,
            center.y - self.radius * length * cos,
        )
    }
}

#[derive(Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Design {
    #[default]
    Ivory,
    Charcoal,
    Navy,
    #[serde(rename = "Steel")]
    LegacySteel,
}

impl Design {
    pub const ALL: [Self; 3] = [Self::Ivory, Self::Charcoal, Self::Navy];

    pub fn label(self, language: crate::Language) -> &'static str {
        match self {
            Self::Ivory => language.text("아이보리", "Ivory"),
            Self::Charcoal | Self::LegacySteel => language.text("차콜", "Charcoal"),
            Self::Navy => language.text("네이비", "Navy"),
        }
    }

    fn palette(self) -> Palette {
        match self {
            Self::Ivory => Palette {
                rim: [39, 47, 46],
                bevel: [145, 150, 136],
                face: [244, 239, 219],
                marks: [46, 53, 51],
                second: [178, 48, 39],
            },
            Self::Charcoal | Self::LegacySteel => Palette {
                rim: [24, 24, 27],
                bevel: [133, 138, 146],
                face: [46, 48, 53],
                marks: [231, 231, 223],
                second: [255, 107, 95],
            },
            Self::Navy => Palette {
                rim: [18, 28, 41],
                bevel: [156, 118, 65],
                face: [30, 48, 70],
                marks: [249, 233, 198],
                second: [248, 176, 86],
            },
        }
    }
}

#[derive(Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum HandStyle {
    Thick,
    #[default]
    Pointed,
}

#[derive(Clone, Copy)]
pub struct Appearance {
    pub design: Design,
    pub hands: HandAppearance,
    pub numerals: NumeralAppearance,
    pub diameter: f32,
    pub rim_width: Option<f32>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct HandAppearance {
    pub style: HandStyle,
    pub width_percent: u8,
}

impl Default for HandAppearance {
    fn default() -> Self {
        Self {
            style: HandStyle::default(),
            width_percent: 110,
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct NumeralAppearance {
    pub font: NumeralFont,
    pub size_percent: u8,
    pub bold: bool,
    pub visible: bool,
}

impl Default for NumeralAppearance {
    fn default() -> Self {
        Self {
            font: NumeralFont::default(),
            size_percent: 100,
            bold: true,
            visible: true,
        }
    }
}

impl Appearance {
    pub fn window_side(self) -> f32 {
        let automatic = automatic_rim_width(self.diameter);
        let extra_rim = self.rim_width.unwrap_or(automatic) - automatic;
        self.diameter + 2.0 + 2.0 * extra_rim
    }
}

#[derive(Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum NumeralFont {
    ClockSans,
    Oswald,
    #[default]
    RobotoCondensed,
    NotoSerif,
    IbmPlexMono,
}

impl NumeralFont {
    pub const ALL: [Self; 5] = [
        Self::ClockSans,
        Self::Oswald,
        Self::RobotoCondensed,
        Self::NotoSerif,
        Self::IbmPlexMono,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::ClockSans => "Clock Sans KR",
            Self::Oswald => "Oswald",
            Self::RobotoCondensed => "Roboto Condensed",
            Self::NotoSerif => "Noto Serif",
            Self::IbmPlexMono => "IBM Plex Mono",
        }
    }

    fn load(self, bold: bool) -> FontRef<'static> {
        let bytes = match (self, bold) {
            (Self::ClockSans, true) => DIAL_FONT_BYTES,
            (Self::ClockSans, false) => FONT_BYTES,
            (Self::Oswald, _) => OSWALD_FONT_BYTES,
            (Self::RobotoCondensed, _) => ROBOTO_CONDENSED_BYTES,
            (Self::NotoSerif, _) => NOTO_SERIF_BYTES,
            (Self::IbmPlexMono, false) => PLEX_MONO_REGULAR_BYTES,
            (Self::IbmPlexMono, true) => PLEX_MONO_BOLD_BYTES,
        };
        let mut font = FontRef::try_from_slice(bytes).expect("valid embedded dial font");
        if bold && matches!(self, Self::Oswald | Self::RobotoCondensed | Self::NotoSerif) {
            font.set_variation(b"wght", 700.0);
        }
        font
    }
}

impl HandStyle {
    pub const ALL: [Self; 2] = [Self::Thick, Self::Pointed];

    pub fn label(self, language: crate::Language) -> &'static str {
        match self {
            Self::Thick => language.text("두툼", "Rounded"),
            Self::Pointed => language.text("뾰족", "Pointed"),
        }
    }
}

type Rgb = [u8; 3];

struct Palette {
    rim: Rgb,
    bevel: Rgb,
    face: Rgb,
    marks: Rgb,
    second: Rgb,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct ClockTime {
    hour: u32,
    minute: u32,
    second: u32,
}

impl From<(u32, u32, u32)> for ClockTime {
    fn from((hour, minute, second): (u32, u32, u32)) -> Self {
        Self {
            hour,
            minute,
            second,
        }
    }
}

pub struct ClockFace {
    texture: TextureHandle,
    base: Pixmap,
    geometry: Option<DialGeometry>,
    design: Design,
    numerals: NumeralAppearance,
    hands: HandAppearance,
    time: Option<ClockTime>,
}

impl ClockFace {
    pub fn new(ctx: &egui::Context, name: &str) -> Self {
        let base = Pixmap::new(1, 1).expect("valid placeholder dimensions");
        let texture = ctx.load_texture(name, image(&base), TextureOptions::LINEAR);
        Self {
            texture,
            base,
            geometry: None,
            design: Design::default(),
            numerals: NumeralAppearance::default(),
            hands: HandAppearance::default(),
            time: None,
        }
    }

    pub fn paint(
        &mut self,
        painter: &Painter,
        bounds: Rect,
        appearance: Appearance,
        time: ClockTime,
        alpha: u8,
    ) {
        let side = bounds.width().min(bounds.height());
        if side < 4.0 {
            return;
        }
        let pixels_per_point = painter.ctx().pixels_per_point();
        let pixels = (side * pixels_per_point).round().max(4.0) as u32;
        let scale = if side + 0.75 / pixels_per_point < appearance.window_side() {
            side / appearance.window_side()
        } else {
            1.0
        };
        let scale = scale * pixels_per_point;
        let Appearance {
            design,
            hands,
            numerals,
            diameter,
            rim_width,
        } = appearance;
        let geometry = DialGeometry::new(
            pixels,
            diameter * scale,
            rim_width.map(|width| width * scale),
        );
        if self.design != design || self.numerals != numerals || self.geometry != Some(geometry) {
            self.base = make_face(design, numerals, geometry);
            self.geometry = Some(geometry);
            self.design = design;
            self.numerals = numerals;
            self.time = None;
        }
        if self.time != Some(time) || self.hands != hands {
            let mut frame = self.base.clone();
            draw_hands(
                &mut frame,
                geometry.details(),
                design,
                hands,
                time,
                pixels_per_point,
            );
            self.texture.set(image(&frame), TextureOptions::LINEAR);
            self.time = Some(time);
            self.hands = hands;
        }

        let side = pixels as f32 / pixels_per_point;
        let corner = bounds.center() - egui::Vec2::splat(side / 2.0);
        let corner = egui::pos2(
            (corner.x * pixels_per_point).round() / pixels_per_point,
            (corner.y * pixels_per_point).round() / pixels_per_point,
        );
        painter.image(
            self.texture.id(),
            Rect::from_min_size(corner, egui::Vec2::splat(side)),
            Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            Color32::from_white_alpha(alpha),
        );
    }
}

fn image(pixmap: &Pixmap) -> ColorImage {
    ColorImage::from_rgba_premultiplied(
        [pixmap.width() as usize, pixmap.height() as usize],
        pixmap.data(),
    )
}

pub fn icon() -> egui::IconData {
    let design = Design::default();
    let geometry = DialGeometry::new(DESIGN_SIZE as u32, 2.0 * DESIGN_RADIUS, None);
    let mut face = make_face(design, NumeralAppearance::default(), geometry);
    draw_hands(
        &mut face,
        geometry,
        design,
        HandAppearance::default(),
        (10, 8, 34).into(),
        DESIGN_SIZE / 64.0,
    );
    let mut icon = Pixmap::new(64, 64).expect("valid icon dimensions");
    icon.draw_pixmap(
        0,
        0,
        face.as_ref(),
        &PixmapPaint {
            quality: tiny_skia::FilterQuality::Bicubic,
            ..Default::default()
        },
        Transform::from_scale(64.0 / DESIGN_SIZE, 64.0 / DESIGN_SIZE),
        None,
    );
    let rgba = icon
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect();
    egui::IconData {
        rgba,
        width: 64,
        height: 64,
    }
}

fn make_face(design: Design, numerals: NumeralAppearance, geometry: DialGeometry) -> Pixmap {
    let mut face = Pixmap::new(geometry.size, geometry.size).expect("valid clock dimensions");
    let palette = design.palette();
    let center = geometry.center();
    let radius = geometry.radius;
    let details = geometry.details();
    let scale = details.scale();
    let inside_rim = radius - geometry.rim_width;
    circle(&mut face, center, radius, palette.rim);
    circle(&mut face, center, inside_rim, palette.bevel);
    circle(&mut face, center, inside_rim - 5.0 * scale, palette.face);

    for mark in 0..60 {
        let direction = mark as f32 / 60.0;
        let (start, width) = match mark % 5 {
            0 => (0.81, 5.0),
            _ => (0.87, 2.0),
        };
        line(
            &mut face,
            details.polar(direction, start),
            details.polar(direction, 0.9),
            (width * scale).max(0.8),
            palette.marks,
        );
    }

    if numerals.visible {
        let font = numerals.font.load(numerals.bold);
        let size_ratio = f32::from(numerals.size_percent) / 100.0;
        let label_radius = 0.69 - (size_ratio - 1.0).max(0.0) * 0.16;
        for hour in 1..=12 {
            number(
                &mut face,
                &font,
                hour,
                details.polar(hour as f32 / 12.0, label_radius),
                (60.0 * scale).max(18.0) * size_ratio,
                palette.marks,
            );
        }
    }
    face
}

fn draw_hands(
    face: &mut Pixmap,
    geometry: DialGeometry,
    design: Design,
    hands: HandAppearance,
    time: ClockTime,
    pixels_per_point: f32,
) {
    let palette = design.palette();
    let scale = geometry.scale();
    let width_ratio = f32::from(hands.width_percent) / 100.0;
    let second = time.second as f32 / 60.0;
    let minute = (time.minute as f32 + second) / 60.0;
    let hour = (time.hour as f32 + minute) / 12.0;
    for (turn, tail, tip, width) in [(hour, -0.07, 0.47, 13.0), (minute, -0.10, 0.68, 9.0)] {
        let width = width * width_ratio;
        if hands.style == HandStyle::Pointed {
            pointed_hand(face, geometry, turn, tip, 2.0 * width, palette.marks);
        } else {
            line(
                face,
                geometry.polar(turn, tail),
                geometry.polar(turn, tip),
                width * scale,
                palette.marks,
            );
        }
    }
    let center = geometry.center();
    line(
        face,
        geometry.polar(second, -0.22),
        center,
        3.0 * pixels_per_point,
        palette.second,
    );
    line(
        face,
        center,
        geometry.polar(second, 0.79),
        pixels_per_point,
        palette.second,
    );
    circle(face, center, 10.0 * scale, palette.marks);
    circle(face, center, 5.0 * scale, palette.second);
}

fn pointed_hand(
    target: &mut Pixmap,
    geometry: DialGeometry,
    turn: f32,
    length: f32,
    width: f32,
    color: Rgb,
) {
    let half_width = width * geometry.scale() / (2.0 * geometry.radius);
    let tail = geometry.polar(turn, -0.14);
    let tip = geometry.polar(turn, length);
    let left = geometry.polar(turn - 0.25, half_width);
    let right = geometry.polar(turn + 0.25, half_width);
    let mut builder = PathBuilder::new();
    builder.move_to(tail.x, tail.y);
    builder.line_to(left.x, left.y);
    builder.line_to(tip.x, tip.y);
    builder.line_to(right.x, right.y);
    builder.close();
    let path = builder.finish().expect("valid pointed hand");
    target.fill_path(
        &path,
        &paint(color),
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

fn paint([r, g, b]: Rgb) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color(Color::from_rgba8(r, g, b, 255));
    paint
}

fn circle(target: &mut Pixmap, center: Point, radius: f32, color: Rgb) {
    let path = PathBuilder::from_circle(center.x, center.y, radius).expect("valid circle radius");
    target.fill_path(
        &path,
        &paint(color),
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

fn line(target: &mut Pixmap, start: Point, end: Point, width: f32, color: Rgb) {
    let mut builder = PathBuilder::new();
    builder.move_to(start.x, start.y);
    builder.line_to(end.x, end.y);
    let path = builder.finish().expect("valid line");
    let stroke = Stroke {
        width,
        line_cap: LineCap::Round,
        ..Default::default()
    };
    target.stroke_path(&path, &paint(color), &stroke, Transform::identity(), None);
}

fn number(
    target: &mut Pixmap,
    font: &FontRef<'_>,
    value: u32,
    center: Point,
    font_size: f32,
    [r, g, b]: Rgb,
) {
    let scale = PxScale::from(font_size);
    let scaled = font.as_scaled(scale);
    let text = value.to_string();
    let width = text
        .chars()
        .map(|digit| scaled.h_advance(font.glyph_id(digit)))
        .sum::<f32>();
    let mut x = center.x - width / 2.0;
    let baseline = center.y + font_size * (12.0 / 43.0);

    for digit in text.chars() {
        let id = font.glyph_id(digit);
        if let Some(outline) =
            font.outline_glyph(id.with_scale_and_position(scale, point(x, baseline)))
        {
            let bounds = outline.px_bounds();
            let width = bounds.width() as usize;
            let mut glyph =
                Pixmap::new(width as u32, bounds.height() as u32).expect("visible numeral");
            outline.draw(|x, y, coverage| {
                let alpha = (coverage * 255.0).round() as u8;
                let color = tiny_skia::ColorU8::from_rgba(r, g, b, alpha).premultiply();
                glyph.pixels_mut()[y as usize * width + x as usize] = color;
            });
            target.draw_pixmap(
                bounds.min.x as i32,
                bounds.min.y as i32,
                glyph.as_ref(),
                &PixmapPaint::default(),
                Transform::identity(),
                None,
            );
        }
        x += scaled.h_advance(id);
    }
}
