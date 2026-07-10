package tui

import (
	"encoding/json"
)

func AnyToMap(input any) (map[string]any, error) {
	b, err := json.Marshal(input)
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
