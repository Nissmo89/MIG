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

// Cyberpunk Theme Colors
const NEON_PINK: Color = Color::Rgb(255, 0, 255);
const NEON_CYAN: Color = Color::Rgb(0, 255, 255);
const NEON_YELLOW: Color = Color::Rgb(255, 255, 0);
const DARK_BG: Color = Color::Rgb(10, 10, 15);

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

            // Time based blink effect
            let elapsed = app.start_time.elapsed().as_secs_f32();
            let blink = (elapsed * 3.0).sin() > 0.0;
            let neon_color = if blink { NEON_PINK } else { NEON_CYAN };

            // 1. BIG MIG LOGO (Cyberpunk Style)
            let mig_logo = vec![
                Line::from(Span::styled(r#" ███╗   ███╗██╗ ██████╗ "#, Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(r#" ████╗ ████║██║██╔════╝ "#, Style::default().fg(NEON_CYAN).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(r#" ██╔████╔██║██║██║  ███╗"#, Style::default().fg(NEON_PINK).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(r#" ██║╚██╔╝██║██║██║   ██║"#, Style::default().fg(NEON_PINK).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(r#" ██║ ╚═╝ ██║██║╚██████╔╝"#, Style::default().fg(NEON_YELLOW).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(r#" ╚═╝     ╚═╝╚═╝ ╚═════╝ "#, Style::default().fg(NEON_YELLOW).add_modifier(Modifier::BOLD))),
                Line::from(Span::styled(if blink { ">>> NEURAL LINK ACTIVE <<<" } else { ">>> STANDBY <<<" }, Style::default().fg(neon_color))),
            ];

            let logo_widget = Paragraph::new(mig_logo)
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(NEON_CYAN))
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

            // 2. PROJECTS (Cyberpunk List)
            let projects: Vec<ListItem> = app.projects.iter().map(|p| {
                ListItem::new(Line::from(Span::styled(p, Style::default().fg(NEON_YELLOW))))
            }).collect();
            let projects_list = List::new(projects)
                .block(Block::default()
                    .title(" SYSTEM_TARGETS ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Thick)
                    .border_style(Style::default().fg(NEON_PINK)));
            f.render_widget(projects_list, left_chunks[0]);

            // 3. API STATUS
            let api_status = Paragraph::new(app.api_status.as_str())
                .style(Style::default().fg(Color::White))
                .block(Block::default()
                    .title(" UPLINK_STATUS ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Thick)
                    .border_style(Style::default().fg(NEON_CYAN)));
            f.render_widget(api_status, left_chunks[1]);

            // 4. RECENT COMMITS
            let commits: Vec<ListItem> = app.recent_commits.iter().map(|c| {
                ListItem::new(Line::from(Span::styled(c, Style::default().fg(Color::Green))))
            }).collect();
            let commits_list = List::new(commits)
                .block(Block::default()
                    .title(" NEURAL_LOGS ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(neon_color)));
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
