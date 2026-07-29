use egui::{Color32, CornerRadius, Style, Visuals};

pub const BLUE: Color32 = Color32::from_rgb(0x35, 0x84, 0xe4);
pub const BLUE_LIGHT: Color32 = Color32::from_rgb(0x6a, 0xa8, 0xef);
pub const BLUE_HOVER: Color32 = Color32::from_rgb(0x47, 0x91, 0xe6);
pub const BLUE_ACTIVE: Color32 = Color32::from_rgb(0x2f, 0x74, 0xc9);
pub const SUCCESS: Color32 = Color32::from_rgb(0x2e, 0xc2, 0x7e);
pub const DESTRUCTIVE: Color32 = Color32::from_rgb(0xf0, 0x47, 0x5a);
pub const TEXT: Color32 = Color32::from_rgb(0xf2, 0xf2, 0xf5);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0xa3, 0xa3, 0xae);

const BG: Color32 = Color32::from_rgb(0x10, 0x0f, 0x13);
const SURFACE: Color32 = Color32::from_rgb(0x1b, 0x1b, 0x22);
const SURFACE_2: Color32 = Color32::from_rgb(0x23, 0x23, 0x2b);
const SURFACE_HOVER: Color32 = Color32::from_rgb(0x27, 0x27, 0x30);
const SURFACE_ACTIVE: Color32 = Color32::from_rgb(0x30, 0x30, 0x3b);
const BORDER: Color32 = Color32::from_rgb(0x35, 0x35, 0x40);
const BORDER_STRONG: Color32 = Color32::from_rgb(0x48, 0x48, 0x56);

pub fn dark_theme() -> Style {
    Style {
        visuals: Visuals {
            dark_mode: true,
            window_fill: SURFACE,
            panel_fill: BG,
            faint_bg_color: Color32::from_rgb(0x18, 0x18, 0x1e),
            extreme_bg_color: Color32::from_rgb(0x14, 0x13, 0x18),
            code_bg_color: SURFACE_2,
            warn_fg_color: Color32::from_rgb(0xe6, 0xa2, 0x3c),
            error_fg_color: DESTRUCTIVE,
            hyperlink_color: BLUE_LIGHT,
            selection: egui::style::Selection {
                bg_fill: Color32::from_rgba_premultiplied(0x35, 0x84, 0xe4, 80),
                stroke: egui::Stroke::new(1.0, BLUE_LIGHT),
            },
            widgets: egui::style::Widgets {
                noninteractive: egui::style::WidgetVisuals {
                    bg_fill: SURFACE,
                    weak_bg_fill: SURFACE_2,
                    bg_stroke: egui::Stroke::new(1.0, BORDER),
                    fg_stroke: egui::Stroke::new(1.0, TEXT),
                    corner_radius: CornerRadius::same(10),
                    expansion: 0.0,
                },
                inactive: egui::style::WidgetVisuals {
                    bg_fill: SURFACE_2,
                    weak_bg_fill: SURFACE_2,
                    bg_stroke: egui::Stroke::new(1.0, BORDER),
                    fg_stroke: egui::Stroke::new(1.0, TEXT),
                    corner_radius: CornerRadius::same(10),
                    expansion: 0.0,
                },
                hovered: egui::style::WidgetVisuals {
                    bg_fill: SURFACE_HOVER,
                    weak_bg_fill: SURFACE_HOVER,
                    bg_stroke: egui::Stroke::new(1.0, BORDER_STRONG),
                    fg_stroke: egui::Stroke::new(1.5, TEXT),
                    corner_radius: CornerRadius::same(10),
                    expansion: 1.0,
                },
                active: egui::style::WidgetVisuals {
                    bg_fill: SURFACE_ACTIVE,
                    weak_bg_fill: SURFACE_ACTIVE,
                    bg_stroke: egui::Stroke::new(1.5, BLUE),
                    fg_stroke: egui::Stroke::new(2.0, TEXT),
                    corner_radius: CornerRadius::same(10),
                    expansion: 0.0,
                },
                open: egui::style::WidgetVisuals {
                    bg_fill: SURFACE_2,
                    weak_bg_fill: SURFACE_2,
                    bg_stroke: egui::Stroke::new(1.0, BLUE),
                    fg_stroke: egui::Stroke::new(1.0, TEXT),
                    corner_radius: CornerRadius::same(10),
                    expansion: 0.0,
                },
            },
            window_corner_radius: CornerRadius::same(16),
            window_stroke: egui::Stroke::new(1.0, BORDER),
            window_shadow: egui::epaint::Shadow {
                offset: [0, 10].into(),
                blur: 36,
                spread: 0,
                color: Color32::from_black_alpha(130),
            },
            popup_shadow: egui::epaint::Shadow {
                offset: [0, 6].into(),
                blur: 20,
                spread: 0,
                color: Color32::from_black_alpha(120),
            },
            ..Default::default()
        },
        spacing: egui::style::Spacing {
            item_spacing: egui::vec2(8.0, 10.0),
            button_padding: egui::vec2(14.0, 8.0),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// A pill-shaped primary call-to-action button in the SyncNotes brand blue,
/// with a soft hover/press animation. Cheap: egui only keeps repainting
/// while the animated value is still in motion, not on every frame.
pub fn primary_button(ui: &mut egui::Ui, text: &str, size: egui::Vec2) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

    let id = response.id;
    let hovered = ui.ctx().animate_bool_responsive(id.with("hover"), response.hovered());
    let pressed = response.is_pointer_button_down_on();

    if ui.is_rect_visible(rect) {
        let base = BLUE;
        let lift = if pressed { 0.0 } else { hovered * 3.0 };
        let color = if pressed {
            BLUE_ACTIVE
        } else {
            lerp_color(base, BLUE_HOVER, hovered)
        };

        let draw_rect = rect.translate(egui::vec2(0.0, -lift));
        let radius = draw_rect.height() / 2.0;

        if hovered > 0.01 && !pressed {
            ui.painter().rect_filled(
                draw_rect.expand(hovered * 2.0),
                radius + hovered * 2.0,
                Color32::from_rgba_unmultiplied(0x35, 0x84, 0xe4, (hovered * 40.0) as u8),
            );
        }

        ui.painter().rect_filled(draw_rect, radius, color);
        ui.painter().text(
            draw_rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            egui::FontId::proportional(15.0),
            Color32::WHITE,
        );
    }

    response
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
    )
}
