//! Formatter for GStreamer ValidateTest files
//!
//! Usage: validatetest-fmt [OPTIONS] <FILE>...
//!
//! Options:
//!   -i, --in-place    Edit files in place
//!   -c, --check       Check if files are formatted (exit 1 if not)
//!   --indent <N>      Indentation width (default: 4)

use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;

use tree_sitter::{Node, Parser};
use tree_sitter_validatetest::LANGUAGE;

const DEFAULT_INDENT: usize = 4;
const DEFAULT_LINE_LENGTH: usize = 120;

struct Formatter<'a> {
    source: &'a [u8],
    output: String,
    indent_width: usize,
    max_line_length: usize,
    current_indent: usize,
}

impl<'a> Formatter<'a> {
    fn new(source: &'a str, indent_width: usize, max_line_length: usize) -> Self {
        Self {
            source: source.as_bytes(),
            output: String::with_capacity(source.len()),
            indent_width,
            max_line_length,
            current_indent: 0,
        }
    }

    fn indent(&self) -> String {
        " ".repeat(self.current_indent)
    }

    fn format(mut self, root: Node<'a>) -> String {
        self.format_node(root);
        // Ensure file ends with newline
        if !self.output.ends_with('\n') {
            self.output.push('\n');
        }
        self.output
    }

    fn node_text(&self, node: Node) -> String {
        node.utf8_text(self.source).unwrap_or("").to_string()
    }

    fn format_node(&mut self, node: Node<'a>) {
        match node.kind() {
            "source_file" => self.format_source_file(node),
            "structure" => self.format_structure(node),
            "array_structure" => self.format_array_structure(node),
            "field_list" => self.format_field_list(node),
            "field" => self.format_field(node),
            "nested_structure_block" => self.format_nested_block(node),
            "array" => self.format_array(node),
            "angle_bracket_array" => self.format_angle_bracket_array(node),
            "comment" => self.format_comment(node),
            _ => self.format_leaf(node),
        }
    }

    fn count_blank_lines_between(&self, end_byte: usize, start_byte: usize) -> usize {
        if start_byte <= end_byte {
            return 0;
        }
        let between = &self.source[end_byte..start_byte];
        // Count newlines, subtract 1 for the line break after the previous node
        let newlines = between.iter().filter(|&&b| b == b'\n').count();
        newlines.saturating_sub(1)
    }

    fn format_source_file(&mut self, node: Node<'a>) {
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();
        let mut prev_end_byte = 0;

        for child in children {
            // Preserve blank lines from source
            let blank_lines = self.count_blank_lines_between(prev_end_byte, child.start_byte());
            for _ in 0..blank_lines {
                self.output.push('\n');
            }

            if child.kind() == "comment" {
                self.format_comment(child);
                self.output.push('\n');
            } else if child.kind() == "structure" {
                self.format_structure(child);
                self.output.push('\n');
            }
            prev_end_byte = child.end_byte();
        }
    }

    fn structure_fits_on_line(&self, node: Node<'a>) -> bool {
        // If structure contains any nested blocks, always split
        if self.contains_nested_block(node) {
            return false;
        }
        // Property-related actions should always be multiline for readability
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "structure_name" {
                let name = self.node_text(child);
                if name == "check-properties"
                    || name == "check-child-properties"
                    || name == "set-child-properties"
                    || name == "set-properties"
                    || name == "expected-issue"
                {
                    return false;
                }
                break;
            }
        }
        let inline = self.format_structure_inline(node);
        self.current_indent + inline.len() <= self.max_line_length && !inline.contains('\n')
    }

    fn contains_nested_block(&self, node: Node<'a>) -> bool {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "nested_structure_block" {
                return true;
            }
            if child.kind() == "field_list"
                || child.kind() == "field"
                || child.kind() == "field_value"
            {
                if self.contains_nested_block(child) {
                    return true;
                }
            }
        }
        false
    }

    fn format_structure_inline(&self, node: Node<'a>) -> String {
        let mut result = String::new();
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();

        // Get structure name
        for child in &children {
            if child.kind() == "structure_name" {
                result.push_str(&self.node_text(*child));
                break;
            }
        }

        // Get field list
        for child in &children {
            if child.kind() == "field_list" {
                result.push_str(", ");
                result.push_str(&self.format_field_list_inline(*child));
                break;
            }
        }

        // Check for semicolon
        if children.iter().any(|c| c.kind() == ";") {
            result.push(';');
        }

        result
    }

    fn format_field_list_inline(&self, node: Node<'a>) -> String {
        let mut result = String::new();
        let mut cursor = node.walk();
        let fields: Vec<_> = node
            .children(&mut cursor)
            .filter(|c| c.kind() == "field")
            .collect();

        for (i, field) in fields.iter().enumerate() {
            result.push_str(&self.format_field_inline(*field));
            if i < fields.len() - 1 {
                result.push_str(", ");
            }
        }
        result
    }

    fn format_field_inline(&self, node: Node<'a>) -> String {
        let mut result = String::new();

        // Field name
        if let Some(name) = node.child_by_field_name("name") {
            result.push_str(&self.node_text(name));
        }

        result.push_str("=");

        // Field value
        if let Some(value) = node.child_by_field_name("value") {
            result.push_str(&self.format_field_value_inline(value));
        }

        result
    }

    fn format_field_value_inline(&self, node: Node<'a>) -> String {
        let mut result = String::new();
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();

        for child in children {
            match child.kind() {
                "nested_structure_block" => {
                    result.push_str(&self.format_nested_block_inline(child))
                }
                "array" => result.push_str(&self.format_array_inline(child)),
                "angle_bracket_array" => {
                    result.push_str(&self.format_angle_bracket_array_inline(child))
                }
                "typed_value" => result.push_str(&self.format_typed_value_inline(child)),
                "value" => result.push_str(&self.format_value_inline(child)),
                _ => {}
            }
        }
        result
    }

    fn format_nested_block_inline(&self, node: Node<'a>) -> String {
        let mut result = String::new();
        let mut cursor = node.walk();
        let children: Vec<_> = node
            .children(&mut cursor)
            .filter(|c| c.kind() != "{" && c.kind() != "}" && c.kind() != ",")
            .collect();

        result.push('{');
        for (i, child) in children.iter().enumerate() {
            match child.kind() {
                "structure" => result.push_str(&self.format_structure_inline(*child)),
                "field_value" => result.push_str(&self.format_field_value_inline(*child)),
                "comment" => result.push_str(&self.node_text(*child)),
                _ => {}
            }
            if i < children.len() - 1 {
                result.push_str(", ");
            }
        }
        result.push('}');
        result
    }

    fn format_typed_value_inline(&self, node: Node<'a>) -> String {
        let mut result = String::new();
        result.push('(');
        if let Some(type_name) = node.child_by_field_name("type") {
            result.push_str(&self.node_text(type_name));
        }
        result.push(')');

        if let Some(value) = node.child_by_field_name("value") {
            match value.kind() {
                "array" => result.push_str(&self.format_array_inline(value)),
                "angle_bracket_array" => {
                    result.push_str(&self.format_angle_bracket_array_inline(value))
                }
                "value" => result.push_str(&self.node_text(value)),
                _ => result.push_str(&self.node_text(value)),
            }
        }
        result
    }

    fn format_array_inline(&self, node: Node<'a>) -> String {
        let mut result = String::new();
        let mut cursor = node.walk();
        let elements: Vec<_> = node
            .children(&mut cursor)
            .filter(|c| c.kind() == "array_element")
            .collect();

        if elements.is_empty() {
            return "[]".to_string();
        }

        result.push('[');
        for (i, elem) in elements.iter().enumerate() {
            result.push_str(&self.format_array_element_inline_str(*elem));
            if i < elements.len() - 1 {
                result.push_str(", ");
            }
        }
        result.push(']');
        result
    }

    fn format_array_element_inline_str(&self, node: Node<'a>) -> String {
        let mut result = String::new();
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();

        for child in children {
            match child.kind() {
                "array_structure" => result.push_str(&self.format_array_structure_inline(child)),
                "typed_value" => result.push_str(&self.format_typed_value_inline(child)),
                "," => {}
                _ => result.push_str(&self.node_text(child)),
            }
        }
        result
    }

    fn format_array_structure_inline(&self, node: Node<'a>) -> String {
        let mut result = String::new();
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();

        for child in &children {
            if child.kind() == "structure_name" {
                result.push_str(&self.node_text(*child));
                break;
            }
        }

        for child in &children {
            if child.kind() == "field_list" {
                result.push_str(", ");
                result.push_str(&self.format_field_list_inline(*child));
                break;
            }
        }
        result
    }

    fn format_angle_bracket_array_inline(&self, node: Node<'a>) -> String {
        let mut result = String::new();
        let mut cursor = node.walk();
        let values: Vec<_> = node
            .children(&mut cursor)
            .filter(|c| c.kind() == "field_value")
            .collect();

        if values.is_empty() {
            return "<>".to_string();
        }

        result.push('<');
        for (i, val) in values.iter().enumerate() {
            result.push_str(&self.format_field_value_inline(*val));
            if i < values.len() - 1 {
                result.push_str(", ");
            }
        }
        result.push('>');
        result
    }

    fn format_structure(&mut self, node: Node<'a>) {
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();

        // Check if structure fits on one line
        if self.structure_fits_on_line(node) {
            let indent = self.indent();
            self.output.push_str(&indent);
            self.output.push_str(&self.format_structure_inline(node));
            return;
        }

        // Get structure name
        for child in &children {
            if child.kind() == "structure_name" {
                let text = self.node_text(*child);
                let indent = self.indent();
                self.output.push_str(&indent);
                self.output.push_str(&text);
                break;
            }
        }

        // Get field list
        for child in &children {
            if child.kind() == "field_list" {
                self.output.push_str(",\n");
                self.current_indent += self.indent_width;
                self.format_field_list(*child);
                self.current_indent -= self.indent_width;
                break;
            }
        }

        // Check for semicolon
        if children.iter().any(|c| c.kind() == ";") {
            self.output.push(';');
        }
    }

    fn format_array_structure(&mut self, node: Node<'a>) {
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();

        // Get structure name
        for child in &children {
            if child.kind() == "structure_name" {
                let text = self.node_text(*child);
                self.output.push_str(&text);
                break;
            }
        }

        // Get field list
        for child in &children {
            if child.kind() == "field_list" {
                self.output.push_str(", ");
                self.format_inline_field_list(*child);
                break;
            }
        }
    }

    fn format_field_list(&mut self, node: Node<'a>) {
        let mut cursor = node.walk();
        let fields: Vec<_> = node
            .children(&mut cursor)
            .filter(|c| c.kind() == "field")
            .collect();

        for (i, field) in fields.iter().enumerate() {
            self.format_field(*field);
            if i < fields.len() - 1 {
                self.output.push_str(",\n");
            }
        }
    }

    fn format_inline_field_list(&mut self, node: Node<'a>) {
        let mut cursor = node.walk();
        let fields: Vec<_> = node
            .children(&mut cursor)
            .filter(|c| c.kind() == "field")
            .collect();

        for (i, field) in fields.iter().enumerate() {
            self.format_inline_field(*field);
            if i < fields.len() - 1 {
                self.output.push_str(", ");
            }
        }
    }

    fn format_field(&mut self, node: Node<'a>) {
        let indent = self.indent();
        self.output.push_str(&indent);
        self.format_inline_field(node);
    }

    fn format_inline_field(&mut self, node: Node<'a>) {
        // Field name
        if let Some(name) = node.child_by_field_name("name") {
            let text = self.node_text(name);
            self.output.push_str(&text);
        }

        self.output.push_str("=");

        // Field value
        if let Some(value) = node.child_by_field_name("value") {
            self.format_field_value(value);
        }
    }

    fn format_field_value(&mut self, node: Node<'a>) {
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();

        for child in children {
            match child.kind() {
                "nested_structure_block" => self.format_nested_block(child),
                "array" => self.format_array(child),
                "angle_bracket_array" => self.format_angle_bracket_array(child),
                "typed_value" => self.format_typed_value(child),
                "value" => self.format_value(child),
                _ => {}
            }
        }
    }

    fn format_typed_value(&mut self, node: Node<'a>) {
        self.output.push('(');
        if let Some(type_name) = node.child_by_field_name("type") {
            let text = self.node_text(type_name);
            self.output.push_str(&text);
        }
        self.output.push(')');

        if let Some(value) = node.child_by_field_name("value") {
            match value.kind() {
                "array" => self.format_array(value),
                "angle_bracket_array" => self.format_angle_bracket_array(value),
                "value" => self.format_value(value),
                _ => {
                    let text = self.node_text(value);
                    self.output.push_str(&text);
                }
            }
        }
    }

    fn format_value(&mut self, node: Node<'a>) {
        let text = self.format_value_inline(node);
        self.output.push_str(&text);
    }

    fn format_value_inline(&self, node: Node<'a>) -> String {
        let text = self.node_text(node);

        // Check if this is a quoted string that should be converted to array structure
        if let Some(converted) = self.try_convert_quoted_structure(&text) {
            return converted;
        }

        text
    }

    /// Check if a quoted string contains a structure that should be converted to array format
    fn try_convert_quoted_structure(&self, text: &str) -> Option<String> {
        // Must be a quoted string
        if !text.starts_with('"') || !text.ends_with('"') {
            return None;
        }

        // Structure names that should be converted from quoted strings to array structures
        let convertible_names = ["expected-issue,", "change-severity,"];

        // Check if the content starts with a convertible structure name
        let inner = &text[1..text.len() - 1]; // Remove quotes
        let is_convertible = convertible_names.iter().any(|name| inner.starts_with(name));

        if !is_convertible {
            return None;
        }

        // Unescape the string content
        let unescaped = self.unescape_string(inner);

        // Parse and format as array structure
        self.parse_and_format_as_array_structure(&unescaped)
    }

    /// Unescape a string: \" -> " and \\ -> \
    fn unescape_string(&self, s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(&next) = chars.peek() {
                    match next {
                        '"' | '\\' => {
                            result.push(next);
                            chars.next();
                        }
                        _ => {
                            result.push(c);
                        }
                    }
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }

        result
    }

    /// Parse a structure string and format it as an array structure [name, fields...]
    fn parse_and_format_as_array_structure(&self, content: &str) -> Option<String> {
        // Parse the content as a structure
        let mut parser = Parser::new();
        parser.set_language(&LANGUAGE.into()).ok()?;

        let tree = parser.parse(content, None)?;
        let root = tree.root_node();

        // Find the structure node
        let structure_node = if root.kind() == "source_file" {
            root.child(0)?
        } else {
            root
        };

        if structure_node.kind() != "structure" {
            return None;
        }

        // Get structure name to check if it should be multiline
        let mut structure_name = None;
        let mut cursor = structure_node.walk();
        for child in structure_node.children(&mut cursor) {
            if child.kind() == "structure_name" {
                structure_name = Some(
                    child
                        .utf8_text(content.as_bytes())
                        .unwrap_or("")
                        .to_string(),
                );
                break;
            }
        }

        // Check if this structure should always be multiline
        let always_multiline = matches!(
            structure_name.as_deref(),
            Some("expected-issue") | Some("change-severity")
        );

        let formatter = Formatter::new(content, self.indent_width, self.max_line_length);
        let inline = formatter.format_structure_inline(structure_node);

        // Check if we should format multiline
        if always_multiline || self.current_indent + inline.len() + 2 > self.max_line_length {
            // Format multiline
            let mut result = String::new();
            result.push_str("[");
            result.push_str(structure_name.as_deref().unwrap_or(""));
            result.push_str(",\n");

            // Get field list and format each field
            let mut cursor = structure_node.walk();
            for child in structure_node.children(&mut cursor) {
                if child.kind() == "field_list" {
                    let indent = " ".repeat(self.current_indent + self.indent_width);
                    let mut field_cursor = child.walk();
                    for field in child.children(&mut field_cursor) {
                        if field.kind() == "field" {
                            result.push_str(&indent);
                            result.push_str(&formatter.format_field_inline(field));
                            result.push_str(",\n");
                        }
                    }
                    break;
                }
            }

            // Close with proper indentation
            let close_indent = " ".repeat(self.current_indent);
            result.push_str(&close_indent);
            result.push(']');
            return Some(result);
        }

        // Return as inline array structure format
        Some(format!("[{}]", inline))
    }

    fn field_value_has_nested_block(&self, node: Node<'a>) -> bool {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "nested_structure_block" => return true,
                "array" => {
                    // Check if any element in the array has nested blocks
                    let mut arr_cursor = child.walk();
                    for arr_child in child.children(&mut arr_cursor) {
                        if arr_child.kind() == "array_element" {
                            if self.array_element_has_nested_block(arr_child) {
                                return true;
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        false
    }

    fn field_value_has_array_structure(&self, node: Node<'a>) -> bool {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "array" {
                let mut arr_cursor = child.walk();
                for arr_child in child.children(&mut arr_cursor) {
                    if arr_child.kind() == "array_element" {
                        let mut elem_cursor = arr_child.walk();
                        for elem_child in arr_child.children(&mut elem_cursor) {
                            if elem_child.kind() == "array_structure" {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    /// Check if a field_value contains an array structure that should always be multiline
    fn field_value_should_be_multiline(&self, node: Node<'a>) -> bool {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "array" {
                let mut arr_cursor = child.walk();
                for arr_child in child.children(&mut arr_cursor) {
                    if arr_child.kind() == "array_element" {
                        if self.array_element_should_be_multiline(arr_child) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn format_nested_block(&mut self, node: Node<'a>) {
        self.output.push_str("{\n");
        self.current_indent += self.indent_width;

        let mut cursor = node.walk();
        let children: Vec<_> = node
            .children(&mut cursor)
            .filter(|c| c.kind() != "{" && c.kind() != "}" && c.kind() != ",")
            .collect();

        // Pre-process: associate trailing comments with their elements
        let mut items: Vec<(Node<'a>, Option<Node<'a>>)> = Vec::new();
        let mut i = 0;
        while i < children.len() {
            let child = children[i];
            if child.kind() == "comment" {
                // Standalone comment
                items.push((child, None));
                i += 1;
            } else {
                // Check for trailing comment
                let trailing = if i + 1 < children.len() {
                    let next = children[i + 1];
                    if next.kind() == "comment"
                        && child.end_position().row == next.start_position().row
                    {
                        i += 1; // Skip the comment in main loop
                        Some(next)
                    } else {
                        None
                    }
                } else {
                    None
                };
                items.push((child, trailing));
                i += 1;
            }
        }

        // Check if any item is complex (structure, has nested blocks, or contains array structures)
        // If so, put each item on its own line
        let has_complex_items = items.iter().any(|(child, _)| {
            child.kind() == "structure"
                || (child.kind() == "field_value" && self.field_value_has_nested_block(*child))
                || (child.kind() == "field_value" && self.field_value_has_array_structure(*child))
        });

        let indent = self.indent();
        let mut current_line_len = 0;
        let mut line_started = false;

        for (idx, (child, trailing_comment)) in items.iter().enumerate() {
            let is_last = idx == items.len() - 1;

            match child.kind() {
                "structure" => {
                    if line_started {
                        self.output.push_str(",\n");
                    }
                    self.format_structure(*child);
                    self.output.push(',');
                    if let Some(comment) = trailing_comment {
                        let comment_text = self.node_text(*comment);
                        self.output.push_str("  ");
                        self.output.push_str(&comment_text);
                    }
                    self.output.push('\n');
                    line_started = false;
                    current_line_len = 0;
                }
                "field_value" => {
                    // Check if this field_value contains nested blocks - format multiline if so
                    if self.field_value_has_nested_block(*child) {
                        if line_started {
                            self.output.push_str(",\n");
                            line_started = false;
                        }
                        self.output.push_str(&indent);
                        self.format_field_value(*child);
                        self.output.push(',');
                        if let Some(comment) = trailing_comment {
                            let comment_text = self.node_text(*comment);
                            self.output.push_str("  ");
                            self.output.push_str(&comment_text);
                        }
                        self.output.push('\n');
                        current_line_len = 0;
                        continue;
                    }

                    let value_str = self.format_field_value_inline(*child);
                    let comment_text = trailing_comment.map(|c| self.node_text(c));
                    let comment_len = comment_text.as_ref().map(|t| 2 + t.len()).unwrap_or(0);

                    // Check if comment would make line too long - if so, put it before
                    let comment_on_own_line = if let Some(ref _ct) = comment_text {
                        self.current_indent + value_str.len() + 1 + comment_len
                            > self.max_line_length
                    } else {
                        false
                    };

                    // Emit comment before if needed
                    if comment_on_own_line {
                        if line_started {
                            self.output.push_str(",\n");
                            line_started = false;
                        }
                        if let Some(comment) = trailing_comment {
                            self.format_comment(*comment);
                            self.output.push('\n');
                        }
                    }

                    // If block has complex items, each item goes on its own line
                    if has_complex_items {
                        if line_started {
                            self.output.push_str(",\n");
                        }

                        // Check if field_value contains array structure that should always be multiline
                        let always_multiline = self.field_value_should_be_multiline(*child);

                        // Check if inline representation exceeds line length or should always be multiline
                        if always_multiline
                            || self.current_indent + value_str.len() > self.max_line_length
                        {
                            // Format multiline
                            self.output.push_str(&indent);
                            self.format_field_value(*child);
                            self.output.push(',');
                        } else {
                            self.output.push_str(&indent);
                            self.output.push_str(&value_str);
                            self.output.push(',');
                        }
                        if !comment_on_own_line {
                            if let Some(ref ct) = comment_text {
                                self.output.push_str("  ");
                                self.output.push_str(ct);
                            }
                        }
                        self.output.push('\n');
                        line_started = false;
                        current_line_len = 0;
                    } else {
                        // Start line if needed
                        if !line_started {
                            self.output.push_str(&indent);
                            current_line_len = self.current_indent;
                            line_started = true;
                        } else {
                            // Check if value fits on current line
                            let value_total =
                                value_str.len() + if comment_on_own_line { 0 } else { comment_len };
                            let needed = 2 + value_total + 1; // ", " + value + ","
                            if current_line_len + needed > self.max_line_length {
                                self.output.push_str(",\n");
                                self.output.push_str(&indent);
                                current_line_len = self.current_indent;
                            } else {
                                self.output.push_str(", ");
                                current_line_len += 2;
                            }
                        }

                        self.output.push_str(&value_str);
                        current_line_len += value_str.len();

                        if is_last {
                            self.output.push(',');
                            if !comment_on_own_line {
                                if let Some(ref ct) = comment_text {
                                    self.output.push_str("  ");
                                    self.output.push_str(ct);
                                }
                            }
                            self.output.push('\n');
                            line_started = false;
                        } else if !comment_on_own_line {
                            if let Some(ref ct) = comment_text {
                                self.output.push(',');
                                self.output.push_str("  ");
                                self.output.push_str(ct);
                                self.output.push('\n');
                                line_started = false;
                                current_line_len = 0;
                            }
                        }
                    }
                }
                "comment" => {
                    // Standalone comment
                    if line_started {
                        self.output.push_str(",\n");
                        line_started = false;
                    }
                    self.format_comment(*child);
                    self.output.push('\n');
                    current_line_len = 0;
                }
                _ => {}
            }
        }

        self.current_indent -= self.indent_width;
        let closing_indent = self.indent();
        self.output.push_str(&closing_indent);
        self.output.push('}');
    }

    fn array_element_has_nested_block(&self, elem: Node<'a>) -> bool {
        let mut cursor = elem.walk();
        for child in elem.children(&mut cursor) {
            if child.kind() == "array_structure" {
                if self.contains_nested_block(child) {
                    return true;
                }
            }
        }
        false
    }

    /// Check if an array element's structure should always be formatted multiline
    fn array_element_should_be_multiline(&self, elem: Node<'a>) -> bool {
        let mut cursor = elem.walk();
        for child in elem.children(&mut cursor) {
            if child.kind() == "array_structure" {
                // Get structure name
                let mut struct_cursor = child.walk();
                for struct_child in child.children(&mut struct_cursor) {
                    if struct_child.kind() == "structure_name" {
                        let name = self.node_text(struct_child);
                        return name == "expected-issue"
                            || name == "change-severity"
                            || name == "check-properties"
                            || name == "check-child-properties"
                            || name == "set-child-properties"
                            || name == "set-properties";
                    }
                }
            }
        }
        false
    }

    fn format_array_element(&mut self, elem: Node<'a>) {
        let mut cursor = elem.walk();
        let children: Vec<_> = elem.children(&mut cursor).collect();

        // Find the array_structure if present
        let array_struct = children.iter().find(|c| c.kind() == "array_structure");

        if let Some(struct_node) = array_struct {
            // Format as name,\n    fields... (no brackets - array handles those)
            self.format_array_structure_multiline(*struct_node);
        } else {
            // Fallback for non-structure elements
            for child in children {
                match child.kind() {
                    "typed_value" => self.format_typed_value(child),
                    "[" | "]" | "," => {}
                    _ => {
                        let text = self.node_text(child);
                        self.output.push_str(&text);
                    }
                }
            }
        }
    }

    fn format_array_structure_multiline(&mut self, node: Node<'a>) {
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();

        // Get structure name and check if it should always be multiline
        let mut structure_name = String::new();
        for child in &children {
            if child.kind() == "structure_name" {
                structure_name = self.node_text(*child);
                self.output.push_str(&structure_name);
                break;
            }
        }

        let always_multiline = structure_name == "expected-issue"
            || structure_name == "change-severity"
            || structure_name == "check-properties"
            || structure_name == "check-child-properties"
            || structure_name == "set-child-properties"
            || structure_name == "set-properties";

        // Get field list - format multiline if it contains nested blocks, exceeds line length, or is always-multiline
        for child in &children {
            if child.kind() == "field_list" {
                let inline_fields = self.format_field_list_inline(*child);
                let needs_multiline = always_multiline
                    || self.contains_nested_block(*child)
                    || self.current_indent + inline_fields.len() + 2 > self.max_line_length;

                if needs_multiline {
                    self.output.push_str(",\n");
                    self.current_indent += self.indent_width;
                    self.format_field_list(*child);
                    self.current_indent -= self.indent_width;
                } else {
                    self.output.push_str(", ");
                    self.output.push_str(&inline_fields);
                }
                break;
            }
        }
    }

    fn format_array(&mut self, node: Node<'a>) {
        let mut cursor = node.walk();
        let elements: Vec<_> = node
            .children(&mut cursor)
            .filter(|c| c.kind() == "array_element")
            .collect();

        if elements.is_empty() {
            self.output.push_str("[]");
            return;
        }

        // Check if any element has nested blocks or should always be multiline
        let has_nested_blocks = elements
            .iter()
            .any(|e| self.array_element_has_nested_block(*e));

        let has_always_multiline = elements
            .iter()
            .any(|e| self.array_element_should_be_multiline(*e));

        if !has_nested_blocks && !has_always_multiline {
            // Check if entire array fits on one line
            let inline_str = self.format_array_inline(node);
            if self.current_indent + inline_str.len() <= self.max_line_length
                && !inline_str.contains('\n')
            {
                self.output.push_str(&inline_str);
                return;
            }
        }

        // Special case: single-element array with nested blocks or always-multiline structure
        if elements.len() == 1 && (has_nested_blocks || has_always_multiline) {
            let elem = elements[0];
            let mut c = elem.walk();
            let children: Vec<_> = elem.children(&mut c).collect();
            if let Some(struct_node) = children.iter().find(|c| c.kind() == "array_structure") {
                self.output.push('[');
                self.format_array_structure_multiline(*struct_node);
                self.output.push(']');
                return;
            }
        }

        // Special case: single-element array with structure that exceeds line length
        if elements.len() == 1 {
            let elem = elements[0];
            let mut c = elem.walk();
            let children: Vec<_> = elem.children(&mut c).collect();
            if let Some(struct_node) = children.iter().find(|c| c.kind() == "array_structure") {
                let inline_str = self.format_array_element_inline_str(elem);
                if self.current_indent + inline_str.len() > self.max_line_length {
                    self.output.push('[');
                    self.format_array_structure_multiline(*struct_node);
                    self.output.push(']');
                    return;
                }
            }
        }

        // Multi-line format with packing
        self.output.push_str("[\n");
        self.current_indent += self.indent_width;

        let indent = self.indent();
        let mut current_line_len = 0;
        let mut line_started = false;

        for (i, elem) in elements.iter().enumerate() {
            let is_last = i == elements.len() - 1;
            let has_nested = self.array_element_has_nested_block(*elem);

            // Check if element contains a structure (needs its own line)
            let has_structure = {
                let mut c = elem.walk();
                let children: Vec<_> = elem.children(&mut c).collect();
                children.iter().any(|c| c.kind() == "array_structure")
            };

            if has_nested {
                // Elements with nested blocks get proper multiline formatting
                if line_started {
                    self.output.push_str(",\n");
                }
                self.output.push_str(&indent);
                self.format_array_element(*elem);
                self.output.push_str(",\n");
                line_started = false;
                current_line_len = 0;
            } else if has_structure {
                // Simple structures get their own line
                let elem_str = self.format_array_element_inline_str(*elem);
                if line_started {
                    self.output.push_str(",\n");
                }

                // Check if this structure should always be multiline
                let always_multiline = self.array_element_should_be_multiline(*elem);

                // Check if inline representation exceeds line length or should always be multiline
                if always_multiline || self.current_indent + elem_str.len() > self.max_line_length {
                    // Format multiline
                    self.output.push_str(&indent);
                    self.format_array_element(*elem);
                    self.output.push_str(",\n");
                } else {
                    self.output.push_str(&indent);
                    self.output.push_str(&elem_str);
                    self.output.push_str(",\n");
                }
                line_started = false;
                current_line_len = 0;
            } else {
                // Simple values can be packed
                let elem_str = self.format_array_element_inline_str(*elem);
                if !line_started {
                    self.output.push_str(&indent);
                    current_line_len = self.current_indent;
                    line_started = true;
                } else {
                    let needed = 2 + elem_str.len();
                    if current_line_len + needed > self.max_line_length {
                        self.output.push_str(",\n");
                        self.output.push_str(&indent);
                        current_line_len = self.current_indent;
                    } else {
                        self.output.push_str(", ");
                        current_line_len += 2;
                    }
                }

                self.output.push_str(&elem_str);
                current_line_len += elem_str.len();

                if is_last {
                    self.output.push_str(",\n");
                    line_started = false;
                }
            }
        }

        self.current_indent -= self.indent_width;
        let closing_indent = self.indent();
        self.output.push_str(&closing_indent);
        self.output.push(']');
    }

    fn format_angle_bracket_array(&mut self, node: Node<'a>) {
        let mut cursor = node.walk();
        let values: Vec<_> = node
            .children(&mut cursor)
            .filter(|c| c.kind() == "field_value")
            .collect();

        if values.is_empty() {
            self.output.push_str("<>");
            return;
        }

        self.output.push('<');
        for (i, val) in values.iter().enumerate() {
            self.format_field_value(*val);
            if i < values.len() - 1 {
                self.output.push_str(", ");
            }
        }
        self.output.push('>');
    }

    fn format_comment(&mut self, node: Node<'a>) {
        let indent = self.indent();
        let text = self.node_text(node);

        // Check if comment fits on one line
        if self.current_indent + text.len() <= self.max_line_length {
            self.output.push_str(&indent);
            self.output.push_str(&text);
            return;
        }

        // Need to wrap the comment
        let content = text.strip_prefix('#').unwrap_or(&text);
        let content = content.strip_prefix(' ').unwrap_or(content);
        let prefix = format!("{}# ", indent);
        let max_content_len = self.max_line_length - prefix.len();

        let words: Vec<&str> = content.split_whitespace().collect();
        let mut current_line = String::new();
        let mut first_line = true;

        for word in words {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() <= max_content_len {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                // Emit current line and start new one
                if !first_line {
                    self.output.push('\n');
                }
                self.output.push_str(&prefix);
                self.output.push_str(&current_line);
                current_line = word.to_string();
                first_line = false;
            }
        }

        // Emit last line
        if !current_line.is_empty() {
            if !first_line {
                self.output.push('\n');
            }
            self.output.push_str(&prefix);
            self.output.push_str(&current_line);
        }
    }

    fn format_leaf(&mut self, node: Node<'a>) {
        let text = self.node_text(node);
        self.output.push_str(&text);
    }
}

fn format_file(
    source: &str,
    indent_width: usize,
    max_line_length: usize,
) -> Result<String, String> {
    let mut parser = Parser::new();
    parser
        .set_language(&LANGUAGE.into())
        .map_err(|e| format!("Failed to load parser: {}", e))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| "Failed to parse file".to_string())?;

    let root = tree.root_node();
    if root.has_error() {
        // Find the error node for better error message
        let mut cursor = root.walk();
        let children: Vec<_> = root.children(&mut cursor).collect();
        for node in children {
            if node.has_error() || node.kind() == "ERROR" {
                return Err(format!(
                    "Parse error at line {}, column {}",
                    node.start_position().row + 1,
                    node.start_position().column + 1
                ));
            }
        }
        return Err("Parse error in file".to_string());
    }

    let formatter = Formatter::new(source, indent_width, max_line_length);
    Ok(formatter.format(root))
}

fn print_usage() {
    eprintln!("Usage: validatetest-fmt [OPTIONS] <FILE>...");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  -i, --in-place      Edit files in place");
    eprintln!("  -c, --check         Check if files are formatted (exit 1 if not)");
    eprintln!("  --indent <N>        Indentation width (default: 4)");
    eprintln!("  --line-length <N>   Maximum line length (default: 120)");
    eprintln!("  -h, --help          Show this help message");
    eprintln!();
    eprintln!("If no FILE is given, reads from stdin and writes to stdout.");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut in_place = false;
    let mut check_only = false;
    let mut indent_width = DEFAULT_INDENT;
    let mut max_line_length = DEFAULT_LINE_LENGTH;
    let mut files: Vec<String> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_usage();
                process::exit(0);
            }
            "-i" | "--in-place" => in_place = true,
            "-c" | "--check" => check_only = true,
            "--indent" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --indent requires a value");
                    process::exit(1);
                }
                indent_width = args[i].parse().unwrap_or_else(|_| {
                    eprintln!("Error: invalid indent value");
                    process::exit(1);
                });
            }
            "--line-length" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --line-length requires a value");
                    process::exit(1);
                }
                max_line_length = args[i].parse().unwrap_or_else(|_| {
                    eprintln!("Error: invalid line-length value");
                    process::exit(1);
                });
            }
            arg if arg.starts_with('-') => {
                eprintln!("Error: unknown option {}", arg);
                process::exit(1);
            }
            _ => files.push(args[i].clone()),
        }
        i += 1;
    }

    // Read from stdin if no files provided
    if files.is_empty() {
        let mut source = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut source) {
            eprintln!("Error reading stdin: {}", e);
            process::exit(1);
        }

        match format_file(&source, indent_width, max_line_length) {
            Ok(formatted) => {
                if check_only {
                    if formatted != source {
                        process::exit(1);
                    }
                } else {
                    print!("{}", formatted);
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
        return;
    }

    let mut any_diff = false;

    for file in &files {
        let source = match fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error reading {}: {}", file, e);
                process::exit(1);
            }
        };

        match format_file(&source, indent_width, max_line_length) {
            Ok(formatted) => {
                if check_only {
                    if formatted != source {
                        eprintln!("{}: needs formatting", file);
                        any_diff = true;
                    }
                } else if in_place {
                    if formatted != source {
                        if let Err(e) = fs::write(file, &formatted) {
                            eprintln!("Error writing {}: {}", file, e);
                            process::exit(1);
                        }
                        eprintln!("Formatted: {}", file);
                    }
                } else {
                    print!("{}", formatted);
                }
            }
            Err(e) => {
                eprintln!("Error formatting {}: {}", file, e);
                process::exit(1);
            }
        }
    }

    if check_only && any_diff {
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(input: &str) -> String {
        format_file(input, DEFAULT_INDENT, DEFAULT_LINE_LENGTH).unwrap()
    }

    #[test]
    fn test_simple_structure_inline() {
        assert_eq!(fmt("action, foo=bar"), "action, foo=bar\n");
    }

    #[test]
    fn test_simple_structure_multiline() {
        assert_eq!(
            fmt("action, foo=bar, baz=123"),
            "action, foo=bar, baz=123\n"
        );
    }

    #[test]
    fn test_long_structure_splits() {
        // This input is >150 chars when formatted, so it should split
        let input="very-long-action-name-here, field1=\"some long value here\", field2=\"another long value\", field3=\"yet another value\", field4=\"and more values\", field5=\"even more values here to exceed the limit\"";
        let output = fmt(input);
        assert!(
            output.contains(",\n    "),
            "Long structure should split to multiple lines"
        );
    }

    #[test]
    fn test_nested_block_packing() {
        let input = "meta, args={-t, video, --sink, fakesink}";
        let output = fmt(input);
        // Short values should be packed on same line
        assert!(output.contains("-t, video, --sink, fakesink"));
    }

    #[test]
    fn test_nested_block_long_value_own_line() {
        // The nested block content exceeds 150 chars, so the structure should go multiline
        // and the long string should be on its own line within the block
        let input = r#"meta, args={-t, video, --sink, "this is a very long string value that definitely exceeds one hundred and fifty characters so it should cause line breaking to occur"}"#;
        let output = fmt(input);
        // Structure should split because nested block is long
        assert!(
            output.contains("args={\n"),
            "Should split to multiline when block content is long"
        );
    }

    #[test]
    fn test_preserves_blank_lines() {
        let input = "action1, foo=bar\n\naction2, baz=123";
        let output = fmt(input);
        assert!(
            output.contains("\n\n"),
            "Should preserve blank line between structures"
        );
    }

    #[test]
    fn test_no_extra_blank_lines() {
        let input = "action1, foo=bar\naction2, baz=123";
        let output = fmt(input);
        assert!(!output.contains("\n\n"), "Should not add blank lines");
    }

    #[test]
    fn test_comment_preserved() {
        let input = "# This is a comment\naction, foo=bar";
        let output = fmt(input);
        assert!(output.starts_with("# This is a comment\n"));
    }

    #[test]
    fn test_long_comment_wrapped() {
        let long_comment="# This is a very long comment that exceeds 150 characters and should be wrapped to multiple lines because we want to keep lines under 150 chars for readability";
        let input = format!("{}\naction, foo=bar", long_comment);
        let output = fmt(&input);
        // Comment should be wrapped to multiple lines
        let lines: Vec<&str> = output.lines().collect();
        assert!(lines[0].starts_with("# "));
        assert!(lines[1].starts_with("# "));
        assert!(lines[0].len() <= DEFAULT_LINE_LENGTH);
        assert!(lines[1].len() <= DEFAULT_LINE_LENGTH);
    }

    #[test]
    fn test_array_inline_short() {
        let input = "action, values=[1, 2, 3]";
        let output = fmt(input);
        assert_eq!(output, "action, values=[1, 2, 3]\n");
    }

    #[test]
    fn test_array_with_structures() {
        // expected-issue should be multiline
        let input = "meta, issues={[expected-issue, level=critical, id=foo]}";
        let output = fmt(input);
        assert!(
            output.contains("[expected-issue,\n"),
            "expected-issue should be multiline: {output}"
        );
        assert!(output.contains("level=critical"));
        assert!(output.contains("id=foo"));
    }

    #[test]
    fn test_semicolon_preserved() {
        let input = "set-vars, foo=\"bar\";";
        let output = fmt(input);
        assert!(output.ends_with(";\n"));
    }

    #[test]
    fn test_typed_value() {
        let input = "action, value=(int)42";
        let output = fmt(input);
        assert!(output.contains("value=(int)42"));
    }

    #[test]
    fn test_spaces_around_equals() {
        let input = "action,foo=bar,baz=123";
        let output = fmt(input);
        assert!(output.contains("foo=bar"));
        assert!(output.contains("baz=123"));
    }

    #[test]
    fn test_idempotent() {
        let input = "meta,\n    handles-states=true,\n    args={\n        \"pipeline\",\n    }\n";
        let output1 = fmt(input);
        let output2 = fmt(&output1);
        assert_eq!(output1, output2, "Formatting should be idempotent");
    }

    #[test]
    fn test_file_ends_with_newline() {
        let input = "action, foo=bar";
        let output = fmt(input);
        assert!(output.ends_with('\n'));
    }

    #[test]
    fn test_trailing_comment_short_stays_on_line() {
        let input = "meta, args={\n    value,  # short\n}";
        let output = fmt(input);
        assert!(
            output.contains("value,  # short"),
            "Short trailing comment should stay on same line"
        );
    }

    #[test]
    fn test_trailing_comment_long_moves_before() {
        let input = "meta, args={\n    [action-with-long-name, param=\"value\"],  # this is a very very very long trailing comment that exceeds the line length limit and should be moved before\n}";
        let output = fmt(input);
        // The comment should appear BEFORE the element it was trailing
        assert!(
            output.contains("# this is a very very very long trailing comment"),
            "Long comment should be preserved"
        );
        assert!(
            output.contains("[action-with-long-name, param=\"value\"],\n"),
            "Element should have comma and newline after, no trailing comment"
        );
        // Verify order: comment comes before element
        let comment_pos = output.find("# this is a very very").unwrap();
        let element_pos = output.find("[action-with-long-name").unwrap();
        assert!(
            comment_pos < element_pos,
            "Comment should appear before element when too long"
        );
    }

    #[test]
    fn test_property_actions_always_multiline() {
        // These short structures should still be multiline
        let input = "check-properties, foo=bar, baz=123";
        let output = fmt(input);
        assert!(
            output.contains(",\n    "),
            "check-properties should always be multiline: {output}"
        );

        let input = "set-properties, foo=bar";
        let output = fmt(input);
        assert!(
            output.contains(",\n    "),
            "set-properties should always be multiline: {output}"
        );

        let input = "check-child-properties, foo=bar";
        let output = fmt(input);
        assert!(
            output.contains(",\n    "),
            "check-child-properties should always be multiline: {output}"
        );

        let input = "set-child-properties, foo=bar";
        let output = fmt(input);
        assert!(
            output.contains(",\n    "),
            "set-child-properties should always be multiline: {output}"
        );
    }

    #[test]
    fn test_expected_issue_always_multiline() {
        let input = "expected-issue, issue-id=foo, level=critical";
        let output = fmt(input);
        assert!(
            output.contains(",\n    "),
            "expected-issue should always be multiline: {output}"
        );
    }

    #[test]
    fn test_quoted_string_to_array_structure_conversion() {
        // Quoted expected-issue strings should be converted to array structures
        let input = r#"meta, expected-issues={
    "expected-issue, issue-id=foo, level=critical",
}"#;
        let output = fmt(input);
        assert!(
            output.contains("[expected-issue,"),
            "Quoted expected-issue should be converted to array structure: {output}"
        );
        assert!(
            !output.contains("\"expected-issue,"),
            "Should not contain quoted expected-issue: {output}"
        );
    }

    #[test]
    fn test_quoted_string_escapes_unescaped() {
        // Escaped quotes and backslashes should be properly unescaped
        let input = r#"meta, expected-issues={
    "expected-issue, issue-id=foo, details=\"test\\\\nvalue\"",
}"#;
        let output = fmt(input);
        // The \" should become " and \\\\ should become \\
        assert!(
            output.contains(r#"details="test\\nvalue""#),
            "Escapes should be properly unescaped: {output}"
        );
    }

    #[test]
    fn test_change_severity_conversion() {
        let input = r#"meta, overrides={
    "change-severity, issue-id=foo, new-severity=warning",
}"#;
        let output = fmt(input);
        assert!(
            output.contains("[change-severity,"),
            "Quoted change-severity should be converted to array structure: {output}"
        );
    }
}
