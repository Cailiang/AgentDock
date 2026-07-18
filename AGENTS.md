# Repository Instructions

- Do not commit, tag, or push to GitHub unless the user explicitly requests it.
- After modifying the repository, run the relevant checks and deploy the verified build with `npm run release:macmini`.
- Every Mac mini release must also create a locally installable universal macOS DMG in `release-artifacts/` and verify that it contains the Intel `x86_64` architecture.
- The default deployment target is `duagent@192.168.14.2:/Applications/AgentDock.app`.
- Only after the user explicitly approves an online release, commit the verified changes and run `npm run release:github`.
- Every GitHub release must use `release-notes/v<version>.md` instead of auto-generated notes. Write an accurate Chinese section first and an English section second; both sections must summarize the release and list new features and bug fixes based on the changes since the previous release.
- Do not publish a GitHub release when its bilingual release notes are missing, incomplete, or still contain template placeholders.
- Keep deployment credentials out of the repository. Use the configured SSH key.
