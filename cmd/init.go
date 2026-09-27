package cmd

import (
	"fmt"
	"os"

	"github.com/Nissmo89/MIG/internal/git"
	"github.com/Nissmo89/MIG/internal/stats"
	"github.com/spf13/cobra"
)

var initCmd = &cobra.Command{
	Use:   "init",
	Short: "Initialize a git repository if not already done, and create a Context.md",
	RunE: func(cmd *cobra.Command, args []string) error {
		if !git.IsRepo(".") {
			if err := git.InitRepo("."); err != nil {
				return err
			}
			fmt.Println("Initialized empty Git repository.")
		} else {
			fmt.Println("Git repository already exists.")
		}

		// Track project in ~/.mig/projects.json
		if cwd, err := os.Getwd(); err == nil {
			_ = stats.TrackProject(cwd)
		}

		contextFile := "Context.md"
		if _, err := os.Stat(contextFile); os.IsNotExist(err) {
			defaultContext := `# MIG Personality Context

You are an auto-contributing bot. Generate small Python scripts.
`
			if err := os.WriteFile(contextFile, []byte(defaultContext), 0644); err != nil {
				return fmt.Errorf("failed to create Context.md: %w", err)
			}
			fmt.Println("Created default Context.md.")
		} else {
			fmt.Println("Context.md already exists.")
		}

		return nil
	},
}

func init() {
	rootCmd.AddCommand(initCmd)
}
