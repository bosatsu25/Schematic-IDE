# Security Policy

## Supported versions

Only the latest released version receives security fixes.

| Version | Supported |
| ------- | --------- |
| 0.1.x   | Yes       |

## What counts as a vulnerability

Schematic IDE runs entirely in the browser and opens files that may come from
untrusted sources. Please report, for example:

- a schematic file that makes the app execute script, read other sites' data,
  or send the user's files anywhere;
- a file that crashes or hangs the engine in a way the app does not recover
  from (the app is expected to show an error instead);
- problems in the service worker that could serve tampered content.

## Reporting

Please report privately through GitHub:
**Security → Report a vulnerability** on this repository
(<https://github.com/bosatsu25/Schematic-IDE/security/advisories/new>).
Do not open a public issue for a suspected vulnerability.

Include the affected version (shown in the app's status bar), a description,
and a file or steps that reproduce the problem. You can expect an initial
reply within 7 days. Confirmed issues are fixed in a new release and credited
in the changelog unless you prefer otherwise.
