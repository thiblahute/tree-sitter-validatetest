# tree-sitter-validatetest

GStreamer ValidateTest grammar for [tree-sitter](https://tree-sitter.github.io/).

This grammar parses [`.validatetest` files](https://gstreamer.freedesktop.org/documentation/gst-devtools/gst-validate-test-file.html) used for testing GStreamer pipelines. These files are executed by `gst-validate-1.0`, `ges-launch-1.0`, or any GStreamer tool that supports the validate test format.

## Features

- Full support for GstStructure serialization format
- Comments (`# ...`)
- Variables (`$(variable_name)`)
- Expressions (`expr(...)`)
- Type casts (`(type)value`)
- Property paths (`element.pad::property`)
- Arrays (`[...]` and `<...>`)
- Nested structure blocks (`{...}`)

## Installation

### Neovim (with nvim-treesitter)

Add the parser to your nvim-treesitter configuration:

```lua
local parser_config = require("nvim-treesitter.parsers").get_parser_configs()
parser_config.validatetest = {
  install_info = {
    url = "https://github.com/thiblahute/tree-sitter-validatetest",
    files = {"src/parser.c"},
    branch = "main",
  },
  filetype = "validatetest",
}

vim.filetype.add({
  extension = {
    validatetest = "validatetest",
  },
})
```

Then run `:TSInstall validatetest`.

### Rust

```toml
[dependencies]
tree-sitter-validatetest = "0.1"
```

### Node.js

```bash
npm install tree-sitter-validatetest
```

## Development

```bash
# Install dependencies
npm install

# Generate the parser
npx tree-sitter generate

# Run tests
npx tree-sitter test

# Parse a file
npx tree-sitter parse path/to/file.validatetest
```

## License

MIT
