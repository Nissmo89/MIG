package cmd

import (
	"github.com/Nissmo89/MIG/internal/server"
	"github.com/spf13/cobra"
)

var dashboardPort int

var dashboardCmd = &cobra.Command{
	Use:     "dashboard",
	Aliases: []string{"web"},
	Short:   "Launch the interactive local MIG web dashboard",
	RunE: func(cmd *cobra.Command, args []string) error {
		return server.StartServer(dashboardPort)
	},
}

func init() {
	dashboardCmd.Flags().IntVarP(&dashboardPort, "port", "p", 8080, "Port for the local dashboard server")
	rootCmd.AddCommand(dashboardCmd)
}
