package cmd

import (
	"fmt"
	"os"

	"github.com/spf13/cobra"
)

var cronCmd = &cobra.Command{
	Use:   "install-cron",
	Short: "Print instructions for scheduling MIG daily via cron or Windows Task Scheduler",
	Run: func(cmd *cobra.Command, args []string) {
		cwd, err := os.Getwd()
		if err != nil {
			cwd = "/path/to/repo"
		}

		fmt.Println("=== Linux / macOS Cron Setup ===")
		fmt.Println("To run MIG on startup, add the following to your crontab (crontab -e):")
		fmt.Printf("@reboot cd %s && mig run\n\n", cwd)
		fmt.Println("Or to run daily at 9:00 AM:")
		fmt.Printf("0 9 * * * cd %s && mig run\n\n", cwd)

		fmt.Println("=== Windows Task Scheduler Setup ===")
		fmt.Println("To run MIG daily at 9:00 AM on Windows, run the following in PowerShell (as Administrator):")
		fmt.Printf("schtasks /create /tn \"MIG Daily Contribution\" /tr \"\\\"%s\\mig.exe\\\" run\" /sc daily /st 09:00 /ru \"%%USERNAME%%\"\n", cwd)
	},
}

func init() {
	rootCmd.AddCommand(cronCmd)
}
