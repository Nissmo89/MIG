use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Terminal,
};
use std::{error::Error, io};

struct App {
    projects: Vec<String>,
    api_status: String,
    recent_commits: Vec<String>,
}

impl App {
    fn new() -> App {
        App {
            projects: vec!["/home/nord/code_base/MIG".to_string()],
            api_status: "Groq (llama-3.1-8b-instant): ONLINE".to_string(),
            recent_commits: vec![
                "feat: add bitwise trie (cpp) - 2 mins ago".to_string(),
                "fix: typo in binary search (python) - 1 day ago".to_string(),
                "feat: complete MIG bot project structure - 2 days ago".to_string(),
            ],
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let app = App::new();

    let res = run_app(&mut terminal, app);

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

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(0),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            // Header
            let header = Paragraph::new("MIG (Make It Green) Control Dashboard")
                .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(header, chunks[0]);

            let body_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(30),
                        Constraint::Percentage(70),
                    ]
                    .as_ref(),
                )
                .split(chunks[1]);

            // Left Pane (Projects & API)
            let left_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(50),
                        Constraint::Percentage(50),
                    ]
                    .as_ref(),
                )
                .split(body_chunks[0]);

            let projects: Vec<ListItem> = app
                .projects
                .iter()
                .map(|p| ListItem::new(Line::from(vec![Span::raw(p)])))
                .collect();
            let projects_list = List::new(projects)
                .block(Block::default().borders(Borders::ALL).title("Managed Projects"));
            f.render_widget(projects_list, left_chunks[0]);

            let api_status = Paragraph::new(app.api_status.as_str())
                .style(Style::default().fg(Color::Cyan))
                .block(Block::default().borders(Borders::ALL).title("API Status"));
            f.render_widget(api_status, left_chunks[1]);

            // Right Pane (Commits)
            let commits: Vec<ListItem> = app
                .recent_commits
                .iter()
                .map(|c| ListItem::new(Line::from(vec![Span::raw(c)])))
                .collect();
            let commits_list = List::new(commits)
                .block(Block::default().borders(Borders::ALL).title("Recent Commits (Global)"));
            f.render_widget(commits_list, body_chunks[1]);
        })?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if let KeyCode::Char('q') = key.code {
                    return Ok(());
                }
            }
        }
    }
}
