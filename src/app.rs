use crate::types::*;
use ratatui::{style::{Color, Modifier, Style}, text::{Line, Span}};
use std::error::Error;
use url::Url;
use html2text::render::text_renderer::RichAnnotation;
use image::GenericImageView;

pub struct App {
    pub current_url: String,
    pub content_lines: Vec<Line<'static>>,
    pub links: Vec<LinkData>,
    pub selected_link_idx: usize,
    pub scroll: u16,
    pub status: String,
    pub mode: Mode,
    pub command_buffer: String,
    pub history: Vec<String>,
    pub future: Vec<String>,
    pub image_preview: Option<Vec<String>>,
}

impl App {
    pub fn new(start_url: &str) -> Self {
        Self {
            current_url: start_url.to_string(),
            content_lines: Vec::new(),
            links: Vec::new(),
            selected_link_idx: 0,
            scroll: 0,
            status: String::from("Voyager Ready"),
            mode: Mode::Normal,
            command_buffer: String::new(),
            history: Vec::new(),
            future: Vec::new(),
            image_preview: None,
        }
    }

    pub async fn navigate(&mut self, mut url: String) -> Result<(), Box<dyn Error>> {
        if !url.starts_with("http://") && !url.starts_with("https://") {
            url = format!("https://{}", url);
        }
        if !self.current_url.is_empty() {
            self.history.push(self.current_url.clone());
        }
        self.future.clear();
        self.current_url = url;
        self.fetch_page().await
    }

    pub async fn fetch_page(&mut self) -> Result<(), Box<dyn Error>> {
        self.status = format!("Fetching {}...", self.current_url);
        let client = reqwest::Client::builder().user_agent("Voyager-Browser/0.1.0").build()?;
        let res = client.get(&self.current_url).send().await?;
        let base_url = Url::parse(&self.current_url)?;
        let html = res.text().await?;

        let mut new_lines = Vec::new();
        let mut new_links = Vec::new();
        let mut link_counter = 0;

        let width = 100;
        let rich_lines = html2text::from_read_rich(html.as_bytes(), width);

        for line in rich_lines {
            let mut spans = Vec::new();
            for tagged_string in line.tagged_strings() {
                let mut style = Style::default();
                let mut current_link = None;

                for annotation in &tagged_string.tag {
                    match annotation {
                        RichAnnotation::Link(target) => {
                            let abs = base_url.join(target).map(|u| u.to_string()).unwrap_or_else(|_| target.clone());
                            current_link = Some((abs, LinkType::Web));
                        }
                        RichAnnotation::Image(src) => {
                            let abs = base_url.join(src).map(|u| u.to_string()).unwrap_or_else(|_| src.clone());
                            current_link = Some((abs, LinkType::Image));
                        }
                        RichAnnotation::Strong => style = style.add_modifier(Modifier::BOLD),
                        _ => {}
                    }
                }

                if let Some((url, ltype)) = current_link {
                    let label = format!("[{}]", link_counter);
                    spans.push(Span::styled(label, Style::default().fg(Color::DarkGray)));
                    
                    let link_style = match ltype {
                        LinkType::Web => Style::default().fg(LINK_COLOR_WEB).add_modifier(Modifier::UNDERLINED),
                        LinkType::Image => Style::default().fg(LINK_COLOR_IMG).add_modifier(Modifier::ITALIC),
                    };
                    spans.push(Span::styled(tagged_string.s.clone(), link_style));
                    new_links.push(LinkData { url, link_type: ltype });
                    link_counter += 1;
                } else {
                    spans.push(Span::styled(tagged_string.s.clone(), style));
                }
            }
            new_lines.push(Line::from(spans));
        }

        self.content_lines = new_lines;
        self.links = new_links;
        self.selected_link_idx = 0;
        self.scroll = 0;
        self.status = format!("Loaded: {}", self.current_url);
        Ok(())
    }

    pub async fn preview_image(&mut self, url: &str) -> Result<(), Box<dyn Error>> {
        self.status = format!("Processing Image AA: {}...", url);
        let res = reqwest::get(url).await?.bytes().await?;
        let img = image::load_from_memory(&res)?;
        
        let (w, h) = img.dimensions();
        let new_w = 80u32;
        let new_h = (new_w as f32 * (h as f32 / w as f32) * 0.5) as u32;
        let resized = img.resize_exact(new_w, new_h, image::imageops::FilterType::Nearest);
        let gray = resized.to_luma8();

        let charset = " `.!|:-=m+*#%@";
        let mut aa = Vec::new();
        for y in 0..new_h {
            let mut row = String::new();
            for x in 0..new_w {
                let p = gray.get_pixel(x, y)[0];
                let idx = (p as usize * (charset.len() - 1)) / 255;
                row.push(charset.chars().nth(idx).unwrap());
            }
            aa.push(row);
        }
        self.image_preview = Some(aa);
        self.status = "Image AA Loaded. Press ESC to close.".to_string();
        Ok(())
    }

    pub fn render_content(&self) -> Vec<Line<'static>> {
        let mut rendered = Vec::new();
        let mut current_idx = 0;
        for line in &self.content_lines {
            let mut spans = Vec::new();
            for span in &line.spans {
                let mut s = span.clone();
                if s.style.fg == Some(LINK_COLOR_WEB) || s.style.fg == Some(LINK_COLOR_IMG) {
                    if current_idx == self.selected_link_idx {
                        s.style = s.style.bg(Color::Yellow).fg(Color::Black).add_modifier(Modifier::BOLD);
                    }
                    current_idx += 1;
                }
                spans.push(s);
            }
            rendered.push(Line::from(spans));
        }
        rendered
    }
}
