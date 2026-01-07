package tui

import (
	"fmt"
	"sort"

	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/lipgloss/tree"
)

func BoxStyle() lipgloss.Style {
	return lipgloss.NewStyle().
		Bold(true).
		Foreground(lipgloss.Color("#FAFAFA")).
		Background(lipgloss.Color("#fd4b2d")).
		Padding(1).
		PaddingLeft(5).
		PaddingRight(5)
}

func InlineStyle() lipgloss.Style {
	return lipgloss.NewStyle().
		Bold(true).
		BorderLeftBackground(lipgloss.Color("#fd4b2d")).
		BorderLeft(true)
}

func RenderMapAsTree(data map[string]any, rootTitle string) string {
	// Create styles for different types
	keyStyle := lipgloss.NewStyle().Foreground(lipgloss.Color("6"))
	valueStyle := lipgloss.NewStyle().Foreground(lipgloss.Color("2"))

	// Create the root tree
	t := tree.New().Root(rootTitle).Enumerator(tree.RoundedEnumerator)

	// Sort keys for consistent output
	keys := make([]string, 0, len(data))
	for k := range data {
		keys = append(keys, k)
	}
	sort.Strings(keys)

	// Add each key-value pair to the tree
	for _, key := range keys {
		addNodeToTree(t, keyStyle.Render(key), data[key], keyStyle, valueStyle)
	}

	return t.String()
}

func addNodeToTree(parent *tree.Tree, label string, value any, keyStyle, valueStyle lipgloss.Style) {
	switch v := value.(type) {
	case map[string]any:
		// Create a child tree for nested maps
		child := tree.New().Root(label)

		// Sort keys
		keys := make([]string, 0, len(v))
		for k := range v {
			keys = append(keys, k)
		}
		sort.Strings(keys)

		// Recursively add children
		for _, key := range keys {
			addNodeToTree(child, keyStyle.Render(key), v[key], keyStyle, valueStyle)
		}

		parent.Child(child)

	case []any:
		// Handle slices
		child := tree.New().Root(label)
		for i, item := range v {
			indexLabel := keyStyle.Render(fmt.Sprintf("[%d]", i))
			addNodeToTree(child, indexLabel, item, keyStyle, valueStyle)
		}
		parent.Child(child)

	default:
		// Leaf node - render key: value
		nodeLabel := fmt.Sprintf("%s: %s", label, valueStyle.Render(fmt.Sprintf("%v", v)))
		parent.Child(nodeLabel)
	}
}
