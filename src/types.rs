use ratatui::style::Color;

#[derive(Debug, PartialEq, Clone, Copy)]
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

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PageContent {
    pub title: String,
    pub lines: Vec<Vec<TextSegment>>,
    pub links: Vec<LinkItem>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TextSegment {
    pub text: String,
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub underline: bool,
    pub color: Option<String>,
    #[serde(default)]
    pub link_idx: i32,
    pub is_block: Option<bool>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LinkItem {
    pub url: String,
    pub text: String,
    pub idx: usize,
}
