; Highlights for GStreamer ValidateTest files

; Comments
(comment) @comment

; Structure/action names (function calls - actions are "called" with field arguments)
(structure_name
  (identifier) @function.call)

(array_structure
  (structure_name
    (identifier) @function.call))

; Field names
(field_name
  (identifier) @label)

(field_name
  (property_path
    (identifier) @label))

; Type names in casts
(typed_value
  (type_name) @type)

; Strings (quoted)
(string) @string

; Unquoted string values (use same as quoted strings)
(unquoted_string) @string

; Escape sequences within strings
(escape_sequence) @string.escape

; Numbers
(number) @number
(hex_number) @number
(fraction) @number

; Booleans
(boolean) @boolean

; Variables like $(foo)
(variable) @variable

; Expressions like expr(...)
(expression) @function.call

; Flags (like flush+accurate)
(flags) @constant

; Namespaced identifiers (like scenario::execution-error)
(namespaced_identifier) @module

; CLI arguments (like -t, --videosink)
(cli_argument) @attribute

; Operators and punctuation
"=" @operator
"::" @punctuation.delimiter

; Brackets and braces
"[" @punctuation.bracket
"]" @punctuation.bracket
"{" @punctuation.bracket
"}" @punctuation.bracket
"(" @punctuation.bracket
")" @punctuation.bracket
"<" @punctuation.bracket
">" @punctuation.bracket

; Separators
"," @punctuation.delimiter
";" @punctuation.delimiter
