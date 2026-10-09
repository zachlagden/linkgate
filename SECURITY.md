# Security policy

## Reporting a vulnerability

Report vulnerabilities privately through [GitHub's private vulnerability reporting](https://github.com/zachlagden/linkgate/security/advisories/new). Don't open a public issue for one.

Include the version, the steps to reproduce and what an attacker gains.

## What counts

linkgate's job is to show where a link goes before it opens. These are in scope:

- A link that the window displays as a different domain from the one the browser opens.
- A way to make `linkgate.exe` or `linkgate-open` run a command or open something other than the chosen browser and link.
- A blocklist lookup that misses a listed domain or one of its parent domains.
- Anything that writes a link to disk or sends it over the network. linkgate's only network traffic is the blocklist download.

A domain missing from the Pi-hole Optimized Blocklists is a report for [that repository](https://github.com/zachlagden/Pi-hole-Optimized-Blocklists), not a vulnerability here.

## Supported versions

Only the latest release receives fixes.
