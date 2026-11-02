# Security

## Supported versions

Only the latest commit on `main` is supported.

## Reporting a vulnerability

Please don't open a public issue. Report it privately through [GitHub's security advisories](https://github.com/jonorl/carmen-esp32/security/advisories/new), or email hello@jonathan-orlowski.dev.

Say what's affected, how to reproduce it, and what an attacker could do with it. You'll get a reply within a week.

Out of scope: the WiFi network name and password, which are built into the firmware from `cfg.toml` (kept out of git) and can be read from a device someone holds; and anyone being able to record arrests under any name, which is how the bureau's server works (see [Carmen's SECURITY.md](https://github.com/jonorl/carmen/blob/main/SECURITY.md)).
