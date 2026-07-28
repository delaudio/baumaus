mod domain;
mod mesh;
mod parser;

use std::io;

use color_eyre::eyre::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use domain::{OpeningKind, Project};
use parser::compile;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Context, Line as CanvasLine},
        Block, Borders, Paragraph, Widget,
    },
    Frame, Terminal,
};
use ratatui_ratty::{ObjectFormat, RattyGraphic, RattyGraphicSettings};

const EXAMPLE: &str = r#"project(name = "Casa Baumaus");

wall([0, 0], [6000, 0], thickness = 300);
wall([6000, 0], [6000, 4500], thickness = 300);
wall([6000, 4500], [0, 4500], thickness = 300);
wall([0, 4500], [0, 0], thickness = 300);

door("wall-001", offset = 1200, width = 900);
window("wall-002", offset = 900, width = 1200, sill = 900);
view.fit();"#;
const DEFAULT_ROTATION: [f32; 3] = [35.0, -125.0, 0.0];
const DEFAULT_SCALE: f32 = 0.75;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Editor,
    Plan,
}
struct App {
    source: Vec<String>,
    project: Project,
    cursor_row: usize,
    cursor_col: usize,
    focus: Focus,
    auto_build: bool,
    preview: RattyGraphic<'static>,
    has_preview: bool,
    status: String,
    quit: bool,
}

impl App {
    fn new() -> Self {
        let source = EXAMPLE.lines().map(str::to_owned).collect();
        let mut app = Self {
            source,
            project: Project::default(),
            cursor_row: 0,
            cursor_col: 0,
            focus: Focus::Editor,
            auto_build: true,
            preview: RattyGraphic::new(
                RattyGraphicSettings::new("baumaus-live.obj")
                    .id(1)
                    .format(ObjectFormat::Obj)
                    .animate(false)
                    .scale(DEFAULT_SCALE)
                    .color([214, 188, 133])
                    .rotation(DEFAULT_ROTATION),
            ),
            has_preview: false,
            status: "Ready".into(),
            quit: false,
        };
        let _ = app.build();
        app
    }
    fn build(&mut self) -> bool {
        match compile(&self.source.join("\n")) {
            Ok(project) => {
                self.project = project;
                let mut status = format!(
                    "Built: {} walls, {} openings",
                    self.project.walls.len(),
                    self.project.openings.len()
                );
                match self
                    .preview
                    .register_payload(&mesh::project_to_obj(&self.project))
                {
                    Ok(()) => self.has_preview = true,
                    Err(error) => status = format!("{status} · Ratty preview: {error}"),
                }
                self.status = status;
                true
            }
            Err(error) => {
                self.status = format!("Line {}: {}", error.line, error.message);
                false
            }
        }
    }
    fn changed(&mut self) {
        if self.auto_build {
            let _ = self.build();
        } else {
            self.status = "Modified — press F5 to build".into();
        }
    }
    fn line(&self) -> &str {
        self.source.get(self.cursor_row).map_or("", String::as_str)
    }
    fn insert(&mut self, character: char) {
        let row = self.cursor_row;
        let col = self.cursor_col;
        self.source[row].insert(col, character);
        self.cursor_col += character.len_utf8();
        self.changed();
    }
    fn backspace(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col = previous_boundary(&self.source[self.cursor_row], self.cursor_col);
            self.source[self.cursor_row].remove(self.cursor_col);
        } else if self.cursor_row > 0 {
            let line = self.source.remove(self.cursor_row);
            self.cursor_row -= 1;
            self.cursor_col = self.source[self.cursor_row].len();
            self.source[self.cursor_row].push_str(&line);
        }
        self.changed();
    }
    fn newline(&mut self) {
        let rest = self.source[self.cursor_row].split_off(self.cursor_col);
        self.cursor_row += 1;
        self.cursor_col = 0;
        self.source.insert(self.cursor_row, rest);
        self.changed();
    }
    fn save(&mut self) {
        if !self.build() {
            return;
        }
        match serde_json::to_string_pretty(&self.project)
            .and_then(|text| std::fs::write("baumaus.json", text).map_err(serde_json::Error::io))
        {
            Ok(()) => self.status = "Saved baumaus.json".into(),
            Err(error) => self.status = format!("Could not save: {error}"),
        }
    }
    fn rotate_preview(&mut self, vertical: f32, horizontal: f32) {
        self.preview.settings_mut().rotation[0] += vertical * 5.0;
        self.preview.settings_mut().rotation[1] += horizontal * 5.0;
        self.update_preview("Could not rotate preview");
    }
    fn zoom_preview(&mut self, factor: f32) {
        self.preview.settings_mut().scale =
            (self.preview.settings().scale * factor).clamp(0.2, 2.0);
        self.update_preview("Could not zoom preview");
    }
    fn reset_preview(&mut self) {
        self.preview.settings_mut().rotation = DEFAULT_ROTATION;
        self.preview.settings_mut().scale = DEFAULT_SCALE;
        self.update_preview("Could not reset preview");
    }
    fn update_preview(&mut self, context: &str) {
        if self.has_preview {
            if let Err(error) = self.preview.update() {
                self.status = format!("{context}: {error}");
            }
        }
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    if let Err(error) = execute!(stdout, EnterAlternateScreen) {
        disable_raw_mode()?;
        return Err(error.into());
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(terminal) => terminal,
        Err(error) => {
            disable_raw_mode()?;
            execute!(io::stdout(), LeaveAlternateScreen)?;
            return Err(error.into());
        }
    };
    let result = run(&mut terminal);
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

fn run(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    let mut app = App::new();
    while !app.quit {
        terminal.draw(|frame| draw(frame, &app))?;
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    handle_key(&mut app, key.code);
                }
            }
        }
    }
    if app.has_preview {
        app.preview.clear()?;
    }
    Ok(())
}

fn handle_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') if app.focus == Focus::Plan => app.quit = true,
        KeyCode::Esc => app.focus = Focus::Plan,
        KeyCode::Tab => {
            app.focus = if app.focus == Focus::Editor {
                Focus::Plan
            } else {
                Focus::Editor
            }
        }
        KeyCode::F(5) => {
            let _ = app.build();
        }
        KeyCode::Char('a') if app.focus == Focus::Plan => {
            app.auto_build = !app.auto_build;
            app.status = format!("Auto-build {}", if app.auto_build { "on" } else { "off" });
        }
        KeyCode::Char('s') if app.focus == Focus::Plan => app.save(),
        KeyCode::Up if app.focus == Focus::Plan => app.rotate_preview(-1.0, 0.0),
        KeyCode::Down if app.focus == Focus::Plan => app.rotate_preview(1.0, 0.0),
        KeyCode::Left if app.focus == Focus::Plan => app.rotate_preview(0.0, -1.0),
        KeyCode::Right if app.focus == Focus::Plan => app.rotate_preview(0.0, 1.0),
        KeyCode::Char('z') if app.focus == Focus::Plan => app.zoom_preview(1.1),
        KeyCode::Char('x') if app.focus == Focus::Plan => app.zoom_preview(1.0 / 1.1),
        KeyCode::Char('r') if app.focus == Focus::Plan => app.reset_preview(),
        _ if app.focus == Focus::Editor => match code {
            KeyCode::Char(c) => app.insert(c),
            KeyCode::Backspace => app.backspace(),
            KeyCode::Enter => app.newline(),
            KeyCode::Left => app.cursor_col = previous_boundary(app.line(), app.cursor_col),
            KeyCode::Right => app.cursor_col = next_boundary(app.line(), app.cursor_col),
            KeyCode::Up => {
                app.cursor_row = app.cursor_row.saturating_sub(1);
                app.cursor_col = snap_boundary(app.line(), app.cursor_col);
            }
            KeyCode::Down => {
                app.cursor_row = (app.cursor_row + 1).min(app.source.len() - 1);
                app.cursor_col = snap_boundary(app.line(), app.cursor_col);
            }
            _ => {}
        },
        _ => {}
    }
}

fn previous_boundary(line: &str, cursor: usize) -> usize {
    line[..cursor]
        .char_indices()
        .last()
        .map_or(0, |(index, _)| index)
}

fn next_boundary(line: &str, cursor: usize) -> usize {
    line[cursor..]
        .chars()
        .next()
        .map_or(cursor, |character| cursor + character.len_utf8())
}

fn snap_boundary(line: &str, cursor: usize) -> usize {
    let cursor = cursor.min(line.len());
    if line.is_char_boundary(cursor) {
        cursor
    } else {
        line.char_indices()
            .take_while(|(index, _)| *index < cursor)
            .last()
            .map_or(0, |(index, _)| index)
    }
}

fn draw(frame: &mut Frame, app: &App) {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(8), Constraint::Length(2)])
        .split(frame.area());
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(areas[0]);
    draw_editor(frame, app, columns[0]);
    draw_plan(frame, app, columns[1]);
    let hint = format!(
        " {} | Tab pane · arrows rotate · z/x zoom · r reset · F5 build · a auto-build · s save · Esc plan · q quit",
        app.status
    );
    frame.render_widget(
        Paragraph::new(hint).style(Style::default().fg(Color::Black).bg(Color::Cyan)),
        areas[1],
    );
}

fn draw_editor(frame: &mut Frame, app: &App, area: Rect) {
    let title = if app.focus == Focus::Editor {
        " Script (editing) "
    } else {
        " Script "
    };
    let lines: Vec<Line> = app
        .source
        .iter()
        .enumerate()
        .map(|(index, text)| {
            let marker = if app.focus == Focus::Editor && index == app.cursor_row {
                ">"
            } else {
                " "
            };
            Line::from(vec![
                Span::styled(
                    format!("{marker}{:>3} ", index + 1),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(text),
            ])
        })
        .collect();
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border(app.focus == Focus::Editor)),
        ),
        area,
    );
    if app.focus == Focus::Editor {
        frame.set_cursor_position((
            area.x
                .saturating_add(5)
                .saturating_add(u16::try_from(app.cursor_col).unwrap_or(u16::MAX)),
            area.y
                .saturating_add(1)
                .saturating_add(u16::try_from(app.cursor_row).unwrap_or(u16::MAX)),
        ));
    }
}

fn draw_plan(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!(" Plan · {} ", app.project.name);
    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(border(app.focus == Focus::Plan)),
        )
        .x_bounds(bounds(&app.project, true))
        .y_bounds(bounds(&app.project, false))
        .marker(symbols::Marker::Braille)
        .paint(|ctx| render_project(ctx, &app.project));
    frame.render_widget(canvas, area);
    if app.has_preview {
        app.preview.render(area, frame.buffer_mut());
    }
}
fn border(active: bool) -> Style {
    if active {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}
fn bounds(project: &Project, x: bool) -> [f64; 2] {
    let Some((min, max)) = project.bounds() else {
        return [-1.0, 1.0];
    };
    let (low, high) = if x { (min.x, max.x) } else { (min.y, max.y) };
    let pad = ((high - low).abs() * 0.1).max(500.0);
    [low - pad, high + pad]
}
fn render_project(ctx: &mut Context, project: &Project) {
    for wall in &project.walls {
        ctx.draw(&CanvasLine {
            x1: wall.start.x,
            y1: wall.start.y,
            x2: wall.end.x,
            y2: wall.end.y,
            color: Color::White,
        });
    }
    for opening in &project.openings {
        if let Some(wall) = project.walls.iter().find(|wall| wall.id == opening.wall_id) {
            let length = (wall.end.x - wall.start.x).hypot(wall.end.y - wall.start.y);
            if length > 0.0 {
                let t = (opening.offset + opening.width / 2.0) / length;
                let x = wall.start.x + (wall.end.x - wall.start.x) * t;
                let y = wall.start.y + (wall.end.y - wall.start.y) * t;
                let color = match opening.kind {
                    OpeningKind::Door => Color::Yellow,
                    OpeningKind::Window { .. } => Color::Cyan,
                };
                ctx.print(x, y, Span::styled("●", Style::default().fg(color)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{next_boundary, previous_boundary, snap_boundary};

    #[test]
    fn cursor_moves_on_utf8_boundaries() {
        let line = "casa è";
        let end = line.len();
        let before_e = previous_boundary(line, end);
        assert_eq!(&line[before_e..], "è");
        assert_eq!(next_boundary(line, before_e), end);
        assert_eq!(snap_boundary("è", 1), 0);
    }
}
