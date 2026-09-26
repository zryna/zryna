# Local beta pilot report template

Keep this report local until its contents have been reviewed for disclosure. Copy a
blank report for each host and exact candidate. Do not paste source, secrets, access
tokens, user names, private paths, environment dumps or unsanitized logs.

## Candidate and host

| Field | Value |
| --- | --- |
| Date (UTC) | |
| Candidate archive filename | |
| Candidate archive SHA-256 (independently supplied / locally calculated) | / |
| `setup.json` SHA-256 (independently supplied / locally calculated) | / |
| Compiler, server, editor versions reported | |
| OS release and CPU architecture | |
| Host type (clean image / existing machine) | |
| Linux GNU toolchain version, if native run attempted | |
| Editor version and isolated profile used, if applicable | |
| Checkout revision, only for repository-local comparison | |

## Installation and fixed workflows

Use `pass`, `fail`, `blocked`, or `not run` for each row. For failures, record the
stable diagnostic code and exit status. Record elapsed time only when measured;
state the measurement method. Keep candidate and checkout results separate.

| Lane | Profile | Target / editor action | Input | Expected | Status | Actual scalar or diagnostic; exit status | Elapsed / method |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Candidate install and `--version` | setup / M1 | host | reviewed digests | admitted; expected version | | | |
| Candidate CLI | default M1 | JavaScript | `main()` | `i32 42` | | | |
| Candidate CLI | default M1 | WebAssembly | `main()` | `i32 42` | | | |
| Candidate editor | scalar | JavaScript Run | `add(13, -4)` | `i32 9` | | | |
| Candidate editor | scalar | WebAssembly Run | `double(13)` | `i32 26` | | | |
| Candidate editor | M2 control flow | JavaScript Run | `accumulate(true, 5)` | `i32 10` | | | |
| Candidate editor | M2 control flow | WebAssembly Run | `accumulate(false, 3)` | `i32 -6` | | | |
| Candidate editor | scalar / M2 | format, diagnostic, save | guide exercise | guide observations | | | |
| Checkout comparison | default M1 | JavaScript | `add(20, 22)` | `i32 42` | | | |
| Checkout comparison | M2 control flow | WebAssembly | `choose(true, 21)` | `i32 42` | | | |
| Checkout comparison | M3 data ownership | JavaScript | `score(2, 3)` | `i32 65` | | | |
| Checkout comparison | M3 data ownership | WebAssembly | `score(2, 3)` | `i32 65` | | | |
| Optional supported Linux host | selected profile | native | documented example | same scalar result | | | |

## Findings and follow-up

| Field | Value |
| --- | --- |
| Installation or compatibility observation | |
| Diagnostic quality observation (code and location only) | |
| Performance observation and measurement method | |
| Security observation (no exploit details here) | |
| Known limitation encountered | |
| Blocker / follow-up issue URL and disposition | |
| Retest candidate digest and result, if applicable | |

If a problem needs a reproducer, make the smallest synthetic project that shows it.
Request consent before collecting any participant source. Sanitize commands, paths
and logs before filing a public issue. Report suspected vulnerabilities through
the repository's private security channel, not a public issue.
