# Security Policy

## Supported Versions

AgentDock is under active early development. Security fixes are applied to the latest released version only.

## Reporting a Vulnerability

Use GitHub's private vulnerability reporting under the repository's **Security** tab when it is available. If private reporting is unavailable, open a minimal issue asking the maintainers for a private contact channel. Do not include exploit details, API keys, access tokens, private configuration, or diagnostic archives in a public issue.

Include the affected AgentDock version, operating system, reproduction steps, impact, and a sanitized proof of concept. Remove credentials and personal file paths before attaching logs or diagnostics.

## Local Secrets

Provider credentials are runtime user data and must never be committed to this repository. AgentDock diagnostic exports redact configured secret values, but users should still inspect every report before sharing it.
