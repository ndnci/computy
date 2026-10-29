# Agent instructions

- After each task that changes the repository, run appropriate checks and create a commit unless the user explicitly instructs otherwise.
- Each commit must have a single author: use the user's existing Git identity without replacing it.
- Never add `Co-authored-by`, a coauthor, or any signature or trailer attributing work to an agent, AI or tool.
- Write a clear commit message describing the changes. Do not push commits without an explicit user request.
- Maintain project documentation in English. Keep the application's supported UI languages and localization catalogs intact.
- Record user-facing changes in root `CHANGELOG.md` using Keep a Changelog sections. Keep release tags, package versions and dated changelog entries consistent.
