/**
 * @file GStreamer ValidateTest grammar for tree-sitter
 * @author Thibault Saunier
 * @license MIT
 *
 * Grammar for .validatetest files used by gst-validate-launcher.
 * Based on GstStructure serialization format with additional constructs
 * for variables, expressions, and comments.
 */

/// <reference types="tree-sitter-cli/dsl" />
// @ts-check

module.exports = grammar({
  name: "validatetest",

  extras: ($) => [/\s/, $.line_continuation],

  conflicts: ($) => [
    [$.array_structure],
    [$.array_structure, $.field_value],
    [$.structure],
    [$.structure, $.field_value],
    [$.field_list],
  ],

  rules: {
    // A file is a sequence of entries (structures) and comments
    source_file: ($) => repeat(choice($.structure, $.comment)),

    // Comments start with # and go to end of line
    comment: ($) => seq("#", /.*/),

    // Line continuation with backslash
    line_continuation: ($) => seq("\\", /\r?\n/),

    // A structure is: name, field=value, field=value, ...
    // Can end with semicolon, newline, or EOF
    structure: ($) =>
      seq($.structure_name, optional(seq(",", $.field_list)), optional(";")),

    // Structure name (action type)
    structure_name: ($) => $.identifier,

    // Comma-separated list of fields
    field_list: ($) => sep1($.field, ","),

    // A field is: name = value
    field: ($) =>
      seq(field("name", $.field_name), "=", field("value", $.field_value)),

    // Field name can be a simple identifier or a property path
    field_name: ($) => choice($.property_path, $.identifier),

    // Property path: element.pad::property or element::property
    property_path: ($) =>
      seq(
        $.identifier,
        optional(seq(".", $.identifier)),
        "::",
        $.identifier,
      ),

    // Field value
    field_value: ($) =>
      choice(
        $.typed_value,
        $.value,
        $.array,
        $.angle_bracket_array,
        $.nested_structure_block,
      ),

    // Typed value: (type)value
    typed_value: ($) =>
      seq("(", field("type", $.type_name), ")", field("value", $.value)),

    // Type name for casts
    type_name: ($) => /[a-zA-Z_][a-zA-Z0-9_]*/,

    // A value can be many things
    // Order matters: more specific patterns first, unquoted_string last as fallback
    value: ($) =>
      choice(
        $.string,
        $.hex_number,
        $.fraction,
        $.number,
        $.boolean,
        $.variable,
        $.expression,
        prec(2, $.flags),
        prec(2, $.namespaced_identifier),
        $.cli_argument,
        $.unquoted_string,
      ),

    // CLI arguments like -t, --videosink (used in args blocks)
    cli_argument: ($) => /--?[a-zA-Z][a-zA-Z0-9_-]*/,

    // Double-quoted string with escapes
    // Expression and variable are matched first, then raw text
    string: ($) =>
      seq(
        '"',
        repeat(
          choice(
            $.escape_sequence,
            $.expression,
            $.variable,
            $.string_content,
            "$",  // Lone $ that's not part of $(...)
          ),
        ),
        '"',
      ),

    // String content that's not a special sequence
    // Excludes: " (end), \ (escape), $ (variable start), e (expr start)
    string_content: ($) => /[^"\\$e]+|e/,

    // Escape sequences
    escape_sequence: ($) => /\\./,

    // Variable: $(name) or $(name.subfield)
    variable: ($) => seq("$(", /[a-zA-Z_][a-zA-Z0-9_]*(\.[a-zA-Z0-9_]+)*/, ")"),

    // Expression: expr(...)
    // Handle nested parentheses by matching balanced content
    expression: ($) => token(seq(
      "expr(",
      repeat(choice(
        /[^()]+/,                    // Non-paren characters
        seq("(", /[^()]*/, ")"),     // One level of nested parens
      )),
      ")"
    )),

    // Integer or float
    number: ($) => {
      const integer = /[+-]?[0-9]+/;
      const float = /[+-]?[0-9]+\.[0-9]*/;
      return choice(float, integer);
    },

    // Fraction: num/denom (e.g., 30/1 for framerate)
    fraction: ($) => /[0-9]+\/[0-9]+/,

    // Hexadecimal number
    hex_number: ($) => /0x[0-9a-fA-F]+/,

    // Boolean
    boolean: ($) => choice("true", "false"),

    // Flags: flag1+flag2+flag3
    // Use token to match the whole flags expression as a single token
    flags: ($) => token(seq(
      /[a-zA-Z_][a-zA-Z0-9_-]*/,
      repeat1(seq("+", /[a-zA-Z_][a-zA-Z0-9_-]*/))
    )),

    // Namespaced identifier: namespace::name
    // Use token to match the whole namespaced identifier as a single token
    namespaced_identifier: ($) => token(seq(
      /[a-zA-Z_][a-zA-Z0-9_-]*/,
      "::",
      /[a-zA-Z_][a-zA-Z0-9_-]*/
    )),

    // Unquoted string (bare identifier or value)
    unquoted_string: ($) => /[a-zA-Z_][a-zA-Z0-9_\-.]*/,

    // Basic identifier
    identifier: ($) => /[a-zA-Z_][a-zA-Z0-9_\-]*/,

    // Array: [ item, item, ... ] or [ structure, structure, ... ]
    // Allows trailing commas
    // Prefer array_structure over field_value when ambiguous
    array: ($) =>
      seq(
        "[",
        optional(seq(
          sep1(choice(prec(1, $.array_structure), $.field_value), ","),
          optional(",")  // Allow trailing comma
        )),
        "]",
      ),

    // GstValueArray: < item, item, ... > (angle bracket array)
    angle_bracket_array: ($) =>
      seq(
        "<",
        optional(sep1($.field_value, ",")),
        ">",
      ),

    // Structure inside an array (without the trailing semicolon rules)
    array_structure: ($) =>
      seq($.structure_name, optional(seq(",", $.field_list))),

    // Nested structure block: { structure, structure, ... } or { "string", "string", ... }
    // Note: strings, arrays, and other values are captured via field_value
    // Allows trailing commas and embedded comments
    nested_structure_block: ($) =>
      seq(
        "{",
        repeat(choice(
          $.comment,
          seq(choice($.structure, $.field_value), optional(",")),
        )),
        "}",
      ),
  },
});

/**
 * Creates a rule to match one or more of the rule separated by the separator.
 *
 * @param {Rule} rule
 * @param {Rule} sep
 * @returns {SeqRule}
 */
function sep1(rule, sep) {
  return seq(rule, repeat(seq(sep, rule)));
}