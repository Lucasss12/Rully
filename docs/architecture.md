# Architecture

## Crate layout

The project is split across two Cargo targets.

```text
┌────────────────────────────────────────────────────┐
│ rully_core   (lib)                                 │
│                 errors · http · request · response │
│                 network I/O, never the terminal    │
└───────────────────────▲────────────────────────────┘
                        │ depends on
┌───────────────────────┴────────────────────────────┐
│ rully        (bin)                                 │
│                 cli · output · style               │
│                 owns argv and stdout               │
└────────────────────────────────────────────────────┘
```

`rully_core` is the only crate that touches the network, and it never
touches the terminal. `rully` is the only crate that reads argv and writes
stdout. `main.rs` owns nothing but the wiring between the two.

That boundary is what makes `tests/http.rs` possible: the integration tests
drive the real HTTP pipeline without spawning a binary or capturing stdout.

## The pipeline

```text
argv
 │
 ▼
┌────────────────────────────────────────────────────┐
│ cli.rs          terminal adapter (Clap)            │
│                 Cli { method, url, headers,        │
│                       queries, body, body_file,    │
│                       verbose }                    │
└───────────────────────┬────────────────────────────┘
                        │ Request::try_from(&cli)
                        │   Url::parse      → InvalidUrl
                        │   add_query       → InvalidQuery
                        │   add_headers     → InvalidHeader
                        │   resolve_body    → FileRead / ConflictingBodyOptions
                        ▼
┌────────────────────────────────────────────────────┐
│ request.rs      INPUT MODEL                        │
│                 Request { method, url, headers,    │
│                            body }                  │
└───────────────────────┬────────────────────────────┘
                        │ http::default_client()
                        ▼
┌────────────────────────────────────────────────────┐
│ http.rs          CLIENT FACTORY                    │
│                 default_client()                   │
│                 one Client per process, reused     │
│                   connect timeout   10s            │
│                   read    timeout   30s            │
│                   build error → ClientBuild        │
└───────────────────────┬────────────────────────────┘
                        │ http::execute(&client, &request)
                        ▼
┌────────────────────────────────────────────────────┐
│ http.rs          THE CORE                          │
│                 1. validate_json_body → InvalidJson│
│                 2. build_request(&client, request) │
│                      .request(method, url)         │
│                      .headers(..) / .body(..)      │
│                 3. Instant::now()  ← clock starts  │
│                 4. .send().await                   │
│                      is_timeout → Timeout          │
│                      else       → Network(msg)     │
│                 5. status + headers + bytes        │
│                      error      → ResponseBody     │
└───────────────────────┬────────────────────────────┘
                        │ Ok(Response)
                        ▼
┌────────────────────────────────────────────────────┐
│ response.rs     OUTPUT MODEL                       │
│                 Response { status, headers, body,  │
│                             size, duration }       │
│                 new()         from_utf8 → lossy    │
│                 content_type()  drops charset      │
│                 duration_ms()                      │
└───────────────────────┬────────────────────────────┘
                        │ output::display_response(&response, &request, verbose)
                        ▼
┌────────────────────────────────────────────────────┐
│ output.rs       terminal adapter (stdout)          │
│                 verbose → request line + headers    │
│                 summary  → status·time·size·ct     │
│                 verbose → response headers         │
│                 body    → pretty JSON if ct=json   │
└───────────────────────┬────────────────────────────┘
                        ▼
                     stdout
```

The client is built once per process and passed into `execute`, so every
request shares one connection pool. `main.rs` builds it after the `Request`
so that an invalid URL fails before any socket is opened.

## Testing

Unit tests live in `src/`, next to the code they cover. They cover
everything that needs no socket: argument parsing, request building,
response parsing, error mapping.

`tests/http.rs` covers what does need one. `reqwest` cannot be mocked, so
the tests start a real server on `127.0.0.1:0` and make real requests to
it.

`tests/support/mod.rs` is that server. It binds an ephemeral port, then
answers each connection with one canned HTTP response written as a literal
string. It never parses HTTP; it only records the request it received and
writes bytes back. Two modes:

- `TestServer::start(responses)` serves the given responses in order, which
  is how the redirect test gets a 302 followed by a 200
- `TestServer::start_hanging()` accepts the connection and never answers,
  which is how the timeout test provokes a real timeout rather than a
  simulated one

Because the port is ephemeral, every test gets its own server and the whole
suite can run in parallel.
