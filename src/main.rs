//! A Minecraft skin editor with a Material 3 interface.

// No console window behind the app on Windows.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod default_skin;
mod material;
mod skin;

use std::cell::RefCell;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use iced::alignment::Horizontal;
use iced::border::Radius;
use iced::widget::canvas::{self, event::Status};
use iced::widget::image::Handle;
use iced::widget::{
    button, center, checkbox, column, container, horizontal_rule, horizontal_space, mouse_area, opaque, row,
    scrollable, stack, text_input, toggler, tooltip, vertical_space, Space,
};
use iced::{
    keyboard, mouse, time, window, Alignment, Color, Element, Length, Point, Rectangle, Size, Subscription, Task,
    Theme,
};

use material::{self as m3, Accent, Emphasis, Icon, Scheme, Segment};
use skin::{Camera, Layer, Model, Scene, Skin, Texel, Visibility, PART_NAMES};

#[cfg(target_os = "linux")]
const APP_ID: &str = "io.github.SkinEditor";

fn main() -> iced::Result {
    let files: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();

    #[allow(unused_mut)]
    let mut window = window::Settings {
        size: Size::new(1220.0, 800.0),
        min_size: Some(Size::new(860.0, 560.0)),
        ..Default::default()
    };
    #[cfg(target_os = "linux")]
    {
        window.platform_specific.application_id = APP_ID.to_string();
    }

    iced::application("Skin Editor", SkinEditor::update, SkinEditor::view)
        .subscription(SkinEditor::subscription)
        .theme(SkinEditor::theme)
        .window(window)
        .font(m3::ICONS)
        .font(m3::ICONS_FILLED)
        .antialiasing(true)
        .run_with(move || SkinEditor::new(files.clone()))
}

const UNDO_LIMIT: usize = 200;
const SNACKBAR_TIME: Duration = Duration::from_secs(4);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tool {
    Pencil,
    Eraser,
    Fill,
    Picker,
}

/// Pointer activity in the 3D viewport, in viewport coordinates.
#[derive(Clone, Copy, Debug)]
enum ViewEvent {
    StrokeStart(Point, Size),
    StrokeMove(Point, Size),
    StrokeEnd,
    Rotate(f32, f32),
    /// Pointer delta, and the viewport height it is relative to.
    Pan(f32, f32, f32),
    Zoom(f32),
}

/// Identifies a tab across asynchronous work such as a save dialog.
type DocId = u64;

#[derive(Clone, Debug)]
enum Message {
    New,
    Open,
    Opened(Vec<PathBuf>),
    Save,
    SaveAs,
    SaveTo(DocId, PathBuf),
    Nothing,
    Tick,
    DismissSnackbar,
    TabActivate(DocId),
    TabClose(DocId),
    CloseActive,
    CloseConfirmed,
    CloseCancelled,
    SetTool(Tool),
    Slot(usize),
    PickSv(f32, f32),
    PickHue(f32),
    HexInput(String),
    /// A typed value for the red (0), green (1) or blue (2) channel.
    RgbInput(usize, String),
    SetModel(Model),
    Visible(Layer, usize, bool),
    LayerVisible(Layer, bool),
    /// Asks before clearing; the two below answer the question.
    ClearOverlay,
    ClearOverlayConfirmed,
    ClearOverlayCancelled,
    Grid(bool),
    Dark(bool),
    SetAccent(Accent),
    OpenOptions,
    CloseOptions,
    Escape,
    ResetView,
    Undo,
    Redo,
    View(ViewEvent),
}

// --------------------------------------------------------------- document ---

/// One open skin: a tab.
struct Document {
    id: DocId,
    name: String,
    path: Option<PathBuf>,
    skin: Skin,
    /// The texture as it is on disk, to tell whether there are unsaved edits.
    saved: Skin,
    model: Model,
    boxes: Vec<skin::BoxDef>,
    vis: Visibility,
    cam: Camera,
    undo: Vec<Skin>,
    redo: Vec<Skin>,
    last_point: Option<Point>,
    /// Bumped whenever the preview has to be redrawn.
    revision: u64,
    render: RefCell<Option<((u64, usize, usize, bool), Handle)>>,
}

impl Document {
    fn new(id: DocId, name: String, path: Option<PathBuf>, skin: Skin, model: Model) -> Self {
        Self {
            id,
            name,
            path,
            saved: skin.clone(),
            skin,
            model,
            boxes: skin::boxes(model),
            vis: Visibility::default(),
            cam: Camera::default(),
            undo: Vec::new(),
            redo: Vec::new(),
            last_point: None,
            revision: 0,
            render: RefCell::new(None),
        }
    }

    fn dirty(&self) -> bool {
        self.skin != self.saved
    }

    fn touch(&mut self) {
        self.revision += 1;
    }

    fn scene(&self) -> Scene<'_> {
        Scene {
            skin: &self.skin,
            boxes: &self.boxes,
            vis: &self.vis,
        }
    }

    /// The texel a tool would act on at a viewport position.
    fn pick(&self, pos: Point, size: Size, tool: Tool) -> Option<Texel> {
        let ray = self.cam.ray(pos.x, pos.y, size.width, size.height);
        match tool {
            Tool::Picker => self.scene().pick_visible(&ray),
            _ => self.scene().pick_surface(&ray),
        }
    }

    fn checkpoint(&mut self) {
        self.undo.push(self.skin.clone());
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    fn undo(&mut self) {
        if let Some(previous) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.skin, previous));
            self.touch();
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.skin, next));
            self.touch();
        }
    }

    /// The preview image for a viewport of the given size, rendered on demand.
    fn rendered(&self, size: Size, grid: bool) -> Handle {
        // Supersample for smooth edges, within a fixed pixel budget.
        let scale = (3_000_000.0 / (size.width * size.height)).sqrt().min(2.0);
        let w = (size.width * scale).round() as usize;
        let h = (size.height * scale).round() as usize;
        let key = (self.revision, w, h, grid);

        let mut cache = self.render.borrow_mut();
        match &*cache {
            Some((cached, handle)) if *cached == key => handle.clone(),
            _ => {
                let pixels = self.scene().render(&self.cam, w, h, grid);
                let handle = Handle::from_rgba(w as u32, h as u32, pixels);
                *cache = Some((key, handle.clone()));
                handle
            }
        }
    }
}

// ----------------------------------------------------------------- colour ---

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> [u8; 3] {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [r, g, b].map(|f| ((f + m) * 255.0).round() as u8)
}

fn rgb_to_hsv([r, g, b]: [u8; 3]) -> (f32, f32, f32) {
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let max = r.max(g).max(b);
    let delta = max - r.min(g).min(b);
    let h = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    (h, if max == 0.0 { 0.0 } else { delta / max }, max)
}

fn hex([r, g, b]: [u8; 3]) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}

fn parse_hex(text: &str) -> Option<[u8; 3]> {
    let digits = text.trim().trim_start_matches('#');
    if digits.len() != 6 || !digits.is_ascii() {
        return None;
    }
    let n = u32::from_str_radix(digits, 16).ok()?;
    Some([(n >> 16) as u8, (n >> 8) as u8, n as u8])
}

fn color([r, g, b]: [u8; 3]) -> Color {
    Color::from_rgb8(r, g, b)
}

/// Whether the desktop is in dark mode; light when it cannot tell.
fn system_prefers_dark() -> bool {
    matches!(dark_light::detect(), Ok(dark_light::Mode::Dark))
}

// --------------------------------------------------------------- settings ---

/// Options kept between runs, in a small `key=value` file.
struct Settings {
    accent: Accent,
    dark: bool,
}

impl Settings {
    fn path() -> Option<PathBuf> {
        dirs::config_dir().map(|dir| dir.join("skin-editor").join("settings"))
    }

    /// The saved options; the theme falls back to the desktop's preference.
    fn load() -> Self {
        let mut settings = Settings {
            accent: Accent::Purple,
            dark: system_prefers_dark(),
        };
        let text = Self::path().and_then(|path| std::fs::read_to_string(path).ok());
        for line in text.as_deref().unwrap_or("").lines() {
            match line.split_once('=').map(|(k, v)| (k.trim(), v.trim())) {
                Some(("accent", value)) => {
                    if let Some(accent) = Accent::from_name(value) {
                        settings.accent = accent;
                    }
                }
                Some(("theme", "dark")) => settings.dark = true,
                Some(("theme", "light")) => settings.dark = false,
                _ => {}
            }
        }
        settings
    }

    fn save(&self) -> Result<(), String> {
        let path = Self::path().ok_or("no configuration folder")?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let theme = if self.dark { "dark" } else { "light" };
        let text = format!("accent={}\ntheme={theme}\n", self.accent.name().to_lowercase());
        std::fs::write(&path, text).map_err(|e| e.to_string())
    }
}

// ------------------------------------------------------------ application ---

struct SkinEditor {
    docs: Vec<Document>,
    active: usize,
    next_id: DocId,
    untitled: usize,
    tool: Tool,
    palette: [[u8; 3]; 8],
    slot: usize,
    /// Picker position; kept apart from the palette so hue survives grey.
    hsv: (f32, f32, f32),
    hex_input: String,
    rgb_input: [String; 3],
    hue_strip: Handle,
    sv_square: RefCell<Option<(u32, Handle)>>,
    grid: bool,
    dark: bool,
    accent: Accent,
    /// The scheme for `accent` and `dark`, and each accent's primary colour
    /// for the options dialog; generating them is not free, so they are
    /// rebuilt only when an option changes.
    scheme: Scheme,
    chips: [Color; 5],
    options_open: bool,
    snackbar: Option<(String, Instant)>,
    confirm_close: Option<DocId>,
    /// The tab whose overlay the user has been asked about clearing.
    confirm_clear: Option<DocId>,
}

impl SkinEditor {
    fn new(files: Vec<PathBuf>) -> (Self, Task<Message>) {
        let palette = [
            [0x1B, 0x1B, 0x1B],
            [0xF5, 0xF5, 0xF5],
            [0xE5, 0x39, 0x35],
            [0xFB, 0x8C, 0x00],
            [0xFD, 0xD8, 0x35],
            [0x43, 0xA0, 0x47],
            [0x1E, 0x88, 0xE5],
            [0xE0, 0xAC, 0x88],
        ];
        let hue_strip = {
            let mut pixels = Vec::with_capacity(360 * 4);
            for h in 0..360 {
                let [r, g, b] = hsv_to_rgb(h as f32, 1.0, 1.0);
                pixels.extend_from_slice(&[r, g, b, 255]);
            }
            Handle::from_rgba(360, 1, pixels)
        };
        let slot = 2;
        let settings = Settings::load();
        let mut app = Self {
            docs: Vec::new(),
            active: 0,
            next_id: 0,
            untitled: 0,
            tool: Tool::Pencil,
            palette,
            slot,
            hsv: rgb_to_hsv(palette[slot]),
            hex_input: hex(palette[slot]),
            rgb_input: palette[slot].map(|c| c.to_string()),
            hue_strip,
            sv_square: RefCell::new(None),
            grid: true,
            dark: settings.dark,
            accent: settings.accent,
            scheme: Scheme::new(settings.accent, settings.dark),
            chips: [Color::BLACK; 5],
            options_open: false,
            snackbar: None,
            confirm_close: None,
            confirm_clear: None,
        };
        app.refresh_scheme();
        app.open_files(files);
        if app.docs.is_empty() {
            app.new_document();
        }
        (app, Task::none())
    }

    fn scheme(&self) -> Scheme {
        self.scheme
    }

    fn theme(&self) -> Theme {
        self.scheme.theme()
    }

    fn refresh_scheme(&mut self) {
        self.scheme = Scheme::new(self.accent, self.dark);
        self.chips = Accent::ALL.map(|accent| Scheme::new(accent, self.dark).primary);
    }

    /// Applies a changed option and remembers it for next time.
    fn options_changed(&mut self) {
        self.refresh_scheme();
        let settings = Settings {
            accent: self.accent,
            dark: self.dark,
        };
        if let Err(err) = settings.save() {
            self.notify(format!("Could not save options: {err}"));
        }
    }

    fn notify(&mut self, text: String) {
        self.snackbar = Some((text, Instant::now()));
    }

    fn index_of(&self, id: DocId) -> Option<usize> {
        self.docs.iter().position(|doc| doc.id == id)
    }

    fn active(&self) -> Option<&Document> {
        self.docs.get(self.active)
    }

    fn active_mut(&mut self) -> Option<&mut Document> {
        self.docs.get_mut(self.active)
    }

    fn add_document(&mut self, name: String, path: Option<PathBuf>, skin: Skin, model: Model) {
        let id = self.next_id;
        self.next_id += 1;
        self.docs.push(Document::new(id, name, path, skin, model));
        self.active = self.docs.len() - 1;
    }

    fn new_document(&mut self) {
        self.untitled += 1;
        let name = if self.untitled == 1 {
            "Untitled".to_string()
        } else {
            format!("Untitled {}", self.untitled)
        };
        let (skin, model) = Skin::default_character();
        self.add_document(name, None, skin, model);
    }

    fn open_files(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            if let Some(index) = self.docs.iter().position(|doc| doc.path.as_deref() == Some(path.as_path())) {
                self.active = index;
                continue;
            }
            match Skin::load(&path) {
                Ok((skin, model)) => {
                    let name = file_name(&path);
                    self.notify(format!("Opened {name}"));
                    self.add_document(name, Some(path), skin, model);
                }
                Err(err) => self.notify(format!("Could not open {}: {err}", path.display())),
            }
        }
    }

    fn close(&mut self, id: DocId) {
        let Some(index) = self.index_of(id) else {
            return;
        };
        self.docs.remove(index);
        // Keep the same tab selected; closing the active one selects the
        // tab that moved into its place (or the new last one).
        if index < self.active {
            self.active -= 1;
        }
        self.active = self.active.min(self.docs.len().saturating_sub(1));
    }

    fn save_to(&mut self, id: DocId, path: PathBuf) {
        let Some(index) = self.index_of(id) else {
            return;
        };
        let doc = &mut self.docs[index];
        let result = doc.skin.save(&path);
        match result {
            Ok(()) => {
                doc.saved = doc.skin.clone();
                doc.name = file_name(&path);
                doc.path = Some(path.clone());
                self.notify(format!("Saved {}", path.display()));
            }
            Err(err) => self.notify(format!("Could not save {}: {err}", path.display())),
        }
    }

    fn save_dialog(&self) -> Task<Message> {
        let Some(doc) = self.active() else {
            return Task::none();
        };
        let id = doc.id;
        let mut suggested = doc.name.clone();
        if !suggested.to_lowercase().ends_with(".png") {
            suggested.push_str(".png");
        }
        let directory = doc
            .path
            .as_ref()
            .and_then(|p| p.parent().map(PathBuf::from))
            .or_else(dirs::download_dir);

        Task::perform(
            async move {
                let mut dialog = rfd::AsyncFileDialog::new()
                    .set_title("Save skin")
                    .set_file_name(suggested)
                    .add_filter("PNG image", &["png"]);
                if let Some(directory) = directory {
                    dialog = dialog.set_directory(directory);
                }
                dialog.save_file().await.map(|file| file.path().to_path_buf())
            },
            move |picked| match picked {
                Some(mut path) => {
                    if path.extension().is_none() {
                        path.set_extension("png");
                    }
                    Message::SaveTo(id, path)
                }
                None => Message::Nothing,
            },
        )
    }

    /// Rewrites the hex and RGB fields from the selected slot's colour.
    fn sync_inputs(&mut self) {
        let rgb = self.palette[self.slot];
        self.hex_input = hex(rgb);
        self.rgb_input = rgb.map(|c| c.to_string());
    }

    fn set_color(&mut self, rgb: [u8; 3]) {
        self.palette[self.slot] = rgb;
        self.hsv = rgb_to_hsv(rgb);
        self.sync_inputs();
    }

    /// Applies the current tool at a viewport position.
    fn apply_tool(&mut self, pos: Point, size: Size) {
        let tool = self.tool;
        let [r, g, b] = self.palette[self.slot];
        let Some(doc) = self.active_mut() else {
            return;
        };
        let Some(texel) = doc.pick(pos, size, tool) else {
            return;
        };
        match tool {
            Tool::Pencil => doc.skin.set(texel.x, texel.y, [r, g, b, 255]),
            Tool::Eraser => doc.skin.set(texel.x, texel.y, [0; 4]),
            Tool::Fill => doc.skin.flood_fill(texel.x, texel.y, texel.face, [r, g, b, 255]),
            Tool::Picker => {
                let [r, g, b, _] = doc.skin.get(texel.x, texel.y);
                self.set_color([r, g, b]);
                self.tool = Tool::Pencil;
                return;
            }
        }
        doc.touch();
    }

    fn view_event(&mut self, event: ViewEvent) {
        let tool = self.tool;
        match event {
            ViewEvent::StrokeStart(pos, size) => {
                if tool != Tool::Picker {
                    if let Some(doc) = self.active_mut() {
                        doc.checkpoint();
                        doc.last_point = Some(pos);
                    }
                }
                self.apply_tool(pos, size);
            }
            ViewEvent::StrokeMove(pos, size) => {
                if !matches!(tool, Tool::Pencil | Tool::Eraser) {
                    return;
                }
                let Some(from) = self.active().and_then(|doc| doc.last_point) else {
                    return;
                };
                // Walk the segment so fast strokes leave no gaps.
                let length = from.distance(pos);
                let steps = ((length / 1.5).ceil() as usize).clamp(1, 600);
                for i in 1..=steps {
                    let t = i as f32 / steps as f32;
                    let p = Point::new(from.x + (pos.x - from.x) * t, from.y + (pos.y - from.y) * t);
                    self.apply_tool(p, size);
                }
                if let Some(doc) = self.active_mut() {
                    doc.last_point = Some(pos);
                }
            }
            ViewEvent::StrokeEnd => {
                if let Some(doc) = self.active_mut() {
                    doc.last_point = None;
                    // A stroke that changed nothing is not worth an undo step.
                    if doc.undo.last() == Some(&doc.skin) {
                        doc.undo.pop();
                    }
                }
            }
            ViewEvent::Rotate(dx, dy) => {
                if let Some(doc) = self.active_mut() {
                    doc.cam.rotate(dx, dy);
                    doc.touch();
                }
            }
            ViewEvent::Pan(dx, dy, height) => {
                if let Some(doc) = self.active_mut() {
                    doc.cam.pan(dx, dy, height);
                    doc.touch();
                }
            }
            ViewEvent::Zoom(steps) => {
                if let Some(doc) = self.active_mut() {
                    doc.cam.zoom(steps);
                    doc.touch();
                }
            }
        }
    }

    fn sv_handle(&self) -> Handle {
        let key = self.hsv.0.round() as u32;
        let mut cache = self.sv_square.borrow_mut();
        match &*cache {
            Some((cached, handle)) if *cached == key => handle.clone(),
            _ => {
                const N: usize = 96;
                let mut pixels = Vec::with_capacity(N * N * 4);
                for y in 0..N {
                    for x in 0..N {
                        let s = x as f32 / (N - 1) as f32;
                        let v = 1.0 - y as f32 / (N - 1) as f32;
                        let [r, g, b] = hsv_to_rgb(key as f32, s, v);
                        pixels.extend_from_slice(&[r, g, b, 255]);
                    }
                }
                let handle = Handle::from_rgba(N as u32, N as u32, pixels);
                *cache = Some((key, handle.clone()));
                handle
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let keys = keyboard::on_key_press(shortcut);
        if self.snackbar.is_some() {
            Subscription::batch([keys, time::every(Duration::from_millis(250)).map(|_| Message::Tick)])
        } else {
            keys
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Nothing => {}
            Message::Tick => {
                if self.snackbar.as_ref().is_some_and(|(_, since)| since.elapsed() >= SNACKBAR_TIME) {
                    self.snackbar = None;
                }
            }
            Message::DismissSnackbar => self.snackbar = None,
            Message::New => self.new_document(),
            Message::Open => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Open skins")
                            .add_filter("Minecraft skins (PNG)", &["png"])
                            .pick_files()
                            .await
                            .map(|files| files.iter().map(|f| f.path().to_path_buf()).collect::<Vec<_>>())
                    },
                    |picked| match picked {
                        Some(paths) => Message::Opened(paths),
                        None => Message::Nothing,
                    },
                );
            }
            Message::Opened(paths) => self.open_files(paths),
            Message::Save => match self.active().map(|doc| (doc.id, doc.path.clone())) {
                Some((id, Some(path))) => self.save_to(id, path),
                Some((_, None)) => return self.save_dialog(),
                None => {}
            },
            Message::SaveAs => return self.save_dialog(),
            Message::SaveTo(id, path) => self.save_to(id, path),
            Message::TabActivate(id) => {
                if let Some(index) = self.index_of(id) {
                    self.active = index;
                }
            }
            Message::TabClose(id) => {
                let Some(index) = self.index_of(id) else {
                    return Task::none();
                };
                if self.docs[index].dirty() {
                    self.active = index;
                    self.confirm_close = Some(id);
                } else {
                    self.close(id);
                }
            }
            Message::CloseActive => {
                if let Some(id) = self.active().map(|doc| doc.id) {
                    return self.update(Message::TabClose(id));
                }
            }
            Message::CloseConfirmed => {
                if let Some(id) = self.confirm_close.take() {
                    self.close(id);
                }
            }
            Message::CloseCancelled => self.confirm_close = None,
            Message::SetTool(tool) => self.tool = tool,
            Message::Slot(slot) => {
                self.slot = slot;
                self.set_color(self.palette[slot]);
                if self.tool == Tool::Eraser {
                    self.tool = Tool::Pencil;
                }
            }
            Message::PickSv(s, v) => {
                self.hsv = (self.hsv.0, s, v);
                self.palette[self.slot] = hsv_to_rgb(self.hsv.0, s, v);
                self.sync_inputs();
            }
            Message::PickHue(h) => {
                self.hsv.0 = h;
                self.palette[self.slot] = hsv_to_rgb(h, self.hsv.1, self.hsv.2);
                self.sync_inputs();
            }
            Message::HexInput(text) => {
                if let Some(rgb) = parse_hex(&text) {
                    self.palette[self.slot] = rgb;
                    self.hsv = rgb_to_hsv(rgb);
                    self.rgb_input = rgb.map(|c| c.to_string());
                }
                self.hex_input = text;
            }
            Message::RgbInput(channel, text) => {
                // Digits only, at most three of them; the colour changes
                // once the field holds a value from 0 to 255.
                if text.len() <= 3 && text.chars().all(|c| c.is_ascii_digit()) {
                    if let Ok(value) = text.parse::<u8>() {
                        let mut rgb = self.palette[self.slot];
                        rgb[channel] = value;
                        self.palette[self.slot] = rgb;
                        self.hsv = rgb_to_hsv(rgb);
                        self.hex_input = hex(rgb);
                    }
                    self.rgb_input[channel] = text;
                }
            }
            Message::SetModel(model) => {
                if let Some(doc) = self.active_mut() {
                    doc.model = model;
                    doc.boxes = skin::boxes(model);
                    doc.touch();
                }
            }
            Message::Visible(layer, part, shown) => {
                if let Some(doc) = self.active_mut() {
                    doc.vis.layer_mut(layer)[part] = shown;
                    doc.touch();
                }
            }
            Message::LayerVisible(layer, shown) => {
                if let Some(doc) = self.active_mut() {
                    *doc.vis.layer_mut(layer) = [shown; 6];
                    doc.touch();
                }
            }
            Message::ClearOverlay => self.confirm_clear = self.active().map(|doc| doc.id),
            Message::ClearOverlayCancelled => self.confirm_clear = None,
            Message::ClearOverlayConfirmed => {
                let index = self.confirm_clear.take().and_then(|id| self.index_of(id));
                if let Some(doc) = index.map(|i| &mut self.docs[i]) {
                    doc.checkpoint();
                    doc.skin.clear_overlay();
                    if doc.undo.last() == Some(&doc.skin) {
                        doc.undo.pop();
                    }
                    doc.touch();
                }
            }
            Message::Grid(grid) => self.grid = grid,
            Message::Dark(dark) => {
                self.dark = dark;
                self.options_changed();
            }
            Message::SetAccent(accent) => {
                self.accent = accent;
                self.options_changed();
            }
            Message::OpenOptions => self.options_open = true,
            Message::CloseOptions => self.options_open = false,
            Message::Escape => {
                if self.options_open {
                    self.options_open = false;
                } else {
                    self.confirm_close = None;
                    self.confirm_clear = None;
                }
            }
            Message::ResetView => {
                if let Some(doc) = self.active_mut() {
                    doc.cam = Camera::default();
                    doc.touch();
                }
            }
            Message::Undo => {
                if let Some(doc) = self.active_mut() {
                    doc.undo();
                }
            }
            Message::Redo => {
                if let Some(doc) = self.active_mut() {
                    doc.redo();
                }
            }
            Message::View(event) => self.view_event(event),
        }
        Task::none()
    }

    // ------------------------------------------------------------------ view ---

    fn view(&self) -> Element<'_, Message> {
        let s = self.scheme();

        let content: Element<'_, Message> = match self.active() {
            Some(doc) => column![self.top_app_bar(s), self.tab_bar(s), self.workspace(s, doc)].into(),
            None => column![self.top_app_bar(s), self.empty_state(s)].into(),
        };
        let mut layers = stack![container(content).width(Length::Fill).height(Length::Fill).style(m3::surface(s))];

        if let Some((message, _)) = &self.snackbar {
            let bar = container(
                row![
                    m3::body_medium(message.as_str()).width(Length::Fill),
                    button(m3::label_large("Dismiss"))
                        .padding([8, 12])
                        .style(move |theme, status| {
                            let mut style = m3::text_button(s)(theme, status);
                            style.text_color = s.inverse_primary;
                            style
                        })
                        .on_press(Message::DismissSnackbar),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding(iced::Padding {
                left: 16.0,
                ..iced::Padding::new(4.0)
            })
            .max_width(560)
            .style(m3::snackbar(s));
            layers = layers.push(
                container(bar)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Bottom)
                    .padding(48),
            );
        }

        if self.options_open {
            layers = layers.push(opaque(
                mouse_area(center(self.options_dialog(s)).style(m3::scrim(s))).on_press(Message::CloseOptions),
            ));
        }

        let confirmation = self
            .close_dialog(s)
            .map(|dialog| (dialog, Message::CloseCancelled))
            .or_else(|| self.clear_dialog(s).map(|dialog| (dialog, Message::ClearOverlayCancelled)));
        if let Some((dialog, cancel)) = confirmation {
            layers = layers.push(opaque(mouse_area(center(dialog).style(m3::scrim(s))).on_press(cancel)));
        }
        layers.into()
    }

    fn top_app_bar(&self, s: Scheme) -> Element<'_, Message> {
        let has_doc = self.active().is_some();
        let action = |icon: Icon, label: &'static str, style: Emphasis, message: Option<Message>| {
            button(icon_label(icon, label))
                .padding(iced::Padding {
                    left: 16.0,
                    right: 24.0,
                    top: 10.0,
                    bottom: 10.0,
                })
                .style(m3::emphasis(s, style))
                .on_press_maybe(message)
        };
        container(
            row![
                m3::title_large("Skin Editor").color(s.on_surface),
                horizontal_space(),
                action(Icon::Add, "New", Emphasis::Text, Some(Message::New)),
                action(Icon::FolderOpen, "Open", Emphasis::Text, Some(Message::Open)),
                action(Icon::SaveAs, "Save as…", Emphasis::Outlined, has_doc.then_some(Message::SaveAs)),
                action(Icon::Save, "Save", Emphasis::Filled, has_doc.then_some(Message::Save)),
                tooltip(
                    button(m3::icon(Icon::Settings, 24.0))
                        .padding(8)
                        .style(m3::icon_button(s))
                        .on_press(Message::OpenOptions),
                    container(m3::body_small("Options")).padding([4, 8]).style(m3::tooltip(s)),
                    tooltip::Position::Bottom,
                )
                .gap(4),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding(iced::Padding {
            left: 24.0,
            right: 16.0,
            top: 12.0,
            bottom: 8.0,
        })
        .into()
    }

    fn tab_bar(&self, s: Scheme) -> Element<'_, Message> {
        let mut tabs = row![].spacing(0).align_y(Alignment::End);
        for (index, doc) in self.docs.iter().enumerate() {
            let active = index == self.active;
            let label = if doc.dirty() { format!("● {}", doc.name) } else { doc.name.clone() };
            let close = button(m3::icon(Icon::Close, 18.0))
                .padding(4)
                .style(m3::icon_button(s))
                .on_press(Message::TabClose(doc.id));
            let head = button(
                row![m3::label_large(label), close]
                    .spacing(8)
                    .align_y(Alignment::Center),
            )
            .padding(iced::Padding {
                left: 20.0,
                right: 8.0,
                top: 10.0,
                bottom: 10.0,
            })
            .style(m3::tab(s, active))
            .on_press(Message::TabActivate(doc.id));
            // The active indicator: a 3 px bar with rounded top corners,
            // as wide as the tab's content.
            let indicator: Element<'_, Message> = if active {
                container(
                    container(Space::new(Length::Fill, 3)).style(m3::fill(
                        s.primary,
                        Radius {
                            top_left: 3.0,
                            top_right: 3.0,
                            bottom_left: 0.0,
                            bottom_right: 0.0,
                        },
                    )),
                )
                .padding([0, 12])
                .into()
            } else {
                Space::new(Length::Fill, 3).into()
            };
            tabs = tabs.push(column![head, indicator].width(Length::Shrink));
        }
        column![
            scrollable(container(tabs).padding([0, 12]))
                .direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::new().width(4).scroller_width(4)
                ))
                .style(m3::scroll(s)),
            horizontal_rule(1).style(m3::divider(s)),
        ]
        .into()
    }

    fn empty_state(&self, s: Scheme) -> Element<'_, Message> {
        let content = column![
            m3::icon(Icon::Brush, 48.0).color(s.primary),
            vertical_space().height(8),
            m3::headline_small("No skin open").color(s.on_surface),
            m3::body_medium("Open a skin from your computer or start a new one.").color(s.on_surface_variant),
            vertical_space().height(8),
            row![
                button(icon_label(Icon::Add, "New")).padding([10, 24]).style(m3::tonal(s)).on_press(Message::New),
                button(icon_label(Icon::FolderOpen, "Open")).padding([10, 24]).style(m3::filled(s)).on_press(Message::Open),
            ]
            .spacing(8),
        ]
        .spacing(8)
        .align_x(Alignment::Center);
        center(content).into()
    }

    fn workspace<'a>(&'a self, s: Scheme, doc: &'a Document) -> Element<'a, Message> {
        let heading = |label: &'static str| m3::title_small(label).color(s.on_surface_variant);

        // --- left: tools, palette, history
        let tool = |icon: Icon, label: &'static str, tool: Tool| {
            let active = self.tool == tool;
            let (icon, label) = if active {
                (m3::icon_filled(icon, 20.0), m3::label_large(label))
            } else {
                (m3::icon(icon, 20.0), m3::body_medium(label))
            };
            button(row![icon, label.width(Length::Fill)].spacing(12).align_y(Alignment::Center))
                .padding([12, 16])
                .width(Length::Fill)
                .style(m3::nav_item(s, active))
                .on_press(Message::SetTool(tool))
        };
        let history = |icon: Icon, label: &'static str, message: Option<Message>| {
            button(container(icon_label(icon, label)).center_x(Length::Fill))
                .padding([10, 8])
                .width(Length::Fill)
                .style(m3::outlined(s))
                .on_press_maybe(message)
        };
        let left = column![
            heading("Tools"),
            column![
                tool(Icon::Edit, "Pencil", Tool::Pencil),
                tool(Icon::Eraser, "Eraser", Tool::Eraser),
                tool(Icon::Fill, "Fill", Tool::Fill),
                tool(Icon::Colorize, "Pick colour", Tool::Picker),
            ]
            .spacing(2),
            horizontal_rule(1).style(m3::divider(s)),
            heading("Colours"),
            canvas(Palette {
                colors: self.palette,
                selected: self.slot,
                scheme: s,
            })
            .width(Length::Fixed(Palette::WIDTH))
            .height(Length::Fixed(Palette::HEIGHT)),
            horizontal_rule(1).style(m3::divider(s)),
            heading("History"),
            row![
                history(Icon::Undo, "Undo", (!doc.undo.is_empty()).then_some(Message::Undo)),
                history(Icon::Redo, "Redo", (!doc.redo.is_empty()).then_some(Message::Redo)),
            ]
            .spacing(8),
        ]
        .spacing(12)
        .width(Length::Fixed(Palette::WIDTH));

        // --- centre: the paintable model
        let viewport = container(
            canvas(Viewport {
                doc,
                tool: self.tool,
                grid: self.grid,
            })
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .padding(1)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(m3::stage(s));

        // --- right: colour, model, layers, view
        // Two layers: iced draws images above shapes within one layer, so the
        // handles need a layer of their own to sit on top of the gradients.
        let picker_layer = |layer: PickerLayer| {
            canvas(Picker {
                layer,
                hsv: self.hsv,
                sv: self.sv_handle(),
                hue: self.hue_strip.clone(),
            })
            .width(Length::Fill)
            .height(Length::Fixed(Picker::HEIGHT))
        };
        let picker = stack![picker_layer(PickerLayer::Gradients), picker_layer(PickerLayer::Handles)];

        let hex_field = text_input("#RRGGBB", &self.hex_input)
            .on_input(Message::HexInput)
            .padding([10, 14])
            .size(14)
            .style(m3::text_field(s));
        let channel = |index: usize, label: &'static str| {
            column![
                m3::label_medium(label).color(s.on_surface_variant),
                text_input("0–255", &self.rgb_input[index])
                    .on_input(move |text| Message::RgbInput(index, text))
                    .padding([10, 12])
                    .size(14)
                    .style(m3::text_field(s)),
            ]
            .spacing(4)
            .width(Length::Fill)
        };
        let rgb_fields = row![channel(0, "R"), channel(1, "G"), channel(2, "B")].spacing(8);
        let current = container(Space::new(40, 40)).style(m3::fill(color(self.palette[self.slot]), 20.0.into()));

        let model = segmented(
            s,
            [
                (None, "Classic", doc.model == Model::Classic, Message::SetModel(Model::Classic)),
                (None, "Slim", doc.model == Model::Slim, Message::SetModel(Model::Slim)),
            ],
        );

        const COLUMN: f32 = 64.0;
        let cell = |layer: Layer, part: usize| {
            container(
                checkbox("", doc.vis.get(layer, part))
                    .size(18)
                    .spacing(0)
                    .style(m3::checkbox_style(s))
                    .on_toggle(move |shown| Message::Visible(layer, part, shown)),
            )
            .center_x(Length::Fixed(COLUMN))
        };
        let caption = |label: &'static str| m3::label_medium(label).color(s.on_surface_variant);
        let mut parts = column![row![
            caption("Show").width(Length::Fill),
            container(caption("Base")).center_x(Length::Fixed(COLUMN)),
            container(caption("Overlay")).center_x(Length::Fixed(COLUMN)),
        ]]
        .spacing(6);
        for (part, name) in PART_NAMES.iter().enumerate() {
            parts = parts.push(
                row![
                    m3::body_medium(*name).width(Length::Fill),
                    cell(Layer::Base, part),
                    cell(Layer::Overlay, part),
                ]
                .align_y(Alignment::Center),
            );
        }

        let switch = |label: &'static str, on: bool, message: fn(bool) -> Message| {
            toggler(on)
                .label(label)
                .size(24)
                .text_size(14)
                .spacing(12)
                .style(m3::switch(s))
                .on_toggle(message)
                .width(Length::Fill)
        };
        let layer_on = |layer: Layer| match layer {
            Layer::Base => doc.vis.base.contains(&true),
            Layer::Overlay => doc.vis.overlay.contains(&true),
        };
        let wide = |icon: Icon, label: &'static str, style: Emphasis, message: Message| {
            button(container(icon_label(icon, label)).center_x(Length::Fill))
                .padding([10, 24])
                .width(Length::Fill)
                .style(m3::emphasis(s, style))
                .on_press(message)
        };

        let right = column![
            heading("Colour"),
            picker,
            row![current, hex_field].spacing(12).align_y(Alignment::Center),
            rgb_fields,
            horizontal_rule(1).style(m3::divider(s)),
            heading("Model"),
            model,
            horizontal_rule(1).style(m3::divider(s)),
            heading("Layers"),
            m3::body_small(
                "You paint on the outermost layer that is shown. Hide the overlay to paint the base underneath.",
            )
            .color(s.on_surface_variant),
            switch("Base layer", layer_on(Layer::Base), |on| Message::LayerVisible(Layer::Base, on)),
            switch("Overlay layer", layer_on(Layer::Overlay), |on| Message::LayerVisible(Layer::Overlay, on)),
            parts,
            wide(Icon::LayersClear, "Clear overlay", Emphasis::Outlined, Message::ClearOverlay),
            horizontal_rule(1).style(m3::divider(s)),
            heading("View"),
            switch("Pixel grid", self.grid, Message::Grid),
            wide(Icon::RestartAlt, "Reset view", Emphasis::Tonal, Message::ResetView),
        ]
        .spacing(12);

        let left_card = container(left).padding(16).style(m3::card(s));
        let right_card = scrollable(container(right).padding(16).width(Length::Fixed(300.0)))
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4).margin(2).spacing(2),
            ))
            .style(m3::scroll(s))
            .height(Length::Fill);
        let right_card = container(right_card).height(Length::Fill).style(m3::card(s));

        let hint = "Drag on the model to paint · drag beside it or right-drag to rotate · \
                    hold the wheel and drag to move · scroll to zoom";
        column![
            row![left_card, viewport, right_card].spacing(16).height(Length::Fill),
            m3::body_small(hint).color(s.on_surface_variant),
        ]
        .spacing(10)
        .padding(iced::Padding {
            top: 16.0,
            right: 16.0,
            bottom: 10.0,
            left: 16.0,
        })
        .into()
    }

    fn options_dialog(&self, s: Scheme) -> Element<'_, Message> {
        let heading = |label: &'static str| m3::title_small(label).color(s.on_surface_variant);

        let mut chips = row![].spacing(16);
        for (accent, chip) in Accent::ALL.into_iter().zip(self.chips) {
            let selected = accent == self.accent;
            let mark: Element<'_, Message> = if selected {
                m3::icon(Icon::Check, 24.0).into()
            } else {
                Space::new(24, 24).into()
            };
            chips = chips.push(
                column![
                    button(container(mark).center(48))
                        .padding(0)
                        .style(m3::color_chip(s, chip, selected))
                        .on_press(Message::SetAccent(accent)),
                    m3::body_small(accent.name()).color(if selected { s.on_surface } else { s.on_surface_variant }),
                ]
                .spacing(6)
                .align_x(Alignment::Center),
            );
        }

        let theme = segmented(
            s,
            [
                (Some(Icon::LightMode), "Light", !self.dark, Message::Dark(false)),
                (Some(Icon::DarkMode), "Dark", self.dark, Message::Dark(true)),
            ],
        );

        let card = container(
            column![
                container(m3::icon(Icon::Settings, 24.0).color(s.primary)).center_x(Length::Fill),
                container(m3::headline_small("Options").color(s.on_surface)).center_x(Length::Fill),
                heading("Colour"),
                chips,
                heading("Theme"),
                theme,
                row![
                    horizontal_space(),
                    button(m3::label_large("Done"))
                        .padding([10, 12])
                        .style(m3::text_button(s))
                        .on_press(Message::CloseOptions),
                ],
            ]
            .spacing(16),
        )
        .padding(24)
        .width(Length::Shrink)
        .style(m3::dialog(s));
        // Clicks on the card must not reach the scrim behind it.
        opaque(card)
    }

    fn close_dialog(&self, s: Scheme) -> Option<Element<'_, Message>> {
        let doc = &self.docs[self.index_of(self.confirm_close?)?];
        Some(confirm_dialog(
            s,
            "Close without saving?",
            format!("“{}” has changes that have not been saved. They will be lost.", doc.name),
            "Discard",
            Message::CloseConfirmed,
            Message::CloseCancelled,
        ))
    }

    fn clear_dialog(&self, s: Scheme) -> Option<Element<'_, Message>> {
        let doc = &self.docs[self.index_of(self.confirm_clear?)?];
        Some(confirm_dialog(
            s,
            "Clear overlay?",
            format!(
                "Are you sure you want to clear the overlay? This deletes everything on the overlay \
                 layer of “{}”: hat, jacket, sleeves and trousers.",
                doc.name
            ),
            "Clear overlay",
            Message::ClearOverlayConfirmed,
            Message::ClearOverlayCancelled,
        ))
    }
}

/// A Material dialog that asks before something destructive happens.
fn confirm_dialog<'a>(
    s: Scheme,
    title: &'a str,
    body: String,
    confirm_label: &'a str,
    confirm: Message,
    cancel: Message,
) -> Element<'a, Message> {
    let action = |label: &'a str, style: Emphasis, message: Message| {
        button(m3::label_large(label)).padding([10, 12]).style(m3::emphasis(s, style)).on_press(message)
    };
    let card = container(
        column![
            container(m3::icon(Icon::Warning, 24.0).color(s.error)).center_x(Length::Fill),
            container(m3::headline_small(title).color(s.on_surface)).center_x(Length::Fill),
            m3::body_medium(body).color(s.on_surface_variant),
            row![
                horizontal_space(),
                action("Cancel", Emphasis::Text, cancel),
                action(confirm_label, Emphasis::Danger, confirm),
            ]
            .spacing(8),
        ]
        .spacing(16),
    )
    .padding(24)
    .max_width(420)
    .style(m3::dialog(s));
    // Clicks on the card must not reach the scrim behind it.
    opaque(card)
}

/// A Material segmented button: a pill split into choices, the selected one
/// filled and marked with a check.
fn segmented<'a, const N: usize>(
    s: Scheme,
    choices: [(Option<Icon>, &'a str, bool, Message); N],
) -> Element<'a, Message> {
    let mut segments = row![].height(Length::Fixed(38.0));
    for (i, (icon, label, selected, message)) in choices.into_iter().enumerate() {
        if i > 0 {
            segments = segments.push(
                container(Space::new(1, Length::Fill)).style(m3::fill(s.outline, 0.0.into())),
            );
        }
        let at = match i {
            0 => Segment::First,
            i if i == N - 1 => Segment::Last,
            _ => Segment::Middle,
        };
        // The selected segment swaps its icon (if any) for a check mark.
        let content: Element<'a, Message> = match (selected, icon) {
            (true, _) => icon_label(Icon::Check, label).into(),
            (false, Some(icon)) => icon_label(icon, label).into(),
            (false, None) => m3::label_large(label).into(),
        };
        segments = segments.push(
            button(container(content).center(Length::Fill))
            .padding([0, 12])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(m3::segment(s, selected, at))
            .on_press(message),
        );
    }
    container(segments).padding(1).style(m3::segmented_frame(s)).into()
}

/// A leading icon and a label: the content of a Material button.
fn icon_label<'a>(icon: Icon, label: &'a str) -> iced::widget::Row<'a, Message> {
    row![m3::icon(icon, 18.0), m3::label_large(label)]
        .spacing(8)
        .align_y(Alignment::Center)
}

fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

fn shortcut(key: keyboard::Key, modifiers: keyboard::Modifiers) -> Option<Message> {
    if key == keyboard::Key::Named(keyboard::key::Named::Escape) {
        return Some(Message::Escape);
    }
    if !modifiers.control() {
        return None;
    }
    let keyboard::Key::Character(c) = key else {
        return None;
    };
    match (c.to_lowercase().as_str(), modifiers.shift()) {
        ("z", false) => Some(Message::Undo),
        ("z", true) | ("y", _) => Some(Message::Redo),
        ("s", false) => Some(Message::Save),
        ("s", true) => Some(Message::SaveAs),
        ("o", _) => Some(Message::Open),
        ("n", _) => Some(Message::New),
        ("w", _) => Some(Message::CloseActive),
        _ => None,
    }
}

fn canvas<P: canvas::Program<Message>>(program: P) -> canvas::Canvas<P, Message> {
    iced::widget::canvas(program)
}

// --------------------------------------------------------------- viewport ---

/// The 3D preview. It only turns pointer input into [`ViewEvent`]s; all
/// editing happens in `update`.
struct Viewport<'a> {
    doc: &'a Document,
    tool: Tool,
    grid: bool,
}

#[derive(Clone, Copy, Default)]
enum Drag {
    #[default]
    None,
    Paint,
    Rotate(Point),
    Pan(Point),
}

impl canvas::Program<Message> for Viewport<'_> {
    type State = Drag;

    fn update(
        &self,
        state: &mut Drag,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (Status, Option<Message>) {
        let canvas::Event::Mouse(event) = event else {
            return (Status::Ignored, None);
        };
        let size = bounds.size();
        let local = cursor.position().map(|p| Point::new(p.x - bounds.x, p.y - bounds.y));
        let send = |event| (Status::Captured, Some(Message::View(event)));

        match event {
            mouse::Event::ButtonPressed(button) => {
                let Some(pos) = cursor.position_in(bounds) else {
                    return (Status::Ignored, None);
                };
                let on_model = button == mouse::Button::Left && self.doc.pick(pos, size, self.tool).is_some();
                if button == mouse::Button::Middle {
                    *state = Drag::Pan(pos);
                    (Status::Captured, None)
                } else if on_model {
                    *state = Drag::Paint;
                    send(ViewEvent::StrokeStart(pos, size))
                } else {
                    *state = Drag::Rotate(pos);
                    (Status::Captured, None)
                }
            }
            mouse::Event::CursorMoved { .. } => match (*state, local) {
                (Drag::Paint, Some(pos)) => send(ViewEvent::StrokeMove(pos, size)),
                (Drag::Rotate(last), Some(pos)) => {
                    *state = Drag::Rotate(pos);
                    send(ViewEvent::Rotate(pos.x - last.x, pos.y - last.y))
                }
                (Drag::Pan(last), Some(pos)) => {
                    *state = Drag::Pan(pos);
                    send(ViewEvent::Pan(pos.x - last.x, pos.y - last.y, size.height))
                }
                _ => (Status::Ignored, None),
            },
            mouse::Event::ButtonReleased(_) => match std::mem::take(state) {
                Drag::Paint => send(ViewEvent::StrokeEnd),
                Drag::Rotate(_) | Drag::Pan(_) => (Status::Captured, None),
                Drag::None => (Status::Ignored, None),
            },
            mouse::Event::WheelScrolled { delta } if cursor.is_over(bounds) => {
                let steps = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y,
                    mouse::ScrollDelta::Pixels { y, .. } => y / 40.0,
                };
                send(ViewEvent::Zoom(steps))
            }
            _ => (Status::Ignored, None),
        }
    }

    fn draw(
        &self,
        _state: &Drag,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let size = bounds.size();
        if size.width < 8.0 || size.height < 8.0 {
            return Vec::new();
        }
        let mut frame = canvas::Frame::new(renderer, size);
        frame.draw_image(Rectangle::with_size(size), &self.doc.rendered(size, self.grid));
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(&self, state: &Drag, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        match state {
            Drag::Rotate(_) | Drag::Pan(_) => mouse::Interaction::Grabbing,
            Drag::Paint => mouse::Interaction::Crosshair,
            Drag::None => match cursor.position_in(bounds) {
                Some(pos) if self.doc.pick(pos, bounds.size(), self.tool).is_some() => {
                    mouse::Interaction::Crosshair
                }
                Some(_) => mouse::Interaction::Grab,
                None => mouse::Interaction::default(),
            },
        }
    }
}

// ---------------------------------------------------------------- palette ---

/// The eight colour slots as round swatches, four per row.
struct Palette {
    colors: [[u8; 3]; 8],
    selected: usize,
    scheme: Scheme,
}

impl Palette {
    const CELL: f32 = 38.0;
    const GAP: f32 = 8.0;
    const COLUMNS: usize = 4;
    const WIDTH: f32 = 4.0 * Self::CELL + 3.0 * Self::GAP;
    const HEIGHT: f32 = 2.0 * Self::CELL + Self::GAP;

    fn cell(index: usize) -> Rectangle {
        let pitch = Self::CELL + Self::GAP;
        Rectangle::new(
            Point::new((index % Self::COLUMNS) as f32 * pitch, (index / Self::COLUMNS) as f32 * pitch),
            Size::new(Self::CELL, Self::CELL),
        )
    }
}

impl canvas::Program<Message> for Palette {
    type State = ();

    fn update(
        &self,
        _state: &mut (),
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (Status, Option<Message>) {
        if let canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
            if let Some(pos) = cursor.position_in(bounds) {
                if let Some(slot) = (0..8).find(|i| Self::cell(*i).contains(pos)) {
                    return (Status::Captured, Some(Message::Slot(slot)));
                }
            }
        }
        (Status::Ignored, None)
    }

    fn draw(
        &self,
        _state: &(),
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let s = self.scheme;
        let hovered = cursor
            .position_in(bounds)
            .and_then(|pos| (0..8).find(|i| Self::cell(*i).contains(pos)));
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        for (i, rgb) in self.colors.iter().enumerate() {
            let cell = Self::cell(i);
            let centre = cell.center();
            let outer = Self::CELL / 2.0;
            let selected = i == self.selected;

            if hovered == Some(i) && !selected {
                frame.fill(&canvas::Path::circle(centre, outer), m3::alpha(s.on_surface, 0.08));
            }
            // The selected swatch shrinks to make room for its ring.
            let radius = if selected { outer - 6.0 } else { outer - 3.0 };
            let swatch = canvas::Path::circle(centre, radius);
            frame.fill(&swatch, color(*rgb));
            frame.stroke(
                &swatch,
                canvas::Stroke::default().with_color(s.outline_variant).with_width(1.0),
            );
            if selected {
                frame.stroke(
                    &canvas::Path::circle(centre, outer - 1.5),
                    canvas::Stroke::default().with_color(s.primary).with_width(3.0),
                );
            }
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(&self, _state: &(), bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if cursor.is_over(bounds) {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

// ----------------------------------------------------------------- picker ---

/// Saturation/value square with a hue strip underneath. It is drawn as two
/// stacked canvases, see [`PickerLayer`].
struct Picker {
    layer: PickerLayer,
    hsv: (f32, f32, f32),
    sv: Handle,
    hue: Handle,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PickerLayer {
    /// The colour square and hue strip images.
    Gradients,
    /// Crosshair, knob and hue thumb; this layer also takes the input.
    Handles,
}

#[derive(Clone, Copy, Default)]
enum PickerDrag {
    #[default]
    None,
    Sv,
    Hue,
}

impl Picker {
    const HEIGHT: f32 = 214.0;
    const HUE_HEIGHT: f32 = 16.0;
    const GAP: f32 = 14.0;
    /// Room below the strip so the hue thumb, which is taller than the
    /// strip, is not cut off.
    const MARGIN: f32 = 4.0;

    fn sv_height(bounds: Rectangle) -> f32 {
        bounds.height - Self::MARGIN - Self::HUE_HEIGHT - Self::GAP
    }

    fn hue_top(bounds: Rectangle) -> f32 {
        bounds.height - Self::MARGIN - Self::HUE_HEIGHT
    }

    fn message(drag: PickerDrag, pos: Point, bounds: Rectangle) -> Option<Message> {
        let x = (pos.x / bounds.width).clamp(0.0, 1.0);
        match drag {
            PickerDrag::Sv => {
                let y = (pos.y / Self::sv_height(bounds)).clamp(0.0, 1.0);
                Some(Message::PickSv(x, 1.0 - y))
            }
            PickerDrag::Hue => Some(Message::PickHue(x * 359.0)),
            PickerDrag::None => None,
        }
    }
}

impl canvas::Program<Message> for Picker {
    type State = PickerDrag;

    fn update(
        &self,
        state: &mut PickerDrag,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (Status, Option<Message>) {
        if self.layer != PickerLayer::Handles {
            return (Status::Ignored, None);
        }
        let canvas::Event::Mouse(event) = event else {
            return (Status::Ignored, None);
        };
        match event {
            mouse::Event::ButtonPressed(mouse::Button::Left) => {
                let Some(pos) = cursor.position_in(bounds) else {
                    return (Status::Ignored, None);
                };
                *state = if pos.y <= Self::sv_height(bounds) {
                    PickerDrag::Sv
                } else if pos.y >= Self::hue_top(bounds) - Self::GAP / 2.0 {
                    PickerDrag::Hue
                } else {
                    PickerDrag::None
                };
                (Status::Captured, Self::message(*state, pos, bounds))
            }
            mouse::Event::CursorMoved { .. } if !matches!(state, PickerDrag::None) => {
                let message = cursor
                    .position()
                    .and_then(|p| Self::message(*state, Point::new(p.x - bounds.x, p.y - bounds.y), bounds));
                (Status::Captured, message)
            }
            mouse::Event::ButtonReleased(mouse::Button::Left) => {
                *state = PickerDrag::None;
                (Status::Ignored, None)
            }
            _ => (Status::Ignored, None),
        }
    }

    fn draw(
        &self,
        _state: &PickerDrag,
        renderer: &iced::Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let (h, s, v) = self.hsv;
        let width = bounds.width;
        let sv_height = Self::sv_height(bounds);
        let hue_top = Self::hue_top(bounds);
        let mut frame = canvas::Frame::new(renderer, bounds.size());

        if self.layer == PickerLayer::Gradients {
            frame.draw_image(Rectangle::new(Point::ORIGIN, Size::new(width, sv_height)), &self.sv);
            frame.draw_image(
                Rectangle::new(Point::new(0.0, hue_top), Size::new(width, Self::HUE_HEIGHT)),
                &self.hue,
            );
            return vec![frame.into_geometry()];
        }

        let stroke = |width: f32, color: Color| canvas::Stroke::default().with_color(color).with_width(width);
        let shadow = m3::alpha(Color::BLACK, 0.45);

        // Crosshair: a horizontal and a vertical line across the square that
        // meet at the current colour. A dark halo keeps them visible on white.
        let centre = Point::new(s * width, (1.0 - v) * sv_height);
        let lines = [
            canvas::Path::line(Point::new(centre.x, 0.0), Point::new(centre.x, sv_height)),
            canvas::Path::line(Point::new(0.0, centre.y), Point::new(width, centre.y)),
        ];
        for line in &lines {
            frame.stroke(line, stroke(3.0, m3::alpha(Color::BLACK, 0.3)));
        }
        for line in &lines {
            frame.stroke(line, stroke(1.0, m3::alpha(Color::WHITE, 0.9)));
        }

        // The knob at the intersection, filled with the current colour.
        let knob = canvas::Path::circle(centre, 8.0);
        frame.fill(&knob, color(hsv_to_rgb(h, s, v)));
        frame.stroke(&canvas::Path::circle(centre, 9.5), stroke(1.0, shadow));
        frame.stroke(&knob, stroke(2.5, Color::WHITE));

        // Hue thumb: a round handle on the strip, like a Material slider.
        let radius = Self::HUE_HEIGHT / 2.0 + 2.0;
        let x = (h / 359.0 * width).clamp(radius + 2.0, width - radius - 2.0);
        let thumb_centre = Point::new(x, hue_top + Self::HUE_HEIGHT / 2.0);
        let thumb = canvas::Path::circle(thumb_centre, radius);
        frame.fill(&thumb, color(hsv_to_rgb(h, 1.0, 1.0)));
        frame.stroke(&canvas::Path::circle(thumb_centre, radius + 1.5), stroke(1.0, shadow));
        frame.stroke(&thumb, stroke(2.5, Color::WHITE));
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(&self, _state: &PickerDrag, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if self.layer == PickerLayer::Handles && cursor.is_over(bounds) {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::default()
        }
    }
}
