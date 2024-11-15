/*
Copyright © 2024 NAME HERE <EMAIL ADDRESS>
*/
package cli

import (
	"fmt"

	"github.com/spf13/cobra"
)

// vaultCmd represents the vault command
var vaultCmd = &cobra.Command{
	Use:   "ak-vault",
	Short: "HashiCorp Vault authentication helper",
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("vault called")
	},
}
