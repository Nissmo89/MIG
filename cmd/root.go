package cmd

import (
	"fmt"
	"os"

	"github.com/joho/godotenv"
	"github.com/spf13/cobra"
)

var rootCmd = &cobra.Command{
	Use:   "mig",
	Short: "MIG (Make It Green) - Standalone Auto Contributing Bot",
	Long: `MIG (Make It Green) is an autonomous, standalone CLI tool that helps you
keep your GitHub contribution graph active by generating clever algorithms and
committing them directly to your repository daily.`,
}

func Execute() {
	if err := rootCmd.Execute(); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}

func init() {
	// Auto-load .env from current directory or parent directory
	_ = godotenv.Load()
	_ = godotenv.Load("../.env")
}
