# Rully Roadmap

Rully is a terminal API client written in Rust.

The TUI is the product: `rully` launches an interactive client over
collections stored as plain `.http` files under `rully/collections/`, so
they are reviewable in Git and shareable through pull requests.

The CLI is kept for scripting and CI (`rully run`, exit codes). It is
maintained, not developed — the TUI is where features land.

This project is a good excuse for me to build a project in Rust after finishing the book :)

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
- [x] Create `Request` and `Response` models
- [x] Preserve status, headers, body, size, and duration
- [x] Return structured errors
- [x] Keep terminal output out of the core
- [x] Keep the core independent from Clap and Ratatui
- [x] Reduce `main.rs` to application orchestration

---

## Phase 6 - Quality and MVP

- [x] Add HTTP integration tests
- [x] Test requests and responses with a local server
- [x] Test the main error cases
- [x] Pass `cargo fmt`
- [x] Pass `cargo clippy -- -D warnings`
- [x] Pass `cargo test`

The MVP should provide:

- [x] `GET` and `POST` requests
- [x] Custom headers
- [x] Request bodies
- [x] Status and response time
- [x] Basic JSON formatting
- [x] Clear basic error handling

---

## Phase 7 - Collections

- [x] Define a human-readable, Git-friendly request file format
- [x] Save requests
- [x] Load requests
- [x] Update requests
- [x] Delete requests
- [x] Organize requests into collections
- [x] Execute saved requests

---

## Phase 8 - TUI Skeleton

**Goal:** a launchable TUI over features that already exist. Nothing new is
introduced here, so nothing here can surprise us.

- [ ] Add `ratatui`, keep the core independent of it
- [ ] Build the test harness first: synthesised key events, assertions
      against the `TestBackend` buffer
- [ ] Launch the TUI with `rully`
- [ ] Render the collections tree
- [ ] Load a request on selection
- [ ] Display method, URL, headers and body read-only

---

## Phase 9 - TUI Editor

**Goal:** the point where the remaining features get demanded by a screen
rather than by a plan.

- [ ] Body types: json, text, form, none
- [ ] Auth: bearer, basic, api key — and where it lives in `.http`
- [ ] Edit URL and method
- [ ] Edit headers and body
- [ ] Save back to `.http`, keeping comments intact
- [ ] Mouse selection, clipboard via OSC 52

---

## Phase 10 - Environments and History

- [ ] Define variables
- [ ] Interpolate variables in requests
- [ ] Add environment files
- [ ] Select an environment
- [ ] Support a default environment
- [ ] Protect secrets from logs and normal output
- [ ] Request history

---

## Phase 11 - Streaming

**Goal:** render the body as it arrives instead of waiting for it to be
complete. The only structural change the TUI forces on the core.

- [ ] Turn `Response.body` into a channel
- [ ] SSE and chunked responses
- [ ] Tail-follow, cancel in flight

---

## Phase 12 - Distribution

**Goal:** a V1 usable personally, then published.

- [ ] Add a README
- [ ] Document installation
- [ ] Document the key map
- [ ] Add automated builds
- [ ] Publish binaries for macOS, Linux, and Windows
- [ ] Prepare GitHub releases
- [ ] Evaluate crates.io and Homebrew distribution

---

## Later

Once the TUI is stable, the CLI reduces to `rully run <path>` for CI.
