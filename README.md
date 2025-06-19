# Org Server

A Rust-based server for parsing Org-mode files and managing reminders with JSON export capabilities.

## Features

### Core Functionality
- **Org-mode File Parser**: Comprehensive parsing of Org-mode syntax using Pest grammar
- **Reminder System**: Desktop notifications for scheduled tasks and deadlines
- **File Monitoring**: Automatic detection of changes in Org-mode files
- **Web Server**: Basic HTTP server for future API extensions

### JSON Export (New!)
- **Command-line JSON Export**: Convert Org-mode files to structured JSON format
- **Pretty-print Support**: Human-readable formatted JSON output
- **Flexible Configuration**: Control output format, depth limits, and included information
- **File and Console Output**: Export to files or display in terminal

## Installation

```bash
# Clone the repository
git clone https://github.com/mopemope/org-server.git
cd org-server

# Build the project
cargo build --release

# The binary will be available at target/release/org-server
```

## Usage

### JSON Export Mode

Convert Org-mode files to JSON format:

```bash
# Basic JSON export to console
org-server parse example.org

# Pretty-printed JSON export to file
org-server parse example.org --output output.json --pretty

# Include position information and control depth
org-server parse example.org --include-position --max-depth 5 --pretty

# Include empty sections in output
org-server parse example.org --include-empty-sections --pretty
```

#### JSON Export Options

- `--output <FILE>`: Output file path (default: stdout)
- `--pretty`: Format JSON with indentation for readability
- `--include-position`: Include position information in JSON output
- `--include-empty-sections`: Include sections with no content
- `--max-depth <N>`: Maximum depth for nested sections (default: 10)

### Server Mode

Run as a background service for file monitoring and reminders:

```bash
# Start server with default configuration
org-server server

# Start server with custom configuration
org-server server --config /path/to/config.toml --port 8080 --host 0.0.0.0
```

#### Server Options

- `--config <FILE>`: Configuration file path
- `--port <PORT>`: Server port (default: 3000)
- `--host <HOST>`: Server host (default: 127.0.0.1)

## Supported Org-mode Elements

The parser supports a comprehensive set of Org-mode syntax:

### Document Structure
- **Headlines**: `*`, `**`, `***` etc. with titles
- **Sections**: Hierarchical document organization
- **Keywords**: `#+TITLE:`, `#+AUTHOR:`, `#+DATE:`, etc.

### Content Elements
- **Text Content**: Plain text with formatting
- **Hyperlinks**: `[[URL][Description]]` format
- **Lists**: Bullet points and checkboxes
- **Properties**: `:PROPERTIES:` blocks with key-value pairs
- **Drawers**: `:LOGBOOK:`, custom drawers

### Scheduling
- **SCHEDULED**: `SCHEDULED: <2024-01-15 Mon 09:00>`
- **DEADLINE**: `DEADLINE: <2024-01-20 Sat>`
- **Time Stamps**: Active and inactive timestamps

### Advanced Features
- **Unicode Support**: Full support for international characters
- **Deep Nesting**: Configurable depth limits for complex documents
- **Error Handling**: Robust parsing with detailed error messages

## JSON Output Format

The JSON export creates a structured representation of your Org-mode files:

```json
{
  "filename": null,
  "id": null,
  "title": "Document Title",
  "keywords": [
    {"key": "TITLE", "value": "Document Title"},
    {"key": "AUTHOR", "value": "Author Name"}
  ],
  "sections": [
    {
      "id": "section-id",
      "headline_symbol": "*",
      "title": "Section Title",
      "properties": [...],
      "contents": [...],
      "scheduling": [...],
      "sections": [...]
    }
  ]
}
```

## Configuration

Create a configuration file for server mode:

```toml
# org-server.toml
server_port = 3000
reminder_intervals = [1800, 600, 60]  # 30min, 10min, 1min before

[paths]
org_files = ["~/Documents/*.org"]
```

## Development

### Project Structure

```
org-server/
├── parser/          # Org-mode parsing library
│   ├── src/
│   │   ├── parser.rs      # Core parsing logic
│   │   ├── json_conversion.rs  # JSON export functionality
│   │   └── reminder.rs    # Reminder extraction
│   └── org.pest     # Pest grammar definition
└── server/          # Main application
    ├── src/
    │   ├── main.rs         # Application entry point
    │   ├── cli.rs          # Command-line interface
    │   ├── json_output.rs  # JSON export implementation
    │   └── ...
    └── Cargo.toml
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test module
cargo test json_output

# Run with output
cargo test -- --nocapture
```

### Building

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Check without building
cargo check
```

## Examples

### Example Org-mode File

```org
#+TITLE: Project Management
#+AUTHOR: John Doe
#+DATE: 2024-01-01

* Tasks
:PROPERTIES:
:ID: tasks-section
:CATEGORY: work
:END:

This section contains project tasks.

** Important Task
SCHEDULED: <2024-01-15 Mon 09:00>
DEADLINE: <2024-01-20 Sat>

This is a critical task that needs attention.

*** Subtask
- [ ] Review requirements
- [X] Initial research completed

** Another Task
:LOGBOOK:
CLOCK: [2024-01-10 Wed 10:00]--[2024-01-10 Wed 12:00] =>  2:00
:END:

Task with time tracking information.

* Notes
:PROPERTIES:
:ID: notes-section
:END:

** Learning Resources
Check out the [[https://orgmode.org/][Org-mode website]] for more information.

- Emacs integration
- Mobile apps
- Export formats
```

### Corresponding JSON Output

The above Org-mode file would be converted to a structured JSON format containing all the hierarchical information, properties, scheduling data, and content.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Submit a pull request

## License

This project is licensed under MIT/Apache-2.0.

## Changelog

### v0.1.0 (Current)
- ✅ Core Org-mode parsing functionality
- ✅ Reminder system with desktop notifications
- ✅ File monitoring and change detection
- ✅ Basic web server
- ✅ **NEW**: Command-line JSON export functionality
- ✅ **NEW**: Pretty-print JSON formatting
- ✅ **NEW**: Configurable output options
- ✅ **NEW**: Unicode and international character support
