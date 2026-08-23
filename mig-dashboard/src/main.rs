use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Chart, Dataset, Axis},
    Terminal,
};
use std::{error::Error, io, time::{Duration, Instant}};

// Catppuccin Mocha Theme Colors
const C_MAUVE: Color = Color::Rgb(203, 166, 247);
const C_BLUE: Color = Color::Rgb(137, 180, 250);
const C_LAVENDER: Color = Color::Rgb(180, 190, 254);
const C_GREEN: Color = Color::Rgb(166, 227, 161);
const C_BASE: Color = Color::Rgb(30, 30, 46);
const C_SURFACE: Color = Color::Rgb(46, 52, 66);
const C_ORANGE: Color = Color::Rgb(253, 150, 83);
const C_YELLOW: Color = Color::Rgb(255, 204, 89);
const C_ROSE: Color = Color::Rgb(245, 104, 168);

struct App {
    projects: Vec<String>,
    api_status: String,
    recent_commits: Vec<String>,
    start_time: Instant,
    chart_data: Vec<f64>,
    scroll: i16,
    focus_idx: usize,
    quit_requested: bool,
}

impl App {
    fn new() -> App {
        let chart_data: Vec<f64> = (0..10)
            .map(|i| (i as f64 * 0.5).sin() * 5.0)
            .collect();
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
            chart_data,
            scroll: 0,
            focus_idx: 0,
            quit_requested: false,
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
            let size = f.size();

            // ---- Layout ----
            // Header (8 lines), body, bottom strip (3 lines)
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints([
                    Constraint::Length(8),
                    Constraint::Min(0),
                    Constraint::Length(3),
                ])
                .split(size);

            // ------------------------------------------------
            // 1. HEADER: logo + blinking status + clock
            // ------------------------------------------------
            let elapsed = app.start_time.elapsed().as_secs_f64();
            let blink = (elapsed * 2.0).sin() > 0.0;
            let status_color = if blink { C_GREEN } else { C_LAVENDER };

            let mig_logo = vec![
                Line::from(Span::styled(
                    r#" ███╗   ███╗██╗ ██████╗ "#,
                    Style::default().fg(C_MAUVE).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    r#" ████╗ ████║██║██╔════╝ "#,
                    Style::default().fg(C_MAUVE).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    r#" ██╔████╔██║██║██║  ███╗"#,
                    Style::default().fg(C_BLUE).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    r#" ██║╚██╔╝██║██║██║   ██║"#,
                    Style::default().fg(C_BLUE).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    r#" ██║ ╚═╝ ██║██║╚██████╔╝"#,
                    Style::default().fg(C_LAVENDER).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    r#" ╚═╝     ╚═╝╚═╝ ╚═════╝ "#,
                    Style::default().fg(C_LAVENDER).add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    if blink {
                        "● SYSTEM ONLINE"
                    } else {
                        "○ SYSTEM ONLINE"
                    },
                    Style::default().fg(status_color),
                )),
            ];
            let logo_widget = Paragraph::new(mig_logo)
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(C_MAUVE)));
            f.render_widget(logo_widget, chunks[0]);

            // Clock top-right
            let total_secs = app.start_time.elapsed().as_secs();
            let hrs = total_secs / 3600;
            let mins = (total_secs % 3600) / 60;
            let secs = total_secs % 60;
            let time_str = format!("{:02}:{:02}:{:02}", hrs, mins, secs);
            let clock_widget = Paragraph::new(time_str)
                .style(Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD))
                .alignment(Alignment::Right);
            let clock_area = Rect {
                x: size.width.saturating_sub(10),
                y: chunks[0].y + 1,
                width: 10,
                height: 1,
            };
            f.render_widget(clock_widget, clock_area);

            // ------------------------------------------------
            // 2. BODY
            // ------------------------------------------------
            let body_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(chunks[1]);

            // Left half: projects + API status
            let left_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(body_chunks[0]);

            // Projects list
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

            // API status
            let api_status = Paragraph::new(app.api_status.as_str())
                .style(Style::default().fg(C_LAVENDER))
                .block(Block::default()
                    .title(" API Connections ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(C_BLUE)));
            f.render_widget(api_status, left_chunks[1]);

            // Right half: commits + chart
            let right_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
                .split(body_chunks[1]);

            // Commits
            let commits: Vec<ListItem> = app.recent_commits.iter().map(|c| {
                ListItem::new(Line::from(Span::styled(c, Style::default().fg(C_LAVENDER))))
            }).collect();
            let commits_list = List::new(commits)
                .block(Block::default()
                    .title(" Activity Feed ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(C_MAUVE)));
            f.render_widget(commits_list, right_chunks[0]);

            // Chart
            app.chart_data.remove(0);
            let new_val: f64 = (elapsed * 1.0).sin() * 5.0;
            app.chart_data.push(new_val);

            let data_points: Vec<(f64, f64)> = app
                .chart_data
                .iter()
                .enumerate()
                .map(|(i, &y)| (i as f64, y))
                .collect();
            let dataset = Dataset::default()
                .data(&data_points)
                .style(Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD));
            let chart = Chart::new(vec![dataset])
                .x_axis(
                    Axis::default()
                        .bounds([0.0, 10.0])
                        .style(Style::default().fg(C_SURFACE)),
                )
                .y_axis(
                    Axis::default()
                        .bounds([-5.0, 5.0])
                        .style(Style::default().fg(C_SURFACE)),
                );
            f.render_widget(chart, right_chunks[1]);

            // ------------------------------------------------
            // 3. BOTTOM STRIP: interactive buttons
            // ------------------------------------------------
            // Buttons area height is 1 line, width equals full strip width
            let button_chunk = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(100)])
                .split(chunks[2])[0];

            // Buttons: focus_idx 0 => Graph, 1 => Quit
            let button_focus = app.focus_idx;

            let button_width = button_chunk.width / 2;
            let button_height = button_chunk.height;

            // --- Graph button ---
            let graph_btn_style = if button_focus == 0 {
                Style::default()
                    .fg(C_MAUVE)
                    .add_modifier(Modifier::BOLD)
                    .bg(C_SURFACE)
            } else {
                Style::default().fg(C_MAUVE)
            };
            let graph_btn = Paragraph::new(" Graph ")
                .style(graph_btn_style)
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(C_MAUVE)));
            let graph_btn_rect = Rect {
                x: button_chunk.x,
                y: button_chunk.y,
                width: button_width,
                height: button_height,
            };
            f.render_widget(graph_btn, graph_btn_rect);

            // --- Quit button ---
            let quit_btn_style = if button_focus == 1 {
                Style::default()
                    .fg(C_ROSE)
                    .add_modifier(Modifier::BOLD)
                    .bg(C_SURFACE)
            } else {
                Style::default().fg(C_ROSE)
            };
            let quit_btn = Paragraph::new(" Quit ")
                .style(quit_btn_style)
                .alignment(Alignment::Center)
                .block(Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(C_ROSE)));
            let quit_btn_rect = Rect {
                x: button_chunk.x + button_width,
                y: button_chunk.y,
                width: button_width,
                height: button_height,
            };
            f.render_widget(quit_btn, quit_btn_rect);

            // ------------------------------------------------
            // 4. Handle key events after draw
            // ------------------------------------------------
            // Poll once per tick to process input
        })?;

        // Process events after draw
        if event::poll(tick_rate)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Up => {
                        app.focus_idx = if app.focus_idx == 0 { 1 } else { 0 };
                    }
                    KeyCode::Down => {
                        app.focus_idx = if app.focus_idx == 0 { 1 } else { 0 };
                    }
                    KeyCode::Enter => {
                        if app.focus_idx == 1 {
                            // Quit button activated
                            return Ok(());
                        } else {
                            // Graph button pressed: toggle focus
                            app.focus_idx = (app.focus_idx + 1) % 2;
                        }
                    }
                    KeyCode::Char('q') => {
                        // immediate quit
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }
    }
}