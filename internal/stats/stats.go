package stats

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"
)

var mu sync.Mutex

type Stats struct {
	LifetimeTokens int64    `json:"lifetime_tokens"`
	PeakTokens     int64    `json:"peak_tokens"`
	LongestTask    float64  `json:"longest_task"`
	Activity       []string `json:"activity"`
}

type ModelStats struct {
	TotalTokens int64   `json:"total_tokens"`
	Cost        float64 `json:"cost"`
	Runs        int     `json:"runs"`
}

type DailyStats struct {
	Tokens int64   `json:"tokens"`
	Cost   float64 `json:"cost"`
}

type GroqAnalytics struct {
	TotalCost float64                `json:"total_cost"`
	Models    map[string]*ModelStats `json:"models"`
	Daily     map[string]*DailyStats `json:"daily"`
}

func GetMigDir() (string, error) {
	home, err := os.UserHomeDir()
	if err != nil {
		return "", err
	}
	migDir := filepath.Join(home, ".mig")
	if err := os.MkdirAll(migDir, 0755); err != nil {
		return "", err
	}
	return migDir, nil
}

// TrackProject registers a project directory path in ~/.mig/projects.json
func TrackProject(path string) error {
	mu.Lock()
	defer mu.Unlock()

	migDir, err := GetMigDir()
	if err != nil {
		return err
	}

	projFile := filepath.Join(migDir, "projects.json")
	projects := []string{}

	if data, err := os.ReadFile(projFile); err == nil {
		_ = json.Unmarshal(data, &projects)
	}

	for _, p := range projects {
		if strings.EqualFold(filepath.Clean(p), filepath.Clean(path)) {
			return nil
		}
	}

	projects = append(projects, filepath.Clean(path))
	data, err := json.MarshalIndent(projects, "", "  ")
	if err != nil {
		return err
	}

	return os.WriteFile(projFile, data, 0644)
}

// GetProjects returns all tracked projects from ~/.mig/projects.json
func GetProjects() ([]string, error) {
	mu.Lock()
	defer mu.Unlock()

	migDir, err := GetMigDir()
	if err != nil {
		return nil, err
	}

	projFile := filepath.Join(migDir, "projects.json")
	if _, err := os.Stat(projFile); os.IsNotExist(err) {
		return []string{}, nil
	}

	data, err := os.ReadFile(projFile)
	if err != nil {
		return nil, err
	}

	var projects []string
	if err := json.Unmarshal(data, &projects); err != nil {
		return []string{}, nil
	}
	return projects, nil
}

// GetStats loads global stats from ~/.mig/stats.json
func GetStats() (*Stats, error) {
	mu.Lock()
	defer mu.Unlock()

	migDir, err := GetMigDir()
	if err != nil {
		return &Stats{Activity: []string{}}, nil
	}

	statsFile := filepath.Join(migDir, "stats.json")
	st := &Stats{Activity: []string{}}

	if data, err := os.ReadFile(statsFile); err == nil {
		_ = json.Unmarshal(data, st)
	}
	if st.Activity == nil {
		st.Activity = []string{}
	}
	return st, nil
}

// UpdateStats updates ~/.mig/stats.json
func UpdateStats(totalTokens int64, durationSeconds float64) error {
	mu.Lock()
	defer mu.Unlock()

	migDir, err := GetMigDir()
	if err != nil {
		return err
	}

	statsFile := filepath.Join(migDir, "stats.json")
	st := &Stats{Activity: []string{}}

	if data, err := os.ReadFile(statsFile); err == nil {
		_ = json.Unmarshal(data, st)
	}
	if st.Activity == nil {
		st.Activity = []string{}
	}

	st.LifetimeTokens += totalTokens
	if totalTokens > st.PeakTokens {
		st.PeakTokens = totalTokens
	}
	if durationSeconds > st.LongestTask {
		st.LongestTask = durationSeconds
	}
	st.Activity = append(st.Activity, time.Now().Format("2006-01-02"))

	data, err := json.Marshal(st)
	if err != nil {
		return err
	}
	return os.WriteFile(statsFile, data, 0644)
}

// GetGroqAnalytics loads ~/.mig/groq_analytics.json
func GetGroqAnalytics() (*GroqAnalytics, error) {
	mu.Lock()
	defer mu.Unlock()

	migDir, err := GetMigDir()
	if err != nil {
		return &GroqAnalytics{Models: make(map[string]*ModelStats), Daily: make(map[string]*DailyStats)}, nil
	}

	analyticsFile := filepath.Join(migDir, "groq_analytics.json")
	analytics := &GroqAnalytics{
		Models: make(map[string]*ModelStats),
		Daily:  make(map[string]*DailyStats),
	}

	if data, err := os.ReadFile(analyticsFile); err == nil {
		_ = json.Unmarshal(data, analytics)
	}
	if analytics.Models == nil {
		analytics.Models = make(map[string]*ModelStats)
	}
	if analytics.Daily == nil {
		analytics.Daily = make(map[string]*DailyStats)
	}
	return analytics, nil
}

// UpdateGroqAnalytics updates ~/.mig/groq_analytics.json
func UpdateGroqAnalytics(model string, totalTokens int64, cost float64) error {
	mu.Lock()
	defer mu.Unlock()

	migDir, err := GetMigDir()
	if err != nil {
		return err
	}

	analyticsFile := filepath.Join(migDir, "groq_analytics.json")
	analytics := &GroqAnalytics{
		Models: make(map[string]*ModelStats),
		Daily:  make(map[string]*DailyStats),
	}

	if data, err := os.ReadFile(analyticsFile); err == nil {
		_ = json.Unmarshal(data, analytics)
	}
	if analytics.Models == nil {
		analytics.Models = make(map[string]*ModelStats)
	}
	if analytics.Daily == nil {
		analytics.Daily = make(map[string]*DailyStats)
	}

	analytics.TotalCost += cost

	if m, ok := analytics.Models[model]; ok {
		m.TotalTokens += totalTokens
		m.Cost += cost
		m.Runs++
	} else {
		analytics.Models[model] = &ModelStats{
			TotalTokens: totalTokens,
			Cost:        cost,
			Runs:        1,
		}
	}

	today := time.Now().Format("2006-01-02")
	if d, ok := analytics.Daily[today]; ok {
		d.Tokens += totalTokens
		d.Cost += cost
	} else {
		analytics.Daily[today] = &DailyStats{
			Tokens: totalTokens,
			Cost:   cost,
		}
	}

	data, err := json.Marshal(analytics)
	if err != nil {
		return err
	}
	return os.WriteFile(analyticsFile, data, 0644)
}

// CalculateCost calculates price based on model token counts
func CalculateCost(model string, inputTokens, outputTokens int64) float64 {
	prices := map[string][2]float64{
		"llama3-8b-8192":        {0.05, 0.08},
		"llama-3.1-8b-instant":  {0.05, 0.08},
		"llama3-70b-8192":       {0.59, 0.79},
		"llama-3.1-70b-versatile": {0.59, 0.79},
		"mixtral-8x7b-32768":    {0.24, 0.24},
		"gemma-7b-it":           {0.07, 0.07},
		"gemma2-9b-it":          {0.20, 0.20},
		"gemini-2.5-flash-lite": {0.075, 0.30},
		"gemini-1.5-flash":      {0.075, 0.30},
	}

	norm := strings.ToLower(model)
	rates, ok := prices[norm]
	if !ok {
		rates = [2]float64{0.59, 0.79}
	}

	return (float64(inputTokens)/1_000_000.0)*rates[0] + (float64(outputTokens)/1_000_000.0)*rates[1]
}

// GetLanguageCounts reads count.json from the given directory (or cwd)
func GetLanguageCounts(repoDir string) (map[string]int, error) {
	mu.Lock()
	defer mu.Unlock()

	countFile := filepath.Join(repoDir, "count.json")
	counts := make(map[string]int)

	if data, err := os.ReadFile(countFile); err == nil {
		_ = json.Unmarshal(data, &counts)
	}
	return counts, nil
}

// IncrementLanguageCount increments the count for a language in count.json
func IncrementLanguageCount(repoDir, language string) error {
	mu.Lock()
	defer mu.Unlock()

	countFile := filepath.Join(repoDir, "count.json")
	counts := make(map[string]int)

	if data, err := os.ReadFile(countFile); err == nil {
		_ = json.Unmarshal(data, &counts)
	}

	lang := strings.ToLower(strings.TrimSpace(language))
	if lang == "" || lang == "." {
		lang = "unknown"
	}

	counts[lang]++

	data, err := json.MarshalIndent(counts, "", "    ")
	if err != nil {
		return err
	}
	return os.WriteFile(countFile, data, 0644)
}
