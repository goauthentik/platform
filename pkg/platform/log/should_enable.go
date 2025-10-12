package log

import (
	"os"

	"golang.org/x/term"
)

var isDebugger = false

func ShouldSwitch() bool {
	if term.IsTerminal(int(os.Stdout.Fd())) {
		return false
	}
	if isDebugger {
		return false
	}
	return true
}
