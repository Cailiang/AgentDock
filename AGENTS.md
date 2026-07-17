# Repository Instructions

- Do not commit, tag, or push to GitHub unless the user explicitly requests it.
- After modifying the repository, run the relevant checks and deploy the verified build with `npm run release:macmini`.
- Every Mac mini release must also create a locally installable universal macOS DMG in `release-artifacts/` and verify that it contains the Intel `x86_64` architecture.
- The default deployment target is `duagent@192.168.14.2:/Applications/AgentDock.app`.
- Only after the user explicitly approves an online release, commit the verified changes and run `npm run release:github`.
- Keep deployment credentials out of the repository. Use the configured SSH key.
