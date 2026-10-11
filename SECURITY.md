# Security policy

## Supported versions

Only the latest published version receives security fixes. Before 1.0.0, that is the most
recent minor release.

## Reporting a vulnerability

Please do not open a public issue for a security problem. Use GitHub's private vulnerability
reporting on this repository (Security tab, then "Report a vulnerability"). That gives the
maintainers a private place to discuss and fix the issue before it is disclosed.

Useful reports include:

- An ICC profile, or any other input, that makes the parser panic, loop, or allocate without
  bound. Attach the bytes if you can.
- A transform whose output is wrong in a way that could mislead a print, proof or display
  pipeline.
- Any `unsafe` code, or a C dependency, that has appeared in a shipped crate.

The parsers are fuzzed (`fuzz/`) and covered by a mutation test, but neither is exhaustive.
Reports of gaps are welcome.
