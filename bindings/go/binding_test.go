package tree_sitter_validatetest_test

import (
	"testing"

	tree_sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_validatetest "github.com/tree-sitter/tree-sitter-validatetest/bindings/go"
)

func TestCanLoadGrammar(t *testing.T) {
	language := tree_sitter.NewLanguage(tree_sitter_validatetest.Language())
	if language == nil {
		t.Errorf("Error loading Validatetest grammar")
	}
}
