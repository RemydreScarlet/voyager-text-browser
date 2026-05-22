use crate::types::*;
use dpi::PhysicalSize;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use servo::{
    EventLoopWaker, JSValue, LoadStatus, Servo, ServoBuilder, SoftwareRenderingContext, WebView,
    WebViewBuilder, WebViewDelegate,
};
use std::{cell::RefCell, rc::Rc};
use url::Url;

struct Waker;

impl EventLoopWaker for Waker {
    fn wake(&self) {}
    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(Waker)
    }
}

const EXTRACT_SCRIPT: &str = r#"(function(){
var r={title:document.title||'',lines:[],links:[]};
var li=0,as=document.querySelectorAll('a[href]');
for(var i=0;i<as.length;i++){var a=as[i];if(a.href&&a.textContent.trim()){a.setAttribute('di',li);r.links.push({url:a.href,text:a.textContent.trim().substring(0,200),idx:li++});}}
var cl=[],cb=null;
function fl(){if(cl.length>0){r.lines.push(cl);cl=[];}}
function w(n,bl){
 if(n.nodeType===3){var t=n.textContent;if(!t||!t.trim())return;
  var e=n.parentElement;if(!e||e.offsetParent===null)return;
  var s=window.getComputedStyle(e);
  if(s.display==='none'||s.visibility==='hidden')return;
  if(bl!==cb&&cl.length>0)fl();cb=bl;
  var li2=-1,p=e;while(p&&p.hasAttribute&&!p.hasAttribute('di'))p=p.parentElement;
  if(p&&p.hasAttribute('di'))li2=parseInt(p.getAttribute('di'));
  var tag=e.tagName;
  cl.push({text:t,bold:parseInt(s.fontWeight)>=700||tag==='B'||tag==='STRONG',italic:s.fontStyle==='italic'||tag==='I'||tag==='EM',underline:s.textDecoration.indexOf('underline')>=0||tag==='U',color:s.color,linkIdx:li2});
 }else if(n.nodeType===1){var tag=n.tagName;
  if(tag==='SCRIPT'||tag==='STYLE'||tag==='NOSCRIPT'||tag==='SVG'||tag==='IFRAME')return;
  var isBl=['P','DIV','H1','H2','H3','H4','H5','H6','LI','BLOCKQUOTE','PRE','HR','TR','TABLE','OL','UL'].indexOf(tag)>=0;
  for(var i=0;i<n.childNodes.length;i++)w(n.childNodes[i],isBl?n:bl);
 }
}
if(document.body)w(document.body,null);fl();
return JSON.stringify(r);})()"#;

pub struct DisplayData {
    pub content_lines: Vec<Line<'static>>,
    pub links: Vec<LinkData>,
    pub selected_link_idx: usize,
    pub scroll: u16,
    pub status: String,
    pub current_url: String,
    pub mode: Mode,
    pub command_buffer: String,
    pub image_preview: Option<Vec<String>>,
    pub history: Vec<String>,
    pub future: Vec<String>,
    pub needs_redraw: bool,
    needs_extract: bool,
    extracting: bool,
}

impl DisplayData {
    fn new(start_url: &str) -> Self {
        Self {
            content_lines: Vec::new(),
            links: Vec::new(),
            selected_link_idx: 0,
            scroll: 0,
            status: "Initializing...".to_string(),
            current_url: start_url.to_string(),
            mode: Mode::Normal,
            command_buffer: String::new(),
            image_preview: None,
            history: Vec::new(),
            future: Vec::new(),
            needs_redraw: true,
            needs_extract: false,
            extracting: false,
        }
    }
}

struct AppDelegate {
    data: Rc<RefCell<DisplayData>>,
}

impl WebViewDelegate for AppDelegate {
    fn request_navigation(&self, _webview: WebView, _request: servo::NavigationRequest) {}

    fn notify_load_status_changed(&self, _webview: WebView, status: LoadStatus) {
        let mut d = self.data.borrow_mut();
        match status {
            LoadStatus::Started => {
                d.status = "Loading...".to_string();
                d.needs_extract = false;
                d.extracting = false;
                d.needs_redraw = true;
            }
            LoadStatus::HeadParsed => {}
            LoadStatus::Complete => {
                d.status = "Processing...".to_string();
                d.needs_extract = true;
                d.needs_redraw = true;
            }
        }
    }

    fn notify_url_changed(&self, _webview: WebView, url: Url) {
        let mut d = self.data.borrow_mut();
        d.current_url = url.to_string();
        d.needs_redraw = true;
    }

    fn notify_new_frame_ready(&self, _webview: WebView) {
        self.data.borrow_mut().needs_redraw = true;
    }
}

pub struct App {
    pub servo: Servo,
    pub webview: WebView,
    pub data: Rc<RefCell<DisplayData>>,
    _delegate: Rc<AppDelegate>,
}

impl App {
    pub fn new(start_url: &str) -> Self {
        let data = Rc::new(RefCell::new(DisplayData::new(start_url)));

        let servo = ServoBuilder::default()
            .event_loop_waker(Box::new(Waker))
            .build();

        let viewport = PhysicalSize::new(1280, 1024);
        let render_ctx = Rc::new(
            SoftwareRenderingContext::new(viewport)
                .expect("Failed to create software rendering context (try: apt install libegl-dev libosmesa6-dev)"),
        );

        let delegate = Rc::new(AppDelegate { data: data.clone() });

        let url = Url::parse(start_url).unwrap_or_else(|_| {
            Url::parse("https://www.rust-lang.org").unwrap()
        });

        let webview = WebViewBuilder::new(&servo, render_ctx)
            .delegate(delegate.clone())
            .url(url)
            .build();

        Self {
            servo,
            webview,
            data,
            _delegate: delegate,
        }
    }

    pub fn navigate(&mut self, url_str: String) {
        let url_str = if !url_str.starts_with("http://") && !url_str.starts_with("https://") {
            format!("https://{}", url_str)
        } else {
            url_str
        };
        let prev_url = self.data.borrow().current_url.clone();
        {
            let mut d = self.data.borrow_mut();
            d.history.push(prev_url);
            d.future.clear();
            d.current_url = url_str.clone();
            d.status = "Loading...".to_string();
            d.needs_redraw = true;
        }
        if let Ok(url) = Url::parse(&url_str) {
            self.webview.load(url);
        }
    }

    pub fn process_extraction(&mut self) {
        if self.data.borrow().extracting {
            return;
        }
        if !self.data.borrow().needs_extract {
            return;
        }
        self.data.borrow_mut().needs_extract = false;
        self.data.borrow_mut().extracting = true;

        let captured = self.data.clone();
        self.webview
            .evaluate_javascript(EXTRACT_SCRIPT, move |result| {
                let mut d = captured.borrow_mut();
                match result {
                    Ok(JSValue::String(json)) => {
                        match serde_json::from_str::<PageContent>(&json) {
                            Ok(content) => {
                                d.content_lines = convert_content(&content, &mut d.links);
                                d.status = format!("Loaded: {}", d.current_url);
                            }
                            Err(e) => {
                                d.status = format!("Parse error: {}", e);
                                d.content_lines = vec![Line::from(format!(
                                    "Parse error: {}",
                                    e
                                ))];
                            }
                        }
                    }
                    Ok(val) => {
                        d.status = format!("Unexpected JS result: {:?}", val);
                    }
                    Err(e) => {
                        d.status = format!("JS error: {:?}", e);
                    }
                }
                d.extracting = false;
                d.needs_redraw = true;
            });
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        let mut d = self.data.borrow_mut();
        d.needs_redraw = true;

        if d.image_preview.is_some() {
            if key.code == crossterm::event::KeyCode::Esc {
                d.image_preview = None;
                d.status = "Closed preview".to_string();
            }
            return true;
        }

        match d.mode {
            Mode::Normal => match key.code {
                crossterm::event::KeyCode::Char(':') => {
                    d.mode = Mode::Command;
                    d.command_buffer.clear();
                }
                crossterm::event::KeyCode::Char('j') => {
                    d.scroll = d.scroll.saturating_add(1);
                }
                crossterm::event::KeyCode::Char('k') => {
                    d.scroll = d.scroll.saturating_sub(1);
                }
                crossterm::event::KeyCode::Char('l') | crossterm::event::KeyCode::Tab => {
                    if !d.links.is_empty() {
                        d.selected_link_idx =
                            (d.selected_link_idx + 1) % d.links.len();
                    }
                }
                crossterm::event::KeyCode::Char('h') => {
                    if !d.links.is_empty() {
                        d.selected_link_idx = if d.selected_link_idx == 0 {
                            d.links.len() - 1
                        } else {
                            d.selected_link_idx - 1
                        };
                    }
                }
                crossterm::event::KeyCode::Enter => {
                    if !d.links.is_empty() {
                        let url = d.links[d.selected_link_idx].url.clone();
                        drop(d);
                        self.navigate(url);
                        return true;
                    }
                }
                _ => {}
            },
            Mode::Command => match key.code {
                crossterm::event::KeyCode::Enter => {
                    let cmd = d.command_buffer.clone();
                    d.mode = Mode::Normal;
                    drop(d);
                    if cmd == "q" {
                        return false;
                    } else if cmd.starts_with("url ") {
                        self.navigate(cmd[4..].to_string());
                    } else if cmd == "back" || cmd == "b" {
                        self.webview.go_back(1);
                    } else if cmd == "forward" || cmd == "f" {
                        self.webview.go_forward(1);
                    } else if cmd == "reload" || cmd == "r" {
                        let url = self.data.borrow().current_url.clone();
                        self.navigate(url);
                    }
                    return true;
                }
                crossterm::event::KeyCode::Esc => {
                    d.mode = Mode::Normal;
                }
                crossterm::event::KeyCode::Char(c) => {
                    d.command_buffer.push(c);
                }
                crossterm::event::KeyCode::Backspace => {
                    d.command_buffer.pop();
                }
                _ => {}
            },
        }
        true
    }
}

fn parse_css_color(color_str: &str) -> Option<Color> {
    let c = color_str.trim();
    if c.starts_with("rgb") {
        let inner = c
            .trim_start_matches("rgba(")
            .trim_start_matches("rgb(")
            .trim_end_matches(')');
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() >= 3 {
            let r = parts[0].trim().parse::<u8>().ok()?;
            let g = parts[1].trim().parse::<u8>().ok()?;
            let b = parts[2].trim().parse::<u8>().ok()?;
            return Some(Color::Rgb(r, g, b));
        }
    }
    if c.starts_with('#') && c.len() == 7 {
        let r = u8::from_str_radix(&c[1..3], 16).ok()?;
        let g = u8::from_str_radix(&c[3..5], 16).ok()?;
        let b = u8::from_str_radix(&c[5..7], 16).ok()?;
        return Some(Color::Rgb(r, g, b));
    }
    None
}

fn convert_content(
    content: &PageContent,
    links: &mut Vec<LinkData>,
) -> Vec<Line<'static>> {
    links.clear();
    for link in &content.links {
        links.push(LinkData {
            url: link.url.clone(),
            link_type: LinkType::Web,
        });
    }
    let mut lines = Vec::new();
    for js_line in &content.lines {
        let mut spans: Vec<Span<'static>> = Vec::new();
        for seg in js_line {
            let mut style = Style::default();
            if seg.bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            if seg.italic {
                style = style.add_modifier(Modifier::ITALIC);
            }
            if seg.underline {
                style = style.add_modifier(Modifier::UNDERLINED);
            }
            if let Some(ref c) = seg.color {
                if let Some(color) = parse_css_color(c) {
                    style = style.fg(color);
                }
            }
            if seg.link_idx >= 0 {
                spans.push(Span::styled(
                    format!("[{}]", seg.link_idx),
                    Style::default().fg(Color::DarkGray),
                ));
                style = style
                    .fg(LINK_COLOR_WEB)
                    .add_modifier(Modifier::UNDERLINED);
            }
            spans.push(Span::styled(seg.text.clone(), style));
        }
        lines.push(Line::from(spans));
    }
    lines
}

pub fn render_content(data: &DisplayData) -> Vec<Line<'static>> {
    let mut rendered = Vec::new();
    let mut current_idx = 0;
    for line in &data.content_lines {
        let mut spans = Vec::new();
        for span in &line.spans {
            let mut s = span.clone();
            let is_link = s.style.fg == Some(LINK_COLOR_WEB)
                || s.style.fg == Some(LINK_COLOR_IMG);
            let is_selected = is_link && current_idx == data.selected_link_idx;
            if is_selected {
                s.style = s
                    .style
                    .bg(Color::Yellow)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD);
            }
            if is_link {
                current_idx += 1;
            }
            spans.push(s);
        }
        rendered.push(Line::from(spans));
    }
    rendered
}
