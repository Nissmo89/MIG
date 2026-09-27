package bot

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/Nissmo89/MIG/internal/bot/llm"
	"github.com/Nissmo89/MIG/internal/git"
	"github.com/Nissmo89/MIG/internal/memory"
	"github.com/Nissmo89/MIG/internal/stats"
)

type RunResult struct {
	Filename      string  `json:"filename"`
	CommitMessage string  `json:"commit_message"`
	Language      string  `json:"language"`
	Model         string  `json:"model"`
	Duration      float64 `json:"duration_seconds"`
	TotalTokens   int64   `json:"total_tokens"`
	Cost          float64 `json:"cost"`
}

// LoadContext loads personality context from Context.md in the current directory
func LoadContext() string {
	contextPath := filepath.Join(".", "Context.md")
	if data, err := os.ReadFile(contextPath); err == nil {
		return string(data)
	}
	return "You are an auto-contributing bot."
}

// RunBot executes a contribution run: prompts LLM, saves code, updates stats & git
func RunBot() (*RunResult, error) {
	startTime := time.Now()

	// Track current repository
	if cwd, err := os.Getwd(); err == nil {
		_ = stats.TrackProject(cwd)
	}

	client, err := llm.GetClient()
	if err != nil {
		return nil, fmt.Errorf("failed to initialize LLM client: %w", err)
	}

	// Connect to SQLite memory
	memStore, err := memory.Open("mig_memory.sqlite")
	if err != nil {
		fmt.Printf("Warning: SQLite memory open error: %v\n", err)
	} else {
		defer memStore.Close()
	}

	var historySnippet string
	if memStore != nil {
		if recent, err := memStore.GetRecentContributions(8); err == nil && len(recent) > 0 {
			historySnippet = "\n\nPast contributions to avoid repeating:\n"
			for _, c := range recent {
				historySnippet += fmt.Sprintf("- %s (%s)\n", c.Filename, c.CommitMessage)
			}
		}
	}

	contextContent := LoadContext()
	systemPrompt := fmt.Sprintf("%s\n\nOutput ONLY a JSON block (and nothing else) with keys: 'filename', 'commit_message', 'code'. The 'code' should be the raw code string.", contextContent)
	userPrompt := "It's a new day! Please generate a new, clever DSA code snippet or micro-algorithm. Pick ANY language (Rust, C, C++, Python, Java, Go). Write purely hand-written style code (no AI boilerplate, no defensive null checks). Return the exact relative path using a language folder (e.g. `rust/trick.rs`), the commit message, and the code block." + historySnippet

	fmt.Printf("Calling %s (%s)...\n", client.GetProviderName(), client.GetModelName())
	resp, err := client.Generate(context.Background(), systemPrompt, userPrompt)
	if err != nil {
		return nil, fmt.Errorf("generation failed: %w", err)
	}

	duration := time.Since(startTime).Seconds()
	cost := stats.CalculateCost(resp.ModelName, resp.InputTokens, resp.OutputTokens)

	// Update stats
	_ = stats.UpdateStats(resp.TotalTokens, duration)
	_ = stats.UpdateGroqAnalytics(resp.ModelName, resp.TotalTokens, cost)

	// Save code to designated file
	cleanFilename := filepath.Clean(resp.Filename)
	dir := filepath.Dir(cleanFilename)
	if dir != "" && dir != "." {
		if err := os.MkdirAll(dir, 0755); err != nil {
			return nil, fmt.Errorf("failed to create directory %s: %w", dir, err)
		}
	}

	if err := os.WriteFile(cleanFilename, []byte(resp.Code), 0644); err != nil {
		return nil, fmt.Errorf("failed to write file %s: %w", cleanFilename, err)
	}
	fmt.Printf("Generated %s\n", cleanFilename)

	// Determine language from folder
	langFolder := strings.ToLower(dir)
	if langFolder == "" || langFolder == "." {
		langFolder = "unknown"
	}

	// Update count.json
	_ = stats.IncrementLanguageCount(".", langFolder)

	// Save memory
	if memStore != nil {
		_ = memStore.SaveContribution(cleanFilename, resp.CommitMessage, langFolder, resp.Code)
	}

	// Stage, commit, push
	if git.IsRepo(".") {
		if err := git.StageFiles(".", cleanFilename, "count.json"); err != nil {
			fmt.Printf("Warning: git add error: %v\n", err)
		} else {
			if err := git.Commit(".", resp.CommitMessage); err != nil {
				fmt.Printf("Warning: git commit error: %v\n", err)
			} else {
				fmt.Println("Committed changes.")
				if err := git.Push(".", "origin"); err != nil {
					fmt.Printf("Warning: git push error: %v\n", err)
				} else {
					fmt.Println("Successfully pushed to remote!")
				}
			}
		}
	}

	return &RunResult{
		Filename:      cleanFilename,
		CommitMessage: resp.CommitMessage,
		Language:      langFolder,
		Model:         resp.ModelName,
		Duration:      duration,
		TotalTokens:   resp.TotalTokens,
		Cost:          cost,
	}, nil
}
