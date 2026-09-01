## Phase 0 — Project Setup

* [ ] Initialize the Cargo project
* [ ] Configure Git
* [ ] Add `.gitignore`
* [ ] Establish the `rully` binary
* [ ] Add basic CLI argument handling
* [ ] Add `--help`
* [ ] Add `--version`

---

## Phase 1 — HTTP Client

**Goal:** send a basic HTTP request and display the response.

Example:

```sh
rully GET https://api.example.com/users
```

Features:

* [ ] GET requests
* [ ] URL arguments
* [ ] HTTP status
* [ ] Response body
* [ ] Response time
* [ ] Network error handling

---

## Phase 2 — Complete HTTP Requests

**Goal:** support the main features needed to test APIs.

### HTTP Methods

* [ ] GET
* [ ] POST
* [ ] PUT
* [ ] PATCH
* [ ] DELETE

### Headers

* [ ] Custom headers
* [ ] Multiple headers
* [ ] Authorization headers
* [ ] Content-Type

Example:

```sh
rully POST https://api.example.com/users \
  --header "Authorization: Bearer xxx" \
  --header "Content-Type: application/json"
```

### Query Parameters

* [ ] Query parameters
* [ ] Multiple parameters
* [ ] Proper URL encoding

### Request Body

* [ ] Raw request body
* [ ] JSON request body
* [ ] Request body from a file

Example:

```sh
rully POST https://api.example.com/users \
  --body '{"name":"John"}'
```

---

## Phase 3 — Output

**Goal:** make the CLI output readable and useful.

* [ ] Pretty-print JSON responses
* [ ] Display status code
* [ ] Display response time
* [ ] Display response size
* [ ] Display content type
* [ ] Add verbose mode
* [ ] Display request headers in verbose mode
* [ ] Display response headers in verbose mode

Example:

```text
→ POST /users

← 201 Created · 142ms · 1.2 KB
```

---

## Phase 4 — Error Handling

**Goal:** provide clear and useful errors.

Handle:

* [ ] Invalid URLs
* [ ] DNS failures
* [ ] Connection failures
* [ ] Timeouts
* [ ] TLS errors
* [ ] Invalid request bodies
* [ ] Missing files
* [ ] Invalid CLI arguments

Errors should be readable and should not expose unnecessary implementation details.

---

## Phase 5 — Collections

**Goal:** save and organize requests.

Example:

```text
my-api/
├── users/
│   ├── list
│   ├── get
│   └── create
└── auth/
    └── login
```

Features:

* [ ] Define a request file format
* [ ] Save requests
* [ ] Load requests
* [ ] Update requests
* [ ] Delete requests
* [ ] Organize requests into collections
* [ ] Execute saved requests

The storage format should be human-readable and Git-friendly.

---

## Phase 6 — Variables & Environments

**Goal:** run the same requests against different environments.

Example:

```text
{{base_url}}/users/{{user_id}}
```

Environments:

```text
local
staging
production
```

Features:

* [ ] Variables
* [ ] Variable interpolation
* [ ] Environment files
* [ ] Environment selection
* [ ] Default environment
* [ ] Secret handling

Secrets must not accidentally be written to logs or displayed in normal output.

---

## Phase 7 — Core Architecture

**Goal:** prepare the project for the TUI.

The architecture should evolve toward:

```text
                RULLY CORE
                    │
          ┌─────────┴─────────┐
          │                   │
         CLI                 TUI
```

The core should contain the application logic for:

* Requests
* Responses
* HTTP execution
* Collections
* Environments
* Application errors

The core should remain independent from:

* CLI argument parsing
* Terminal rendering
* TUI widgets
* CLI-specific output

The goal is to avoid duplicating HTTP logic when the TUI is introduced.

---

## Phase 8 — TUI

**Goal:** launch an interactive API client from the terminal.

Running:

```sh
rully
```

should launch the TUI.

Expected areas:

* [ ] Collections
* [ ] Request editor
* [ ] URL editing
* [ ] HTTP method selection
* [ ] Headers
* [ ] Request body
* [ ] Send request
* [ ] Response viewer
* [ ] JSON formatting
* [ ] Environment selection
* [ ] Request history
* [ ] Keyboard navigation

The TUI should reuse the same core as the CLI.

---

## Phase 9 — Quality

**Goal:** make Rully reliable and pleasant to use.

### Testing

* [ ] Unit tests
* [ ] Integration tests
* [ ] Request/response tests
* [ ] Collection tests
* [ ] Environment tests
* [ ] Error handling tests

### CLI

* [ ] Improve help output
* [ ] Shell completions
* [ ] Machine-readable output
* [ ] Quiet mode

### TUI

* [ ] Search
* [ ] Keyboard shortcuts
* [ ] Better response viewer
* [ ] Request history improvements

---

## Phase 10 — Distribution

**Goal:** make Rully easy to install.

Release binaries for:

* [ ] macOS Apple Silicon
* [ ] macOS Intel
* [ ] Linux
* [ ] Windows

Project:

* [ ] README
* [ ] Installation documentation
* [ ] Usage documentation
* [ ] GitHub releases
* [ ] Automated release builds

Long-term goal:

```sh
brew install rully
```

Rully should be distributed as a native executable.

---

## MVP

If the project starts becoming too large, the first usable version should stop around Phase 3–4.

The MVP should allow:

```sh
rully GET https://api.example.com/users
```

and:

```sh
rully POST https://api.example.com/users \
  --header "Content-Type: application/json" \
  --body '{"name":"John"}'
```

with:

* GET / POST
* Headers
* Request body
* Status code
* Response body
* Response time
* Basic JSON formatting
* Basic error handling

Everything after that can evolve incrementally.

---

## Long-Term Vision

Rully should provide two complementary experiences:

```text
CLI                          TUI
 │                            │
 │ rully GET <url>            │ rully
 │                            │
 └──────────┬─────────────────┘
            │
        RULLY CORE
            │
       HTTP / Storage /
       Collections /
       Environments
```

The CLI should remain useful for quick and scriptable requests.

The TUI should provide a richer interactive experience for exploring APIs, managing requests and working with collections.
