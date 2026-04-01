use ratatui::style::Color;

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub user_msg_color: Color,
    pub assistant_msg_color: Color,
    pub tool_use_color: Color,
    pub tool_result_color: Color,
    pub error_color: Color,
    pub info_color: Color,
    pub dim_color: Color,
    pub border_color: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            name: "dark".to_string(),
            user_msg_color: Color::White,
            assistant_msg_color: Color::Cyan,
            tool_use_color: Color::Yellow,
            tool_result_color: Color::Green,
            error_color: Color::Red,
            info_color: Color::Blue,
            dim_color: Color::DarkGray,
            border_color: Color::Gray,
        }
    }

    pub fn light() -> Self {
        Self {
            name: "light".to_string(),
            user_msg_color: Color::Black,
            assistant_msg_color: Color::DarkGray,
            tool_use_color: Color::Rgb(180, 120, 0),
            tool_result_color: Color::Rgb(0, 120, 0),
            error_color: Color::Red,
            info_color: Color::Blue,
            dim_color: Color::Gray,
            border_color: Color::DarkGray,
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
        assert_eq!(theme.error_color, Color::Red);
        assert_eq!(theme.user_msg_color, Color::White);
    }

    #[test]
    fn test_light_theme() {
        let theme = Theme::light();
        assert_eq!(theme.name, "light");
        assert_eq!(theme.error_color, Color::Red);
        assert_eq!(theme.user_msg_color, Color::Black);
    }

    #[test]
    fn test_default_is_dark() {
        let theme = Theme::default();
        assert_eq!(theme.name, "dark");
    }
}
