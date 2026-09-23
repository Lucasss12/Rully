# Rully Roadmap

Rully is a terminal-first HTTP client written in Rust.
It currently provides a CLI and will progressively evolve toward a TUI.

This project is a good excuse for me to build a project in Rust after finishing the book :)

## Current Focus

Finalize the CLI output and error handling, then extract the HTTP engine
into a reusable core shared by the CLI and the future TUI.

## Legend

- [x] Completed
- [~] In progress
- [ ] Planned
- [-] Postponed or intentionally dropped

---

## Phase 0 - Project Setup

- [x] Initialize the Cargo project
- [x] Configure Git and `.gitignore`
- [x] Add the `rully` binary
- [x] Add basic CLI arguments
- [x] Add `--help`
- [x] Add `--version`

---

## Phase 1 - Basic HTTP Client

- [x] Send an HTTP request
- [x] Support `GET`
- [x] Accept a URL argument
- [x] Display the HTTP status
- [x] Display the response body
- [x] Display the response time
- [x] Handle basic network errors

---

## Phase 2 - Complete HTTP Requests

### HTTP Methods

- [x] `GET`
- [x] `POST`
- [x] `PUT`
- [x] `PATCH`
- [x] `DELETE`

### Headers

- [x] Custom headers
- [x] Multiple headers
- [x] Authorization headers
- [x] `Content-Type` header

### Query Parameters

- [x] Query parameters
- [x] Multiple parameters
- [x] Proper value encoding

### Request Body

- [x] Raw request body with `--body`
- [x] JSON request body validation
- [x] Request body from a file with `--body-file`

---

## Phase 3 - CLI Output

**Goal:** make responses readable without coupling the HTTP engine to terminal output.

- [x] Display the status code and status text
- [x] Display the response time
- [x] Display the response size
- [x] Add verbose mode
- [x] Display request headers in verbose mode
- [x] Display response headers in verbose mode
- [x] Display the `Content-Type` in the summary
- [x] Pretty-print JSON responses
- [x] Preserve raw output for other content types
- [x] Improve Clap help and error messages

Example:

```text
<- 200 OK - 142ms - 1.2 KB - application/json

{
  "users": []
}
```

---

## Phase 4 - Reliable Errors

**Goal:** provide clear errors and consistent exit codes.

- [x] Create an application error type
- [x] Distinguish CLI, URL, network, and response errors
- [x] Handle invalid URLs
- [x] Handle DNS and connection failures
- [x] Handle timeouts
- [x] Handle invalid request bodies
- [x] Handle missing files
- [x] Return a non-zero exit code on failure
- [x] Avoid exposing unnecessary implementation details

---

## Phase 5 - Core Architecture

**Goal:** separate the HTTP engine from the CLI so it can be reused by the TUI.

- [x] Extract CLI parsing from `main.rs`
- [x] Extract HTTP request execution
- [ ] Create `Request` and `Response` models
- [~] Preserve status, headers, body, size, and duration
- [x] Return structured errors
- [x] Keep terminal output out of the core
- [ ] Keep the core independent from Clap and Ratatui
- [x] Reduce `main.rs` to application orchestration

Target initial structure:

```text
src/
├── main.rs
├── cli.rs
├── http.rs
├── output.rs
└── error.rs
```

---

## Phase 6 - Quality and MVP

- [ ] Add HTTP integration tests
- [ ] Test requests and responses with a local server
- [ ] Test the main error cases
- [ ] Add a README
- [ ] Document installation
- [ ] Document usage examples
- [~] Pass `cargo fmt`
- [~] Pass `cargo clippy -- -D warnings`
- [~] Pass `cargo test`

The MVP should provide:

- [x] `GET` and `POST` requests
- [x] Custom headers
- [x] Request bodies
- [x] Status and response time
- [~] Basic JSON formatting
- [~] Clear basic error handling

---

## Phase 7 - Collections

- [ ] Define a human-readable, Git-friendly request file format
- [ ] Save requests
- [ ] Load requests
- [ ] Update requests
- [ ] Delete requests
- [ ] Organize requests into collections
- [ ] Execute saved requests

---

## Phase 8 - Variables and Environments

- [ ] Define variables
- [ ] Interpolate variables in requests
- [ ] Add environment files
- [ ] Select an environment
- [ ] Support a default environment
- [ ] Protect secrets from logs and normal output

---

## Phase 9 - TUI

**Goal:** provide an interactive API client built on top of the same core.

- [ ] Launch the TUI with `rully`
- [ ] Display collections
- [ ] Edit requests
- [ ] Edit URLs and methods
- [ ] Edit headers and request bodies
- [ ] Send requests
- [ ] Display responses
- [ ] Format JSON in the response view
- [ ] Select an environment
- [ ] Add request history
- [ ] Add keyboard navigation

---

## Phase 10 - Distribution

- [ ] Finalize the documentation
- [ ] Add automated builds
- [ ] Publish binaries for macOS, Linux, and Windows
- [ ] Prepare GitHub releases
- [ ] Evaluate Homebrew distribution
