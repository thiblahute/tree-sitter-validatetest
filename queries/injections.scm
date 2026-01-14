; Injections for GStreamer ValidateTest files
; Re-parse embedded GstStructure syntax within strings

; Strings in 'configs' field contain GstStructure syntax
((field
  name: (field_name (identifier) @_field_name)
  value: (field_value
    (nested_structure_block
      (field_value (value (string (string_inner) @injection.content))))))
 (#eq? @_field_name "configs")
 (#set! injection.language "validatetest")
 (#set! injection.include-children))

; Strings in 'expected-issues' field contain GstStructure syntax
((field
  name: (field_name (identifier) @_field_name)
  value: (field_value
    (nested_structure_block
      (field_value (value (string (string_inner) @injection.content))))))
 (#eq? @_field_name "expected-issues")
 (#set! injection.language "validatetest")
 (#set! injection.include-children))
