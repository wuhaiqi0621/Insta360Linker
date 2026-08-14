# Insta360Linker Repository Instructions

- All text files must use UTF-8 with CRLF line endings. Do not use GBK.
- Before making any code change, run `git fetch origin`, compare the current `HEAD` with `origin/main`, and inspect the working tree.
- If the local branch is behind GitHub and the tracked working tree can be updated safely, run `git pull --ff-only origin main` before editing. Never overwrite or discard local work.
- After synchronization, read `DEVELOPMENT_LOG.md` and review the incoming changes before continuing from the user's latest request.
- Record each development operation, implementation decision, verification result, build result and cleanup action in `DEVELOPMENT_LOG.md` so another model can continue reliably.
- Perform project work on the F drive. Do not create project artifacts on the C drive.
