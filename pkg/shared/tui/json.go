package tui

import (
	"encoding/json"
	"os"
)

func JSON(input any) error {
	return json.NewEncoder(os.Stdout).Encode(input)
}

func AnyToMap(input any) (map[string]any, error) {
	b, err := json.MarshalIndent(input, "", "\t")
	if err != nil {
		return nil, err
	}
	out := map[string]any{}
	err = json.Unmarshal(b, &out)
	if err != nil {
		return nil, err
	}
	return out, nil
}
