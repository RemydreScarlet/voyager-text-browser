use ratatui::style::Color;

#[derive(Debug, PartialEq, Clone)]
pub enum Mode {
    Normal,
    Command,
}

#[derive(Debug, PartialEq, Clone)]
pub enum LinkType {
    Web,
    Image,
}

#[derive(Clone)]
pub struct LinkData {
    pub url: String,
    pub link_type: LinkType,
}

pub const LINK_COLOR_WEB: Color = Color::Blue;
pub const LINK_COLOR_IMG: Color = Color::Magenta;
