use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::{error::Error, io, time::{Duration, Instant}};

// Catppuccin Mocha Theme Colors
const C_MAUVE: Color = Color::Rgb(203, 166, 247);
const C_BLUE: Color = Color::Rgb(137, 180, 250);
const C_LAVENDER: Color = Color::Rgb(180, 190, 254);
const C_GREEN: Color = Color::Rgb(166, 227, 161);
const C_BASE: Color = Color::Rgb(30, 30, 46);

struct App {
    projects: Vec<String>,
    api_status: String,
    recent_commits: Vec<String>,
    start_time: Instant,
}

impl App {
    fn new() -> App {
        App {
            projects: vec![
                "▶ /home/nord/code_base/MIG".to_string(),
                "▶ /home/nord/code_base/backend_api".to_string(),
            ],
            api_status: "[GROQ] llama-3.1-8b-instant : ONLINE\n[GEMINI] gemini-2.5-flash : STANDBY".to_string(),
            recent_commits: vec![
                "[0xFA19] feat: bitwise trie optimizations (cpp) - 2m ago".to_string(),
                "[0x9B2C] fix: overflow in binary search (python) - 1d ago".to_string(),
                "[0x11DF] feat: initialization sequence complete - 2d ago".to_string(),
            ],
            start_time: Instant::now(),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new();
    let res = run_app(&mut terminal, app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }
    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    let tick_rate = Duration::from_millis(50);

    loop {
        terminal.draw(|f| {
            // Main layout
            let size = f.size();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(8),  // Header / Logo
                    Constraint::Min(0),     // Body
                ])
                .split(size);

            // Time based blink effect for status
            let elapsed = app.start_time.elapsed().as_secs_f32();
            let blink = (elapsed * 2.0).sin() > 0.0;
            let status_color = if blink { C_GREEN } else { C_LAVENDER };

            // 1. BIG MIG LOGO (Catppuccin Style)
            let mig_logo = vec![
                Line::from(Span::styled(r#" ███╗   ███╗██╗ ██████╗ "#, Style::default().fg(C_MAUVE).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(r#" ████╗ ████║██║██╔════╝ "#, Style::default().fg(C_MAUVE).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(r#" ██╔████╔██║██║██║  ███╗"#, Style::default().fg(C_BLUE).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(r#" ██║╚██╔╝██║██║██║   ██║"#, Style::default().fg(C_BLUE).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(r#" ██║ ╚═╝ ██║██║╚██████╔╝"#, Style::default().fg(C_LAVENDER).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(r#" ╚═╝     ╚═╝╚═╝ ╚═════╝ "#, Style::default().fg(C_LAVENDER).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(if blink { "● SYSTEM ONLINE" } else { "○ SYSTEM ONLINE" }, Style::default().fg(status_color))),
            ];

            let logo_widget = Paragraph::new(mig_logo)
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(C_MAUVE))
                );
            f.render_widget(logo_widget, chunks[0]);

            // Body Layout
            let body_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(chunks[1]);

            let left_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(body_chunks[0]);

            // 2. PROJECTS (Catppuccin List)
            let projects: Vec<ListItem> = app.projects.iter().map(|p| {
                ListItem::new(Line::from(Span::styled(p, Style::default().fg(C_LAVENDER))))
            }).collect();
            let projects_list = List::new(projects)
                .block(Block::default()
                    .title(" Managed Projects ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(C_BLUE)));
            f.render_widget(projects_list, left_chunks[0]);

            // 3. API STATUS
            let api_status = Paragraph::new(app.api_status.as_str())
                .style(Style::default().fg(C_LAVENDER))
                .block(Block::default()
                    .title(" API Connections ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(C_BLUE)));
            f.render_widget(api_status, left_chunks[1]);

            // 4. RECENT COMMITS
            let commits: Vec<ListItem> = app.recent_commits.iter().map(|c| {
                ListItem::new(Line::from(Span::styled(c, Style::default().fg(C_LAVENDER))))
            }).collect();
            let commits_list = List::new(commits)
                .block(Block::default()
                    .title(" Activity Feed ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(C_MAUVE)));
            f.render_widget(commits_list, body_chunks[1]);
        })?;

        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                if let KeyCode::Char('q') = key.code {
                    return Ok(());
                }
            }
        }
    }
}
