# Security Policy

## Supported Versions

Security fixes are made on the tip of `main` in this fork.

| Version | Supported |
| --- | --- |
| `main` (this fork) | :white_check_mark: |
| Older snapshots | :x: |
| Upstream nhrbuilds/flashbrowse | :x: (report here first) |

The fork may lag behind upstream releases. When you report a problem, include
the revision you built so we can tell whether it is already fixed upstream.

## Reporting a Vulnerability

Do not open a public issue for a security problem. Email
<developer@lightyagami.dev> with:

- the affected crate and the exact revision you tested;
- reproduction steps and a proof of concept, if available;
- the impact on the CLI or on the MCP server;
- any mitigation you have already applied.

You should receive an acknowledgement within five working days. We will keep
the report private, prepare the fix in this fork and agree on disclosure with
you before publishing details.

Never include tokens, browser profiles, captured pages or other user data in a
public report. If a bug leaks data, describe the path and the affected storage
location without attaching the data.
