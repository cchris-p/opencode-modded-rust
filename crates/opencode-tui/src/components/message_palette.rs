use ratatui::style::Color;

use crate::theme::Theme;

/// Every conversation surface (user, assistant, thinking) shares the dialog
/// `background_panel` treatment so the transcript reads as one uniform field,
/// exactly like an open menu. Line-level hierarchy is carried by borders,
/// glyphs, and foreground color instead of background lightness.
pub fn user_message_bg(theme: &Theme) -> Color {
    theme.background_panel
}

pub fn assistant_message_bg(theme: &Theme) -> Color {
    theme.background_panel
}

pub fn thinking_message_bg(theme: &Theme) -> Color {
    theme.background_panel
}

pub fn assistant_border_color(theme: &Theme) -> Color {
    theme.info
}

pub fn thinking_border_color(theme: &Theme) -> Color {
    theme.background_element
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversation_surfaces_share_theme_panel_background() {
        let mut theme = Theme::dark();
        theme.background_panel = Color::Rgb(9, 8, 7);

        assert_eq!(user_message_bg(&theme), theme.background_panel);
        assert_eq!(assistant_message_bg(&theme), theme.background_panel);
        assert_eq!(thinking_message_bg(&theme), theme.background_panel);
    }
}
