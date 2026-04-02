use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,

    // Message roles
    pub user_msg_color: Color,
    pub assistant_msg_color: Color,
    pub system_msg_color: Color,
    pub tool_use_color: Color,
    pub tool_result_color: Color,

    // Status
    pub error_color: Color,
    pub info_color: Color,
    pub dim_color: Color,
    pub border_color: Color,

    // Banner
    pub banner_title_color: Color,
    pub banner_info_color: Color,

    // Thinking
    pub thinking_color: Color,

    // Code blocks
    pub code_bg: Color,
    pub code_border: Color,

    // Markdown
    pub heading_color: Color,
    pub link_color: Color,
    pub bold_color: Color,

    // Status bar
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,

    // Input
    pub input_border_active: Color,
    pub input_border_inactive: Color,

    // Permission
    pub permission_border: Color,
    pub permission_highlight: Color,

    // Spinner
    pub spinner_color: Color,
}

impl Theme {
    /// Claude Code inspired dark theme — terracotta accents, muted palette.
    pub fn dark() -> Self {
        Self {
            name: "dark".to_string(),

            // Claude Code: user = muted blue, assistant = warm white
            user_msg_color: Color::Rgb(120, 160, 220),   // soft blue
            assistant_msg_color: Color::Rgb(220, 220, 215), // warm off-white
            system_msg_color: Color::Rgb(200, 80, 80),    // muted red
            tool_use_color: Color::Rgb(218, 119, 86),     // terracotta (#DA7756)
            tool_result_color: Color::Rgb(130, 190, 130), // soft green

            error_color: Color::Rgb(220, 90, 90),         // warm red
            info_color: Color::Rgb(130, 170, 220),        // soft blue
            dim_color: Color::Rgb(100, 100, 110),         // medium gray
            border_color: Color::Rgb(55, 55, 65),         // subtle border

            // Banner: terracotta title
            banner_title_color: Color::Rgb(218, 119, 86), // terracotta
            banner_info_color: Color::Rgb(100, 100, 110),

            thinking_color: Color::Rgb(140, 140, 165),    // lavender-gray

            // Code: dark bg with subtle border
            code_bg: Color::Rgb(25, 25, 35),
            code_border: Color::Rgb(60, 60, 75),

            // Markdown
            heading_color: Color::Rgb(218, 119, 86),      // terracotta headings
            link_color: Color::Rgb(120, 180, 230),        // soft blue links
            bold_color: Color::Rgb(235, 235, 230),        // bright white

            // Status bar: dark, low contrast
            status_bar_bg: Color::Rgb(25, 25, 35),
            status_bar_fg: Color::Rgb(100, 100, 115),

            // Input: terracotta active, dim inactive
            input_border_active: Color::Rgb(218, 119, 86),
            input_border_inactive: Color::Rgb(55, 55, 65),

            // Permission: amber
            permission_border: Color::Rgb(220, 180, 60),
            permission_highlight: Color::Rgb(220, 180, 60),

            // Spinner: terracotta
            spinner_color: Color::Rgb(218, 119, 86),
        }
    }

    pub fn light() -> Self {
        Self {
            name: "light".to_string(),

            user_msg_color: Color::Rgb(0, 120, 0),
            assistant_msg_color: Color::Black,
            system_msg_color: Color::Red,
            tool_use_color: Color::Rgb(180, 120, 0),
            tool_result_color: Color::Rgb(0, 120, 120),

            error_color: Color::Red,
            info_color: Color::Blue,
            dim_color: Color::Gray,
            border_color: Color::Rgb(200, 200, 200),

            banner_title_color: Color::Rgb(100, 60, 200),
            banner_info_color: Color::Gray,

            thinking_color: Color::Gray,

            code_bg: Color::Rgb(240, 240, 245),
            code_border: Color::Rgb(200, 200, 210),

            heading_color: Color::Rgb(40, 80, 180),
            link_color: Color::Rgb(0, 100, 200),
            bold_color: Color::Black,

            status_bar_bg: Color::Rgb(235, 235, 240),
            status_bar_fg: Color::Gray,

            input_border_active: Color::Rgb(100, 60, 200),
            input_border_inactive: Color::Rgb(200, 200, 200),

            permission_border: Color::Rgb(200, 150, 0),
            permission_highlight: Color::Rgb(200, 150, 0),

            spinner_color: Color::Rgb(100, 60, 200),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dark_theme() {
        let theme = Theme::dark();
        assert_eq!(theme.name, "dark");
        // Terracotta accent for tool_use
        assert_eq!(theme.tool_use_color, Color::Rgb(218, 119, 86));
    }

    #[test]
    fn test_light_theme() {
        let theme = Theme::light();
        assert_eq!(theme.name, "light");
    }

    #[test]
    fn test_default_is_dark() {
        let theme = Theme::default();
        assert_eq!(theme.name, "dark");
    }
}
