# validatetest-fmt

A formatter for GStreamer ValidateTest files (`.validatetest`).

## Installation

```bash
uv tool install validatetest-fmt
```

## Usage

```bash
# Format files in place
validatetest-fmt -i file.validatetest

# Check if files are formatted (useful for CI)
validatetest-fmt --check file.validatetest

# Read from stdin, write to stdout
cat file.validatetest | validatetest-fmt

# Custom indentation (default: 4 spaces)
validatetest-fmt --indent 2 file.validatetest

# Custom line length (default: 120)
validatetest-fmt --line-length 80 file.validatetest
```

## Pre-commit Hook

Add to your `.pre-commit-config.yaml`:

```yaml
- repo: local
  hooks:
    - id: validatetest-fmt
      name: validatetest-fmt
      language: python
      entry: validatetest-fmt --check
      types_or: [file]
      files: '\.validatetest$'
      additional_dependencies: ["validatetest-fmt>=0.1.0"]
```

## License

MIT
