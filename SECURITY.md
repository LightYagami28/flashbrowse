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

Do not open a public issue for a security problem. Use GitHub's private
vulnerability reporting for this fork
([Report a vulnerability](https://github.com/LightYagami28/flashbrowse/security/advisories/new))
with:

- the affected crate and the exact revision you tested;
- reproduction steps and a proof of concept, if available;
- the impact on the CLI or on the Model Context Protocol (MCP) server;
- any mitigation you have already applied.

You should receive an acknowledgement within five working days. We will keep
the report private, prepare the fix in this fork and agree on disclosure with
you before publishing details.

Keep tokens, browser profiles, captured pages and other user data out of a
public report. If a bug leaks data, describe the path and the affected storage
location instead of attaching the data.
