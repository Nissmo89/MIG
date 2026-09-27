package cmd

import (
	"fmt"
	"os"

	"github.com/Nissmo89/MIG/internal/bot"
	"github.com/Nissmo89/MIG/internal/stats"
	"github.com/spf13/cobra"
)

var runCmd = &cobra.Command{
	Use:   "run",
	Short: "Run the bot to generate and commit an autonomous contribution",
	RunE: func(cmd *cobra.Command, args []string) error {
		fmt.Println("Running MIG...")

		if cwd, err := os.Getwd(); err == nil {
			_ = stats.TrackProject(cwd)
		}

		result, err := bot.RunBot()
		if err != nil {
			return fmt.Errorf("error running MIG: %w", err)
		}

		fmt.Printf("MIG finished successfully! Generated: %s (%s)\n", result.Filename, result.CommitMessage)
		return nil
	},
}

func init() {
	rootCmd.AddCommand(runCmd)
}
