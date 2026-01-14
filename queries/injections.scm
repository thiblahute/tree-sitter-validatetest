; Injections for GStreamer ValidateTest files
; This file can be used to inject other language parsers (like gstlaunch for pipeline strings)

; Inject gstlaunch parser for pipeline description strings in args field
; Uncomment when tree-sitter-gstlaunch is available and configured
; ((field
;   (field_name (identifier) @_field_name)
;   (field_value (value (string) @injection.content)))
;  (#eq? @_field_name "args")
;  (#set! injection.language "gstlaunch"))
