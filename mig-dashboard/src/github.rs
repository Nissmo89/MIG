use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use chrono::{Utc, Duration};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionDay {
    pub date: String,
    pub count: u32,
    pub level: String,
    pub color: String,
    pub weekday: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionWeek {
    pub first_day: String,
    pub days: Vec<ContributionDay>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionMonth {
    pub name: String,
    pub year: i32,
    pub first_day: String,
    pub total_weeks: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributionCalendar {
    pub total_contributions: u32,
    pub colors: Vec<String>,
    pub months: Vec<ContributionMonth>,
    pub weeks: Vec<ContributionWeek>,
    pub username: String,
    pub fetched_at: String,
}

pub struct GitHubContributionService {
    client: Client,
    cache_dir: PathBuf,
}

impl GitHubContributionService {
    pub fn new() -> Self {
        let mut cache_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        cache_dir.push(".mig");
        fs::create_dir_all(&cache_dir).ok();
        
        Self {
            client: Client::new(),
            cache_dir,
        }
    }

    pub async fn get_contributions(&self, username: &str, token: &str, force_refresh: bool) -> Result<ContributionCalendar, String> {
        let cache_file = self.cache_dir.join(format!("{}_github_cache.json", username));
        
        // Check cache first
        if !force_refresh && cache_file.exists() {
            if let Ok(content) = fs::read_to_string(&cache_file) {
                if let Ok(cached) = serde_json::from_str::<ContributionCalendar>(&content) {
                    if let Ok(fetched) = chrono::DateTime::parse_from_rfc3339(&cached.fetched_at) {
                        let now = Utc::now();
                        // Cache for 60 minutes
                        if now.signed_duration_since(fetched.with_timezone(&Utc)).num_minutes() < 60 {
                            return Ok(cached);
                        }
                    }
                }
            }
        }

        // Generate date ranges dynamically
        let to = Utc::now();
        let from = to - Duration::days(365);
        
        let query = r#"
            query ContributionCalendar($username: String!, $from: DateTime, $to: DateTime) {
              user(login: $username) {
                contributionsCollection(from: $from, to: $to) {
                  contributionCalendar {
                    totalContributions
                    colors
                    months { name year firstDay totalWeeks }
                    weeks {
                      firstDay
                      contributionDays { date contributionCount contributionLevel color weekday }
                    }
                  }
                }
              }
            }
        "#;
        
        let req_body = json!({
            "query": query,
            "variables": {
                "username": username,
                "from": from.to_rfc3339(),
                "to": to.to_rfc3339(),
            }
        });
        
        let resp = self.client.post("https://api.github.com/graphql")
            .header("Authorization", format!("Bearer {}", token))
            .header("User-Agent", "mig-dashboard")
            .json(&req_body)
            .send()
            .await
            .map_err(|e| format!("Network error: {}", e))?;
            
        let status = resp.status();
        let json_resp: serde_json::Value = resp.json().await.map_err(|e| format!("JSON parsing error: {}", e))?;
        
        if !status.is_success() {
            if let Some(msg) = json_resp.get("message").and_then(|m| m.as_str()) {
                return Err(format!("GitHub Auth Error: {}", msg));
            }
            return Err(format!("HTTP Error: {}", status));
        }
        
        if let Some(errors) = json_resp.get("errors") {
            return Err(format!("GraphQL Error: {}", errors));
        }
        
        let user = json_resp.get("data")
            .and_then(|d| d.get("user"))
            .ok_or_else(|| format!("GitHub user '{}' was not found.", username))?;
            
        let calendar_val = user.get("contributionsCollection")
            .and_then(|c| c.get("contributionCalendar"))
            .ok_or_else(|| "Missing contributionCalendar in response.".to_string())?;
            
        let total_contributions = calendar_val.get("totalContributions").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let colors: Vec<String> = calendar_val.get("colors")
            .and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|s| s.as_str().map(String::from)).collect())
            .unwrap_or_default();
            
        let mut months = Vec::new();
        if let Some(months_arr) = calendar_val.get("months").and_then(|v| v.as_array()) {
            for m in months_arr {
                months.push(ContributionMonth {
                    name: m.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    year: m.get("year").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                    first_day: m.get("firstDay").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    total_weeks: m.get("totalWeeks").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                });
            }
        }
        
        let mut weeks = Vec::new();
        if let Some(weeks_arr) = calendar_val.get("weeks").and_then(|v| v.as_array()) {
            for w in weeks_arr {
                let first_day = w.get("firstDay").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let mut days = Vec::new();
                if let Some(days_arr) = w.get("contributionDays").and_then(|v| v.as_array()) {
                    for d in days_arr {
                        days.push(ContributionDay {
                            date: d.get("date").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            count: d.get("contributionCount").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                            level: d.get("contributionLevel").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            color: d.get("color").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                            weekday: d.get("weekday").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
                        });
                    }
                }
                weeks.push(ContributionWeek { first_day, days });
            }
        }
        
        let cal = ContributionCalendar {
            total_contributions,
            colors,
            months,
            weeks,
            username: username.to_string(),
            fetched_at: Utc::now().to_rfc3339(),
        };
        
        // Cache the result
        if let Ok(json_str) = serde_json::to_string(&cal) {
            fs::write(&cache_file, json_str).ok();
        }
        
        Ok(cal)
    }
}
