package facts

import (
	"encoding/json"
	"testing"

	log "github.com/sirupsen/logrus"
)

func TestGather(t *testing.T) {
	sysInfo, err := Gather(log.WithField("foo", "bar"))
	if err != nil {
		t.Fatalf("Failed to gather system info: %v", err)
	}

	if sysInfo == nil {
		t.Fatal("SystemInfo is nil")
	}

	// Test JSON conversion
	jsonStr, err := sysInfo.MarshalJSON()
	if err != nil {
		t.Fatalf("Failed to convert to JSON: %v", err)
	}

	if string(jsonStr) == "" {
		t.Fatal("JSON string is empty")
	}

	// Test JSON validity
	var temp interface{}
	if err := json.Unmarshal([]byte(jsonStr), &temp); err != nil {
		t.Fatalf("Invalid JSON produced: %v", err)
	}
}

func TestSystemInfoStructure(t *testing.T) {
	sysInfo, err := Gather(log.WithField("foo", "bar"))
	if err != nil {
		t.Fatalf("Failed to gather system info: %v", err)
	}

	// Test that all major sections are present
	if sysInfo.Os.Get().Family == "" {
		t.Error("OS family is empty")
	}

	if sysInfo.Os.Get().Arch == "" {
		t.Error("OS architecture is empty")
	}

	if len(sysInfo.Disks) == 0 {
		t.Error("No disks found")
	}

	if len(sysInfo.Network.Get().Interfaces) == 0 {
		t.Error("No network interfaces found")
	}
}
