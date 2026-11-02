# Security

## Supported versions

Only what's live is supported: the game at https://jonathan-orlowski.dev/carmen-sandiego/, its records server, the latest releases and the latest commit on `main`.

## Reporting a vulnerability

Please don't open a public issue. Report it privately through [GitHub's security advisories](https://github.com/jonorl/carmen-sandiego/security/advisories/new), or email hello@jonathan-orlowski.dev.

Say what's affected, how to reproduce it, and what an attacker could do with it. You'll get a reply within a week.

Out of scope:

- Anyone being able to record arrests under any name. There are no accounts by design: a name is a detective.
- On the ESP32, the WiFi network name and password, which are built into the firmware from `cfg.toml` (kept out of git) and can be read from a device someone holds.
