use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind, MouseButton},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph, Chart, Dataset, Axis, Tabs},
    Terminal,
};
use std::{error::Error, io, time::{Duration, Instant}};
use std::env;
use std::process::Command;
use serde_json::Value;

mod github;
use github::{GitHubContributionService, ContributionCalendar};

const C_MAUVE: Color = Color::Rgb(203, 166, 247);
const C_BLUE: Color = Color::Rgb(137, 180, 250);
const C_LAVENDER: Color = Color::Rgb(180, 190, 254);
const C_GREEN: Color = Color::Rgb(166, 227, 161);
const C_SURFACE: Color = Color::Rgb(46, 52, 66);
const C_YELLOW: Color = Color::Rgb(255, 204, 89);
const C_ROSE: Color = Color::Rgb(245, 104, 168);
const C_GRAY: Color = Color::Rgb(108, 112, 134);

struct Stats {
    lifetime_tokens: u64,
    peak_tokens: u64,
    longest_task: f64,
    activity: Vec<String>,
}

struct ApiStatus {
    model_name: String,
    account_tier: String,
    limit_info: String,
    limit_percent: f64,
}

use std::collections::HashMap;

#[derive(Debug, Default, serde::Deserialize, Clone)]
pub struct GroqModelStats {
    pub total_tokens: u64,
    pub cost: f64,
    pub runs: u64,
}

#[derive(Debug, Default, serde::Deserialize, Clone)]
pub struct GroqDailyStats {
    pub tokens: u64,
    pub cost: f64,
}

#[derive(Debug, Default, serde::Deserialize, Clone)]
pub struct GroqStats {
    pub total_cost: f64,
    pub models: HashMap<String, GroqModelStats>,
    pub daily: HashMap<String, GroqDailyStats>,
}

struct App {
    projects: Vec<String>,
    recent_commits: Vec<String>,
    start_time: Instant,
    chart_data: Vec<f64>,
    focus_idx: usize,
    right_panel_view: usize,
    active_tab: usize,
    keys_status: [bool; 3],
    api_details: [String; 3],
    stats: Stats,
    api_status: ApiStatus,
    github_cal: Option<ContributionCalendar>,
    github_error: Option<String>,
    mouse_pos: Option<(u16, u16)>,
    groq_stats: Option<GroqStats>,
}

impl App {
    fn fetch_api_status() -> ApiStatus {
        let mut status = ApiStatus {
            model_name: "Local/Default".to_string(),
            account_tier: "Unknown".to_string(),
            limit_info: "Unknown".to_string(),
            limit_percent: 0.0,
        };

        if let Ok(rt) = tokio::runtime::Runtime::new() {
            rt.block_on(async {
                let client = reqwest::Client::new();
                if let Ok(key) = env::var("OPENROUTER_API_KEY") {
                    status.model_name = "OpenRouter Auto".to_string();
                    if let Ok(resp) = client.get("https://openrouter.ai/api/v1/auth/key")
                        .header("Authorization", format!("Bearer {}", key))
                        .send().await {
                        if let Ok(json) = resp.json::<serde_json::Value>().await {
                            if let Some(data) = json.get("data") {
                                let limit = data.get("limit").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                let usage = data.get("usage").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                let is_free = data.get("is_free_tier").and_then(|v| v.as_bool()).unwrap_or(false);
                                status.account_tier = if is_free { "Free Tier".to_string() } else { "Paid Tier".to_string() };
                                
                                if limit > 0.0 {
                                    status.limit_percent = (usage / limit) * 100.0;
                                    status.limit_info = format!("Usage: ${:.4} / ${:.4}", usage, limit);
                                } else {
                                    status.limit_info = format!("Usage: ${:.4} (No Limit)", usage);
                                }
                            }
                        }
                    }
                } else if let Ok(key) = env::var("GROQ_API_KEY") {
                    status.model_name = "Groq Simulated".to_string();
                    let req_body = serde_json::json!({
                        "model": "groq/compound-mini",
                        "messages": [{"role": "user", "content": "hi"}]
                    });
                    if let Ok(resp) = client.post("https://api.groq.com/openai/v1/chat/completions")
                        .header("Authorization", format!("Bearer {}", key))
                        .json(&req_body)
                        .send().await {
                            
                        let headers = resp.headers();
                        let limit_req = headers.get("x-ratelimit-limit-requests").and_then(|v| v.to_str().ok()).unwrap_or("0");
                        let rem_req = headers.get("x-ratelimit-remaining-requests").and_then(|v| v.to_str().ok()).unwrap_or("0");
                        
                        let limit: f64 = limit_req.parse().unwrap_or(0.0);
                        let rem: f64 = rem_req.parse().unwrap_or(0.0);
                        
                        status.account_tier = "Groq Developer".to_string();
                        if limit > 0.0 {
                            status.limit_percent = ((limit - rem) / limit) * 100.0;
                            status.limit_info = format!("Reqs: {}/{} remaining", rem_req, limit_req);
                        } else {
                            status.limit_info = "Status Active".to_string();
                        }
                    }
                } else if let Ok(_) = env::var("GEMINI_API_KEY") {
                    status.model_name = "Gemini-1.5-Pro".to_string();
                    status.account_tier = "Google Default".to_string();
                    status.limit_info = "Active (Quotas apply)".to_string();
                }
            });
        }
        status
    }

    fn get_stats() -> Stats {
        let default_stats = Stats {
            lifetime_tokens: 0,
            peak_tokens: 0,
            longest_task: 0.0,
            activity: vec![],
        };
        
        let home = match env::var("HOME") {
            Ok(h) => h,
            Err(_) => return default_stats,
        };
        
        let path = format!("{}/.mig/stats.json", home);
        if let Ok(file_content) = std::fs::read_to_string(&path) {
            if let Ok(json) = serde_json::from_str::<Value>(&file_content) {
                return Stats {
                    lifetime_tokens: json["lifetime_tokens"].as_u64().unwrap_or(0),
                    peak_tokens: json["peak_tokens"].as_u64().unwrap_or(0),
                    longest_task: json["longest_task"].as_f64().unwrap_or(0.0),
                    activity: json["activity"].as_array().map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default(),
                };
            }
        }
        default_stats
    }

    fn new() -> App {
        let _ = dotenv::from_filename("../.env");
        let _ = dotenv::dotenv();

        let github_username = env::var("GITHUB_USERNAME").ok();
        let github_token = env::var("GITHUB_TOKEN").ok();
        
        let mut github_cal = None;
        let mut github_error = None;
        
        if let (Some(username), Some(token)) = (&github_username, &github_token) {
            if let Ok(rt) = tokio::runtime::Runtime::new() {
                rt.block_on(async {
                    let svc = GitHubContributionService::new();
                    match svc.get_contributions(username, token, false).await {
                        Ok(cal) => github_cal = Some(cal),
                        Err(e) => github_error = Some(e),
                    }
                });
            }
        } else {
            github_error = Some("GITHUB_USERNAME or GITHUB_TOKEN not found in .env".to_string());
        }

        let openrouter_key = env::var("OPENROUTER_API_KEY").ok();
        let gemini_key = env::var("GEMINI_API_KEY").ok();
        let groq_key = env::var("GROQ_API_KEY").ok();

        let keys_status = [
            openrouter_key.is_some(),
            gemini_key.is_some(),
            groq_key.is_some(),
        ];

        let mut active_tab = 0;
        if keys_status[0] { active_tab = 0; }
        else if keys_status[1] { active_tab = 1; }
        else if keys_status[2] { active_tab = 2; }

        let format_key = |k: &Option<String>| -> String {
            if let Some(key) = k {
                if key.len() > 10 {
                    format!("{}...{}", &key[0..5], &key[key.len()-5..])
                } else {
                    "***".to_string()
                }
            } else {
                "None".to_string()
            }
        };

        let api_details = [
            if keys_status[0] {
                format!("Provider: OpenRouter\nStatus: ONLINE\nKey: {}", format_key(&openrouter_key))
            } else {
                "Provider: OpenRouter\nStatus: OFFLINE\nAPI Key missing in .env".to_string()
            },
            if keys_status[1] {
                format!("Provider: Gemini\nStatus: ONLINE\nKey: {}", format_key(&gemini_key))
            } else {
                "Provider: Gemini\nStatus: OFFLINE\nAPI Key missing in .env".to_string()
            },
            if keys_status[2] {
                format!("Provider: Groq\nStatus: ONLINE\nKey: {}", format_key(&groq_key))
            } else {
                "Provider: Groq\nStatus: OFFLINE\nAPI Key missing in .env".to_string()
            },
        ];

        let mut loaded_projects = vec![];
        let mut project_paths = vec![];
        let proj_path = format!("{}/.mig/projects.json", env::var("HOME").unwrap_or_default());
        if let Ok(file_content) = std::fs::read_to_string(&proj_path) {
            if let Ok(json_arr) = serde_json::from_str::<Vec<String>>(&file_content) {
                for p in json_arr {
                    loaded_projects.push(format!("▶ {}", p));
                    project_paths.push(p);
                }
            }
        }
        if loaded_projects.is_empty() {
            loaded_projects.push("No projects tracked yet.".to_string());
            let cwd = env::current_dir().unwrap_or_default().to_string_lossy().to_string();
            project_paths.push(cwd);
        }

        let mut recent_commits = vec![];
        for path in &project_paths {
            if let Ok(output) = Command::new("git")
                .current_dir(path)
                .args(&["log", "-n", "3", "--pretty=format:%h - %s (%cr)"])
                .output() {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let repo_name = path.split('/').last().unwrap_or("repo");
                for line in output_str.lines() {
                    if !line.trim().is_empty() {
                        recent_commits.push(format!("[{}] {}", repo_name, line.trim()));
                    }
                }
            }
        }
        if recent_commits.is_empty() {
            recent_commits.push("No recent commits found across tracked projects.".to_string());
        }

        let chart_data: Vec<f64> = (0..200)
            .map(|i| (i as f64 * 0.1).sin() * 5.0)
            .collect();

        let mut groq_stats = None;
        let mut stats_dir = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
        stats_dir.push(".mig");
        let g_stats_file = stats_dir.join("groq_analytics.json");
        if let Ok(c) = std::fs::read_to_string(&g_stats_file) {
            if let Ok(g) = serde_json::from_str::<GroqStats>(&c) {
                groq_stats = Some(g);
            }
        }
        App {
            projects: loaded_projects,
            recent_commits,
            start_time: Instant::now(),
            chart_data,
            focus_idx: 0,
            right_panel_view: 0,
            active_tab,
            keys_status,
            api_details,
            stats: Self::get_stats(),
            api_status: Self::fetch_api_status(),
            github_cal,
            github_error,
            mouse_pos: None,
            groq_stats,
        }
    }

    fn next_tab(&mut self) {
        for i in 1..=3 {
            let next = (self.active_tab + i) % 3;
            if self.keys_status[next] {
                self.active_tab = next;
                break;
            }
        }
    }

    fn prev_tab(&mut self) {
        for i in 1..=3 {
            let prev = (self.active_tab + 3 - i) % 3;
            if self.keys_status[prev] {
                self.active_tab = prev;
                break;
            }
        }
    }
}

fn is_inside(rect: Rect, col: u16, row: u16) -> bool {
    col >= rect.x && col < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
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
        let size = terminal.size()?;

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(8),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(size);

        let body_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(chunks[1]);

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Min(0)])
            .split(body_chunks[0]);

        let api_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)])
            .split(left_chunks[1]);

        let button_chunk = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(chunks[2])[0];

        let button_width = button_chunk.width / 2;
        let graph_btn_rect = Rect {
            x: button_chunk.x,
            y: button_chunk.y,
            width: button_width,
            height: button_chunk.height,
        };
        let quit_btn_rect = Rect {
            x: button_chunk.x + button_width,
            y: button_chunk.y,
            width: button_width,
            height: button_chunk.height,
        };
        
        let tab_0_rect = Rect { x: api_area[0].x + 1, y: api_area[0].y + 1, width: 10, height: 1 };
        let tab_1_rect = Rect { x: api_area[0].x + 1 + 10 + 3, y: api_area[0].y + 1, width: 6, height: 1 };
        let tab_2_rect = Rect { x: api_area[0].x + 1 + 10 + 3 + 6 + 3, y: api_area[0].y + 1, width: 4, height: 1 };

        terminal.draw(|f| {
            let elapsed = app.start_time.elapsed().as_secs_f64();
            let blink = (elapsed * 2.0).sin() > 0.0;
            let status_color = if blink { C_GREEN } else { C_LAVENDER };

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
                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(C_MAUVE)));
            f.render_widget(logo_widget, chunks[0]);

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

            let projects: Vec<ListItem> = app.projects.iter().map(|p| {
                ListItem::new(Line::from(Span::styled(p, Style::default().fg(C_LAVENDER))))
            }).collect();
            let projects_list = List::new(projects)
                .block(Block::default().title(" Managed Projects ").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(C_BLUE)));
            f.render_widget(projects_list, left_chunks[0]);

            let tab_titles = ["OpenRouter", "Gemini", "Groq"].iter().enumerate().map(|(i, &t)| {
                if app.keys_status[i] {
                    Line::from(t)
                } else {
                    Line::from(Span::styled(t, Style::default().fg(C_GRAY)))
                }
            }).collect::<Vec<_>>();
            let tabs = Tabs::new(tab_titles)
                .block(Block::default().borders(Borders::ALL).title(" API Keys ").border_type(BorderType::Rounded).border_style(Style::default().fg(C_BLUE)))
                .select(app.active_tab)
                .highlight_style(Style::default().fg(C_YELLOW).add_modifier(Modifier::BOLD))
                .divider(Span::styled(" | ", Style::default().fg(C_SURFACE)));
            f.render_widget(tabs, api_area[0]);

            let api_details = Paragraph::new(app.api_details[app.active_tab].as_str())
                .style(Style::default().fg(C_LAVENDER))
                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(C_BLUE)));
            f.render_widget(api_details, api_area[1]);

            let right_chunk = body_chunks[1];

            match app.right_panel_view {
                1 => {
                    // Graph view
                    app.chart_data.remove(0);
                    let new_val: f64 = (elapsed * 1.0).sin() * 5.0;
                    app.chart_data.push(new_val);
                    let data_points: Vec<(f64, f64)> = app.chart_data.iter().enumerate().map(|(i, &y)| (i as f64, y)).collect();
                    let dataset = Dataset::default().data(&data_points).style(Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD));
                    let chart = Chart::new(vec![dataset])
                        .block(Block::default().title(" Metrics Graph ").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(C_MAUVE)))
                        .x_axis(Axis::default().bounds([0.0, 200.0]).style(Style::default().fg(C_SURFACE)))
                        .y_axis(Axis::default().bounds([-5.0, 5.0]).style(Style::default().fg(C_SURFACE)));
                    f.render_widget(chart, right_chunk);
                }
                2 => {
                    // Status view


                    let p_bars = (app.api_status.limit_percent / 5.0).round() as usize;
                    let p_bars = p_bars.min(20);
                    let bar_str = format!("{}{} {}%", "█".repeat(p_bars), "░".repeat(20 - p_bars), app.api_status.limit_percent.round());

                    let mut status_text = vec![
                        Line::from(""),
                        Line::from(vec![Span::styled("  >_ LLM API Status", Style::default().fg(C_GREEN).add_modifier(Modifier::BOLD))]),
                        Line::from(format!("  Model:      {}", app.api_status.model_name)),
                        Line::from(format!("  Account:    {}", app.api_status.account_tier)),
                        Line::from(vec![Span::raw("  Usage:      "), Span::styled(format!("[{}] {}", bar_str, app.api_status.limit_info), Style::default().fg(C_GREEN))]),
                        Line::from(""),
                        Line::from(vec![Span::styled("  GitHub API", Style::default().add_modifier(Modifier::BOLD))]),
                        Line::from("  ────────────────────────────"),
                    ];

                    if let Some(cal) = &app.github_cal {
                        let now = chrono::Utc::now();
                        let reset_dt = chrono::DateTime::from_timestamp(cal.rate_limit_reset, 0).unwrap_or(now);
                        let reset_diff = reset_dt.signed_duration_since(now);
                        let mut reset_str = "Reset in past".to_string();
                        if reset_diff.num_seconds() > 0 {
                            let m = reset_diff.num_minutes();
                            let s = reset_diff.num_seconds() % 60;
                            reset_str = format!("{}m {}s", m, s);
                        }
                        
                        let fetched_dt = chrono::DateTime::parse_from_rfc3339(&cal.fetched_at).map(|dt| dt.with_timezone(&chrono::Utc)).unwrap_or(now);
                        let fetched_diff = now.signed_duration_since(fetched_dt);
                        let fetched_str = if fetched_diff.num_seconds() < 60 {
                            format!("{} sec ago", fetched_diff.num_seconds())
                        } else {
                            format!("{} min ago", fetched_diff.num_minutes())
                        };

                        status_text.push(Line::from(vec![Span::styled("  ● Connected", Style::default().fg(C_GREEN))]));
                        status_text.push(Line::from(""));
                        status_text.push(Line::from(vec![Span::styled("  GraphQL", Style::default().add_modifier(Modifier::BOLD))]));
                        status_text.push(Line::from(format!("    Limit:       {} / hour", cal.rate_limit_limit)));
                        status_text.push(Line::from(format!("    Remaining:   {}", cal.rate_limit_remaining)));
                        status_text.push(Line::from(format!("    Used:        {}", cal.rate_limit_used)));
                        status_text.push(Line::from(format!("    Reset:       {}", reset_str)));
                        status_text.push(Line::from(""));
                        status_text.push(Line::from(vec![Span::styled("  Contribution Calendar", Style::default().add_modifier(Modifier::BOLD))]));
                        status_text.push(Line::from(format!("    Last fetch:  {}", fetched_str)));
                        let cache_status = if fetched_diff.num_seconds() < 30 { "Valid" } else { "Stale" };
                        status_text.push(Line::from(vec![Span::raw("    Cache:       "), Span::styled(cache_status, Style::default().fg(C_YELLOW))]));
                    } else {
                        status_text.push(Line::from(vec![Span::styled("  ○ Disconnected / Auth Error", Style::default().fg(C_ROSE))]));
                    }
                    let status_widget = Paragraph::new(status_text)
                        .block(Block::default().title(" System Status ").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(C_MAUVE)));
                    f.render_widget(status_widget, right_chunk);
                }
                3 => {
                    // Usage view
                    let mut streak = 0;
                    if !app.stats.activity.is_empty() {
                        streak = 1;
                    }
                    
                    let mut usage_text = vec![
                        Line::from(vec![Span::styled(" Token activity", Style::default().add_modifier(Modifier::BOLD)), Span::raw("   last 12 months")]),
                        Line::from(format!(" Lifetime {} · Peak {} · Streak {}d · Longest task {:.1}s", 
                            app.stats.lifetime_tokens, app.stats.peak_tokens, streak, app.stats.longest_task)),
                        Line::from(""),
                        Line::from("      Sep     Oct     Nov       Dec     Jan     Feb     Mar       Apr     May       Jun     Jul     Aug"),
                    ];
                    
                    let mut grid = [0u8; 365];
                    for date_str in &app.stats.activity {
                        if date_str.len() >= 10 {
                            let m: usize = date_str[5..7].parse().unwrap_or(0);
                            let d: usize = date_str[8..10].parse().unwrap_or(0);
                            if m > 0 && d > 0 {
                                let day_of_year = (m - 1) * 30 + d;
                                if day_of_year < 365 {
                                    grid[day_of_year] += 1;
                                }
                            }
                        }
                    }

                    let days = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
                    for (row, day) in days.iter().enumerate() {
                        let mut spans = vec![Span::raw(format!(" {} ", day))];
                        for col in 0..52 {
                            let day_of_year = col * 7 + row;
                            let count = grid[day_of_year % 365];
                            let color = match count {
                                0 => Color::Rgb(22, 27, 34),     // #161b22
                                1..=5 => Color::Rgb(14, 68, 41), // #0e4429
                                6..=15 => Color::Rgb(0, 109, 50),// #006d32
                                16..=30 => Color::Rgb(38, 166, 65),// #26a641
                                _ => Color::Rgb(57, 211, 83),    // #39d353
                            };
                            spans.push(Span::styled("■ ", Style::default().fg(color)));
                        }
                        usage_text.push(Line::from(spans));
                    }
                    usage_text.push(Line::from(""));
                    usage_text.push(Line::from(vec![
                        Span::raw("   Less "),
                        Span::styled("■ ", Style::default().fg(Color::Rgb(22, 27, 34))),
                        Span::styled("■ ", Style::default().fg(Color::Rgb(14, 68, 41))),
                        Span::styled("■ ", Style::default().fg(Color::Rgb(0, 109, 50))),
                        Span::styled("■ ", Style::default().fg(Color::Rgb(38, 166, 65))),
                        Span::styled("■ ", Style::default().fg(Color::Rgb(57, 211, 83))),
                        Span::raw("More"),
                    ]));
                    usage_text.push(Line::from("   daily · weekly · cumulative"));
                    
                    let usage_widget = Paragraph::new(usage_text)
                        .block(Block::default().title(" Token Usage ").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(C_MAUVE)));
                    f.render_widget(usage_widget, right_chunk);
                }
                4 => {
                    // GitHub Activity View
                    let mut text = vec![
                        Line::from(vec![Span::styled(" GitHub Activity ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(" ↻ ")]),
                    ];
                    
                    if let Some(err) = &app.github_error {
                        text.push(Line::from(""));
                        text.push(Line::from(Span::styled(err, Style::default().fg(C_ROSE))));
                    } else if let Some(cal) = &app.github_cal {
                        text.push(Line::from(format!(" @{} · {} contributions in the last year", cal.username, cal.total_contributions)));
                        text.push(Line::from(""));
                        
                        let get_level_color = |level: &str| -> Color {
                            match level {
                                "FIRST_QUARTILE" => Color::Rgb(14, 68, 41),
                                "SECOND_QUARTILE" => Color::Rgb(0, 109, 50),
                                "THIRD_QUARTILE" => Color::Rgb(38, 166, 65),
                                "FOURTH_QUARTILE" => Color::Rgb(57, 211, 83),
                                _ => Color::Rgb(22, 27, 34),
                            }
                        };
                        
                        // Months
                        let mut month_line = vec![Span::raw("      ")];
                        for m in &cal.months {
                            let padding = " ".repeat((m.total_weeks as usize).saturating_sub(1) * 2);
                            month_line.push(Span::raw(format!("{}{}", &m.name[0..3], padding)));
                        }
                        text.push(Line::from(month_line));
                        
                        let days = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
                        let day_indices = [0, 1, 2, 3, 4, 5, 6];
                        for (i, row_day) in day_indices.iter().enumerate() {
                            let mut spans = vec![Span::raw(format!(" {} ", days[i]))];
                            for w in &cal.weeks {
                                let mut found = false;
                                for d in &w.days {
                                    if d.weekday == *row_day {
                                        spans.push(Span::styled("■ ", Style::default().fg(get_level_color(&d.level))));
                                        found = true;
                                        break;
                                    }
                                }
                                if !found {
                                    spans.push(Span::raw("  "));
                                }
                            }
                            text.push(Line::from(spans));
                        }
                        text.push(Line::from(""));
                        
                        // Legend
                        let legend = vec![
                            Span::raw("   Less "),
                            Span::styled("■ ", Style::default().fg(Color::Rgb(22, 27, 34))),
                            Span::styled("■ ", Style::default().fg(Color::Rgb(14, 68, 41))),
                            Span::styled("■ ", Style::default().fg(Color::Rgb(0, 109, 50))),
                            Span::styled("■ ", Style::default().fg(Color::Rgb(38, 166, 65))),
                            Span::styled("■ ", Style::default().fg(Color::Rgb(57, 211, 83))),
                            Span::raw("More")
                        ];
                        text.push(Line::from(legend));
                        
                        // Stats
                        text.push(Line::from(""));
                        let mut max_streak = 0;
                        let mut current = 0;
                        for w in &cal.weeks {
                            for d in &w.days {
                                if d.count > 0 {
                                    current += 1;
                                    if current > max_streak { max_streak = current; }
                                } else {
                                    current = 0;
                                }
                            }
                        }
                        text.push(Line::from(format!("  🔥 Longest Streak: {} days", max_streak)));
                    } else {
                        text.push(Line::from(""));
                        text.push(Line::from(Span::styled("██████████████████████", Style::default().fg(C_GRAY))));
                        text.push(Line::from(Span::styled("██████████████████████", Style::default().fg(C_GRAY))));
                        text.push(Line::from(" Loading..."));
                    }
                    
                    let widget = Paragraph::new(text)
                        .block(Block::default().title(" GitHub Contributions ").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(C_MAUVE)));
                    f.render_widget(widget, right_chunk);
                }
                5 => {
                    if let Some(g_stats) = &app.groq_stats {
                        let chunks = ratatui::layout::Layout::default()
                            .direction(ratatui::layout::Direction::Vertical)
                            .constraints([ratatui::layout::Constraint::Length(12), ratatui::layout::Constraint::Min(5)].as_ref())
                            .split(right_chunk);

                        let top_chunks = ratatui::layout::Layout::default()
                            .direction(ratatui::layout::Direction::Horizontal)
                            .constraints([ratatui::layout::Constraint::Percentage(50), ratatui::layout::Constraint::Percentage(50)].as_ref())
                            .split(chunks[0]);

                        let mut days: Vec<(&String, &GroqDailyStats)> = g_stats.daily.iter().collect();
                        days.sort_by(|a, b| a.0.cmp(b.0));
                        let recent_days: Vec<_> = days.iter().rev().take(7).rev().collect();
                        
                        let mut cost_bars = Vec::new();
                        let mut token_bars = Vec::new();
                        let mut labels = Vec::new(); 
                        
                        for (d, _) in &recent_days {
                            let label = if d.len() >= 10 { d[5..10].to_string() } else { (*d).clone() };
                            labels.push(label);
                        }
                        
                        for (i, (_, s)) in recent_days.iter().enumerate() {
                            let cost_cents = (s.cost * 1000.0) as u64; 
                            cost_bars.push((labels[i].as_str(), cost_cents));
                            
                            let tokens = (s.tokens / 1000) as u64;
                            token_bars.push((labels[i].as_str(), tokens));
                        }

                        let barchart_cost = ratatui::widgets::BarChart::default()
                            .block(Block::default().title(" Daily Cost (m$) ").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(C_ROSE)))
                            .data(&cost_bars)
                            .bar_width(5)
                            .bar_gap(1)
                            .bar_style(Style::default().fg(C_ROSE))
                            .value_style(Style::default().fg(C_SURFACE).bg(C_ROSE));

                        let barchart_tokens = ratatui::widgets::BarChart::default()
                            .block(Block::default().title(" Daily Tokens (k) ").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(C_GREEN)))
                            .data(&token_bars)
                            .bar_width(5)
                            .bar_gap(1)
                            .bar_style(Style::default().fg(C_GREEN))
                            .value_style(Style::default().fg(C_SURFACE).bg(C_GREEN));

                        f.render_widget(barchart_cost, top_chunks[0]);
                        f.render_widget(barchart_tokens, top_chunks[1]);

                        let mut model_lines = vec![
                            Line::from(vec![Span::styled("  >_ Model Usage Breakdown", Style::default().fg(C_MAUVE).add_modifier(Modifier::BOLD))]),
                            Line::from(format!("  Total Spend: ${:.4}", g_stats.total_cost)),
                            Line::from(""),
                        ];
                        
                        let mut models: Vec<_> = g_stats.models.iter().collect();
                        models.sort_by(|a, b| b.1.cost.partial_cmp(&a.1.cost).unwrap_or(std::cmp::Ordering::Equal));
                        
                        for (m, s) in models {
                            let bar_len = ((s.cost / g_stats.total_cost.max(0.0001)) * 20.0).round() as usize;
                            let bar_str = format!("{}{} ${:.4}", "█".repeat(bar_len), "░".repeat(20usize.saturating_sub(bar_len)), s.cost);
                            model_lines.push(Line::from(vec![
                                Span::styled(format!("  {:<20} ", m), Style::default().add_modifier(Modifier::BOLD)),
                                Span::styled(bar_str, Style::default().fg(C_MAUVE)),
                                Span::raw(format!("   ({} runs, {} tokens)", s.runs, s.total_tokens))
                            ]));
                        }

                        let model_widget = Paragraph::new(model_lines)
                            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(C_LAVENDER)));
                        
                        f.render_widget(model_widget, chunks[1]);
                    } else {
                        let msg = Paragraph::new("No Groq analytics data found. Run `mig run` to generate stats.")
                            .alignment(Alignment::Center)
                            .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded));
                        f.render_widget(msg, right_chunk);
                    }
                }
                _ => {
                    // Commits view
                    let commits: Vec<ListItem> = app.recent_commits.iter().map(|c| {
                        ListItem::new(Line::from(Span::styled(c, Style::default().fg(C_LAVENDER))))
                    }).collect();
                    let commits_list = List::new(commits)
                        .block(Block::default().title(" Activity Feed ").borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(C_MAUVE)));
                    f.render_widget(commits_list, right_chunk);
                }
            }

            let graph_btn_style = if app.focus_idx == 0 { Style::default().fg(C_MAUVE).add_modifier(Modifier::BOLD).bg(C_SURFACE) } else { Style::default().fg(C_MAUVE) };
            
            let btn_label = match app.right_panel_view {
                0 => " Show Graph ",
                1 => " Show Status ",
                2 => " Show Usage ",
                3 => " Show GitHub ",
                4 => " Show Groq ",
                4 => " Show Groq ",
                _ => " Show Commits ",
            };

            let graph_btn = Paragraph::new(btn_label)
                .style(graph_btn_style).alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(C_MAUVE)));
            f.render_widget(graph_btn, graph_btn_rect);

            let quit_btn_style = if app.focus_idx == 1 { Style::default().fg(C_ROSE).add_modifier(Modifier::BOLD).bg(C_SURFACE) } else { Style::default().fg(C_ROSE) };
            let quit_btn = Paragraph::new(" Quit ")
                .style(quit_btn_style).alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(C_ROSE)));
            f.render_widget(quit_btn, quit_btn_rect);
            // --- Hover Tooltip Logic ---
            if let Some((mx, my)) = app.mouse_pos {
                let mut tooltip_text = None;
                
                if app.right_panel_view == 3 {
                    // Token Usage
                    let top_margin = right_chunk.y + 5;
                    let left_margin = right_chunk.x + 5;
                    if mx >= left_margin && my >= top_margin && my < top_margin + 7 {
                        let d = my - top_margin;
                        let dx = mx - left_margin;
                        let w = dx / 2;
                        if w < 52 {
                            let day_of_year = w * 7 + d;
                            // Calculate date backwards
                                                        // Re-calculate grid locally
                            let mut grid = [0u8; 365];
                            for date_str in &app.stats.activity {
                                if date_str.len() >= 10 {
                                    let m: usize = date_str[5..7].parse().unwrap_or(0);
                                    let d_val: usize = date_str[8..10].parse().unwrap_or(0);
                                    if m > 0 && d_val > 0 {
                                        let doy = (m - 1) * 30 + d_val;
                                        if doy < 365 { grid[doy] += 1; }
                                    }
                                }
                            }
                            let count = grid[(day_of_year as usize) % 365];
                            tooltip_text = Some(format!(" {} requests ", count));
                        }
                    }
                } else if app.right_panel_view == 4 {
                    // GitHub Activity
                    if let Some(cal) = &app.github_cal {
                        let top_margin = right_chunk.y + 5;
                        let left_margin = right_chunk.x + 6;
                        if mx >= left_margin && my >= top_margin && my < top_margin + 7 {
                            let d = my - top_margin;
                            let dx = mx - left_margin;
                            let w = dx / 2;
                            if (w as usize) < cal.weeks.len() {
                                let week = &cal.weeks[w as usize];
                                if let Some(day) = week.days.iter().find(|x| x.weekday == d as u8) {
                                    let count_str = if day.count == 1 { "1 contribution".to_string() } else { format!("{} contributions", day.count) };
                                    tooltip_text = Some(format!(" {} : {} ", day.date, count_str));
                                }
                            }
                        }
                    }
                }
                
                if let Some(text) = tooltip_text {
                    let text_len = text.len() as u16;
                    let mut tx = mx + 1;
                    let ty = my.saturating_sub(1);
                    if tx + text_len > size.width {
                        tx = mx.saturating_sub(text_len + 1);
                    }
                    let t_rect = Rect { x: tx, y: ty, width: text_len, height: 1 };
                    let t_widget = Paragraph::new(Span::styled(text, Style::default().fg(C_SURFACE).bg(C_LAVENDER)));
                    use ratatui::widgets::Clear;
                    f.render_widget(Clear, t_rect);
                    f.render_widget(t_widget, t_rect);
                }
            }
        })?;

        if event::poll(tick_rate)? {
            match event::read()? {
                Event::Key(key) => {
                    match key.code {
                        KeyCode::Up | KeyCode::Down => {
                            app.focus_idx = if app.focus_idx == 0 { 1 } else { 0 };
                        }
                        KeyCode::Left => {
                            app.prev_tab();
                        }
                        KeyCode::Right => {
                            app.next_tab();
                        }
                        KeyCode::Tab => {
                            app.next_tab();
                        }
                        KeyCode::Enter => {
                            if app.focus_idx == 1 {
                                return Ok(());
                            } else {
                                app.right_panel_view = (app.right_panel_view + 1) % 6;
                            }
                        }
                                                KeyCode::Char('r') => {
                            if let Ok(rt) = tokio::runtime::Runtime::new() {
                                rt.block_on(async {
                                    let svc = GitHubContributionService::new();
                                    if let (Some(username), Some(token)) = (std::env::var("GITHUB_USERNAME").ok(), std::env::var("GITHUB_TOKEN").ok()) {
                                        match svc.get_contributions(&username, &token, true).await {
                                            Ok(cal) => app.github_cal = Some(cal),
                                            Err(e) => app.github_error = Some(e),
                                        }
                                    }
                                });
                            }
                        }
                        KeyCode::Char('q') => {
                            return Ok(());
                        }
                        _ => {}
                    }
                }
                                Event::Mouse(mouse_event) => {
                    app.mouse_pos = Some((mouse_event.column, mouse_event.row));
                    if mouse_event.kind == MouseEventKind::Down(MouseButton::Left) {
                        let col = mouse_event.column;
                        let row = mouse_event.row;


                        if is_inside(graph_btn_rect, col, row) {
                            app.right_panel_view = (app.right_panel_view + 1) % 6;
                        } else if is_inside(quit_btn_rect, col, row) {
                            return Ok(());
                        } else if is_inside(tab_0_rect, col, row) && app.keys_status[0] {
                            app.active_tab = 0;
                        } else if is_inside(tab_1_rect, col, row) && app.keys_status[1] {
                            app.active_tab = 1;
                        } else if is_inside(tab_2_rect, col, row) && app.keys_status[2] {
                            app.active_tab = 2;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
