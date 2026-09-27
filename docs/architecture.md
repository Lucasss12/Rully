The pipeline

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
│                            body }                   │
└───────────────────────┬────────────────────────────┘
                        │ http::execute(&request)
                        ▼
┌────────────────────────────────────────────────────┐
│ http.rs          THE CORE                          │
│                 1. validate_json_body → InvalidJson│
│                 2. build_request                   │
│                      Client::new()                 │
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
│                 Response { status, headers, body,   │
│                             size, duration }        │
│                 new()         from_utf8 → lossy    │
│                 content_type()  drops charset     │
│                 duration_ms()                     │
└───────────────────────┬────────────────────────────┘
                        │ output::display_response(&response, &request, verbose)
                        ▼
┌────────────────────────────────────────────────────┐
│ output.rs       terminal adapter (stdout)          │
│                 verbose → request headers          │
│                 summary  → status·time·size·ct     │
│                 verbose → response headers         │
│                 body    → pretty JSON if ct=json  │
└───────────────────────┬────────────────────────────┘
                        ▼
                     stdout
```