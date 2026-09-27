package git

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

// IsRepo checks if directory contains a .git folder
func IsRepo(dir string) bool {
	if dir == "" {
		dir = "."
	}
	gitPath := filepath.Join(dir, ".git")
	fi, err := os.Stat(gitPath)
	return err == nil && (fi.IsDir() || !fi.IsDir()) // .git can be a file in worktrees/submodules
}

// InitRepo runs git init in the directory
func InitRepo(dir string) error {
	cmd := exec.Command("git", "init")
	if dir != "" {
		cmd.Dir = dir
	}
	output, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("git init failed: %s: %w", string(output), err)
	}
	return nil
}

// IsDirty checks if there are untracked or modified files
func IsDirty(dir string) (bool, error) {
	cmd := exec.Command("git", "status", "--porcelain")
	if dir != "" {
		cmd.Dir = dir
	}
	output, err := cmd.Output()
	if err != nil {
		return false, fmt.Errorf("git status failed: %w", err)
	}
	return len(bytes.TrimSpace(output)) > 0, nil
}

// StageFiles stages specified files
func StageFiles(dir string, files ...string) error {
	if len(files) == 0 {
		return nil
	}
	args := append([]string{"add"}, files...)
	cmd := exec.Command("git", args...)
	if dir != "" {
		cmd.Dir = dir
	}
	output, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("git add failed: %s: %w", string(output), err)
	}
	return nil
}

// Commit creates a git commit with the given message
func Commit(dir, message string) error {
	cmd := exec.Command("git", "commit", "-m", message)
	if dir != "" {
		cmd.Dir = dir
	}
	output, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("git commit failed: %s: %w", string(output), err)
	}
	return nil
}

// Push pushes committed changes to remote
func Push(dir, remote string) error {
	if remote == "" {
		remote = "origin"
	}
	cmd := exec.Command("git", "push", remote)
	if dir != "" {
		cmd.Dir = dir
	}
	output, err := cmd.CombinedOutput()
	if err != nil {
		// Try generic git push without explicit remote if origin fails
		fallbackCmd := exec.Command("git", "push")
		if dir != "" {
			fallbackCmd.Dir = dir
		}
		if fbOut, fbErr := fallbackCmd.CombinedOutput(); fbErr != nil {
			return fmt.Errorf("git push failed: %s: %w", string(output), err)
		} else {
			_ = fbOut
			return nil
		}
	}
	return nil
}

// GetRecentCommits retrieves recent commits from a git repo
func GetRecentCommits(dir string, count int) ([]string, error) {
	if count <= 0 {
		count = 3
	}
	cmd := exec.Command("git", "log", fmt.Sprintf("-n%d", count), "--pretty=format:%h - %s (%cr)")
	if dir != "" {
		cmd.Dir = dir
	}
	output, err := cmd.Output()
	if err != nil {
		return nil, err
	}

	lines := strings.Split(strings.TrimSpace(string(output)), "\n")
	var commits []string
	repoName := filepath.Base(dir)
	if repoName == "" || repoName == "." {
		if abs, err := filepath.Abs(dir); err == nil {
			repoName = filepath.Base(abs)
		} else {
			repoName = "repo"
		}
	}

	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		if trimmed != "" {
			commits = append(commits, fmt.Sprintf("[%s] %s", repoName, trimmed))
		}
	}
	return commits, nil
}
