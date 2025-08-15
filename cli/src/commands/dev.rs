use anyhow::Result;
use colored::*;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use std::{
    io,
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::client::Client;

pub async fn run(
    project: PathBuf,
    language: Option<String>,
    client: &Client,
) -> Result<()> {
    println!("{}", "🚀 Starting Interactive Development Environment...".bright_blue().bold());
    
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = DevApp::new(project, language, client).await?;
    
    // Run the app
    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

struct DevApp<'a> {
    client: &'a Client,
    project_path: PathBuf,
    language: Option<String>,
    current_file: Option<PathBuf>,
    file_content: String,
    ai_response: String,
    input_mode: InputMode,
    input_buffer: String,
    cursor_position: usize,
    files: Vec<PathBuf>,
    selected_file: usize,
    show_help: bool,
    status_message: String,
    last_update: Instant,
}

#[derive(PartialEq)]
enum InputMode {
    Normal,
    Editing,
    Command,
}

impl<'a> DevApp<'a> {
    async fn new(project: PathBuf, language: Option<String>, client: &'a Client) -> Result<Self> {
        let files = discover_files(&project)?;
        
        Ok(Self {
            client,
            project_path: project,
            language,
            current_file: None,
            file_content: String::new(),
            ai_response: String::new(),
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            cursor_position: 0,
            files,
            selected_file: 0,
            show_help: false,
            status_message: "Welcome to UAIDA Dev Environment! Press 'h' for help.".to_string(),
            last_update: Instant::now(),
        })
    }

    fn load_file(&mut self, file_path: PathBuf) -> Result<()> {
        if file_path.is_file() {
            self.file_content = std::fs::read_to_string(&file_path)?;
            self.current_file = Some(file_path);
            self.status_message = format!("Loaded: {}", self.current_file.as_ref().unwrap().display());
        }
        Ok(())
    }

    async fn request_completion(&mut self) -> Result<()> {
        if self.file_content.is_empty() {
            self.status_message = "No content to complete".to_string();
            return Ok(());
        }

        self.status_message = "Requesting AI completion...".to_string();
        
        let request = crate::client::CompletionRequest {
            prompt: self.file_content.clone(),
            language: self.language.clone(),
            model: None,
            provider: None,
            max_tokens: Some(500),
            temperature: Some(0.7),
            system_prompt: Some("Complete this code with best practices".to_string()),
        };

        match self.client.complete(request).await {
            Ok(response) => {
                self.ai_response = response.text;
                self.status_message = format!("Completion from {}", response.provider);
            }
            Err(e) => {
                self.status_message = format!("Error: {}", e);
            }
        }

        Ok(())
    }
}

async fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut DevApp<'_>,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match app.input_mode {
                    InputMode::Normal => match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Char('h') => app.show_help = !app.show_help,
                        KeyCode::Char('o') => {
                            if !app.files.is_empty() && app.selected_file < app.files.len() {
                                let file = app.files[app.selected_file].clone();
                                app.load_file(file)?;
                            }
                        }
                        KeyCode::Char('c') => {
                            app.request_completion().await?;
                        }
                        KeyCode::Up => {
                            if app.selected_file > 0 {
                                app.selected_file -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if app.selected_file < app.files.len().saturating_sub(1) {
                                app.selected_file += 1;
                            }
                        }
                        KeyCode::Enter => {
                            if !app.files.is_empty() && app.selected_file < app.files.len() {
                                let file = app.files[app.selected_file].clone();
                                app.load_file(file)?;
                            }
                        }
                        _ => {}
                    },
                    InputMode::Editing => {
                        // Handle text editing
                        match key.code {
                            KeyCode::Esc => app.input_mode = InputMode::Normal,
                            KeyCode::Char(c) => {
                                app.input_buffer.insert(app.cursor_position, c);
                                app.cursor_position += 1;
                            }
                            KeyCode::Backspace => {
                                if app.cursor_position > 0 {
                                    app.input_buffer.remove(app.cursor_position - 1);
                                    app.cursor_position -= 1;
                                }
                            }
                            _ => {}
                        }
                    }
                    InputMode::Command => {
                        // Handle command mode
                        match key.code {
                            KeyCode::Esc => app.input_mode = InputMode::Normal,
                            KeyCode::Enter => {
                                // Process command
                                app.input_mode = InputMode::Normal;
                                app.input_buffer.clear();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &DevApp) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .split(f.size());

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
        .split(chunks[1]);

    // File explorer
    let files: Vec<ListItem> = app
        .files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let style = if i == app.selected_file {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(file.file_name().unwrap().to_string_lossy().to_string()).style(style)
        })
        .collect();

    let files_list = List::new(files)
        .block(Block::default().borders(Borders::ALL).title("Files"))
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol(">> ");

    f.render_widget(files_list, chunks[0]);

    // File content
    let file_content = Paragraph::new(app.file_content.as_ref())
        .block(Block::default().borders(Borders::ALL).title("Current File"))
        .wrap(Wrap { trim: true });

    f.render_widget(file_content, right_chunks[0]);

    // AI Response
    let ai_response = Paragraph::new(app.ai_response.as_ref())
        .block(Block::default().borders(Borders::ALL).title("AI Assistant"))
        .wrap(Wrap { trim: true });

    f.render_widget(ai_response, right_chunks[1]);

    // Status bar
    let status_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)].as_ref())
        .split(f.size());

    let status = Paragraph::new(app.status_message.as_ref())
        .style(Style::default().fg(Color::White).bg(Color::Blue));

    f.render_widget(status, status_chunks[1]);

    // Help overlay
    if app.show_help {
        let help_text = vec![
            Line::from("UAIDA Development Environment"),
            Line::from(""),
            Line::from("Navigation:"),
            Line::from("  ↑/↓  - Navigate files"),
            Line::from("  Enter/o - Open file"),
            Line::from(""),
            Line::from("AI Features:"),
            Line::from("  c - Request completion"),
            Line::from("  a - Analyze code"),
            Line::from(""),
            Line::from("General:"),
            Line::from("  h - Toggle help"),
            Line::from("  q - Quit"),
        ];

        let help = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .wrap(Wrap { trim: true });

        let area = centered_rect(60, 70, f.size());
        f.render_widget(Clear, area);
        f.render_widget(help, area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn discover_files(project_path: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    
    if project_path.is_dir() {
        for entry in walkdir::WalkDir::new(project_path)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if matches!(ext.to_str(), Some("rs") | Some("py") | Some("js") | Some("ts") | Some("go") | Some("java") | Some("cpp") | Some("c") | Some("h")) {
                        files.push(entry.path().to_path_buf());
                    }
                }
            }
        }
    }
    
    files.sort();
    Ok(files)
}