package agentstarter

import "testing"

func TestDarwinAgentExecCandidates(t *testing.T) {
	executablePath := "/nix/store/test-ak-agent/Applications/authentik Agent.app/Contents/MacOS/ak-sysd"

	candidates := darwinAgentExecCandidates(executablePath)

	if len(candidates) != 3 {
		t.Fatalf("expected 3 candidates, got %d: %#v", len(candidates), candidates)
	}
	if candidates[0] != "/Applications/authentik Agent.app" {
		t.Fatalf("unexpected first candidate: %q", candidates[0])
	}
	if candidates[1] != "/Applications/Nix Apps/authentik Agent.app" {
		t.Fatalf("unexpected second candidate: %q", candidates[1])
	}
	if candidates[2] != "/nix/store/test-ak-agent/Applications/authentik Agent.app" {
		t.Fatalf("unexpected bundle candidate: %q", candidates[2])
	}
}

func TestFirstExistingPath(t *testing.T) {
	candidates := []string{
		"/Applications/authentik Agent.app",
		"/Applications/Nix Apps/authentik Agent.app",
	}

	selected := firstExistingPath(candidates, func(path string) bool {
		return path == "/Applications/Nix Apps/authentik Agent.app"
	})

	if selected != "/Applications/Nix Apps/authentik Agent.app" {
		t.Fatalf("unexpected selected path: %q", selected)
	}
}

func TestFirstExistingPathFallback(t *testing.T) {
	candidates := []string{
		"/Applications/authentik Agent.app",
		"/Applications/Nix Apps/authentik Agent.app",
	}

	selected := firstExistingPath(candidates, func(string) bool {
		return false
	})

	if selected != "/Applications/authentik Agent.app" {
		t.Fatalf("unexpected fallback path: %q", selected)
	}
}
