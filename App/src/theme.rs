use egui::{Color32, CornerRadius, Rounding, Style, Visuals};

pub fn dark_theme() -> Style {
    Style {
        visuals: Visuals {
            dark_mode: true,
            window_fill: Color32::from_rgb(0x2d, 0x2d, 0x2d),
            panel_fill: Color32::from_rgb(0x2d, 0x2d, 0x2d),
            faint_bg_color: Color32::from_rgb(0x1e, 0x1e, 0x1e),
            extreme_bg_color: Color32::from_rgb(0x1e, 0x1e, 0x1e),
            code_bg_color: Color32::from_rgb(0x2a, 0x2a, 0x2a),
            warn_fg_color: Color32::from_rgb(0xe6, 0x61, 0x00),
            error_fg_color: Color32::from_rgb(0xe3, 0x3f, 0x3f),
            hyperlink_color: Color32::from_rgb(0x35, 0x84, 0xe4),
            selection: egui::style::Selection {
                bg_fill: Color32::from_rgba_premultiplied(0x35, 0x84, 0xe4, 80),
                stroke: egui::Stroke::new(1.0, Color32::from_rgb(0x35, 0x84, 0xe4)),
            },
            widgets: egui::style::Widgets {
                noninteractive: egui::style::WidgetVisuals {
                    bg_fill: Color32::from_rgb(0x2d, 0x2d, 0x2d),
                    weak_bg_fill: Color32::from_rgb(0x36, 0x36, 0x36),
                    bg_stroke: egui::Stroke::new(1.0, Color32::from_rgb(0x4a, 0x4a, 0x4a)),
                    fg_stroke: egui::Stroke::new(1.0, Color32::from_rgb(0xe8, 0xe8, 0xe8)),
                    corner_radius: CornerRadius::same(6),
                    expansion: 0.0,
                },
                inactive: egui::style::WidgetVisuals {
                    bg_fill: Color32::from_rgb(0x2d, 0x2d, 0x2d),
                    weak_bg_fill: Color32::from_rgb(0x36, 0x36, 0x36),
                    bg_stroke: egui::Stroke::new(1.0, Color32::from_rgb(0x4a, 0x4a, 0x4a)),
                    fg_stroke: egui::Stroke::new(1.0, Color32::from_rgb(0xe8, 0xe8, 0xe8)),
                    corner_radius: CornerRadius::same(6),
                    expansion: 0.0,
                },
                hovered: egui::style::WidgetVisuals {
                    bg_fill: Color32::from_rgb(0x2a, 0x2a, 0x2a),
                    weak_bg_fill: Color32::from_rgb(0x2a, 0x2a, 0x2a),
                    bg_stroke: egui::Stroke::new(1.0, Color32::from_rgb(0x66, 0x66, 0x66)),
                    fg_stroke: egui::Stroke::new(1.5, Color32::from_rgb(0xe8, 0xe8, 0xe8)),
                    corner_radius: CornerRadius::same(6),
                    expansion: 0.0,
                },
                active: egui::style::WidgetVisuals {
                    bg_fill: Color32::from_rgb(0x4a, 0x4a, 0x4a),
                    weak_bg_fill: Color32::from_rgb(0x4a, 0x4a, 0x4a),
                    bg_stroke: egui::Stroke::new(1.0, Color32::from_rgb(0x35, 0x84, 0xe4)),
                    fg_stroke: egui::Stroke::new(2.0, Color32::from_rgb(0xe8, 0xe8, 0xe8)),
                    corner_radius: CornerRadius::same(6),
                    expansion: 0.0,
                },
                open: egui::style::WidgetVisuals {
                    bg_fill: Color32::from_rgb(0x2d, 0x2d, 0x2d),
                    weak_bg_fill: Color32::from_rgb(0x36, 0x36, 0x36),
                    bg_stroke: egui::Stroke::new(1.0, Color32::from_rgb(0x35, 0x84, 0xe4)),
                    fg_stroke: egui::Stroke::new(1.0, Color32::from_rgb(0xe8, 0xe8, 0xe8)),
                    corner_radius: CornerRadius::same(6),
                    expansion: 0.0,
                },
            },
            window_corner_radius: CornerRadius::same(8),
            window_shadow: egui::epaint::Shadow {
                offset: [0, 4].into(),
                blur: 12,
                spread: 0,
                color: Color32::from_black_alpha(100),
            },
            ..Default::default()
        },
        ..Default::default()
    }
}
