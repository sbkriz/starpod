Create a new release for the Starpod workspace.

## Arguments
- $ARGUMENTS: The new version number (e.g. "0.1.3"). Required.

## Steps

1. **Bump version** in the root `Cargo.toml`:
   - Update `[workspace.package] version` to the new version
   - Update all `[workspace.dependencies]` version fields to match
   - Use replace-all to change every occurrence of the old version string

2. **Verify** the workspace compiles: run `cargo check`

3. **Commit** the changes:
   ```
   git add Cargo.toml Cargo.lock
   git commit -m "chore: bump version to <version>"
   ```

4. **Tag** the release:
   ```
   git tag -a v<version> -m "Release v<version>"
   ```

5. **Push** commit and tag:
   ```
   git push origin main
   git push origin v<version>
   ```

6. **Create GitHub release** using `gh release create`:
   - Title: `v<version>`
   - Body: changelog generated from `git log` between the previous tag and the new one
   - Format each commit as a bullet point with its type prefix (fix, feat, chore, etc.)

7. **Publish to crates.io** in dependency order (each with `cargo publish -p <crate>`):
   1. `starpod-hooks`, `starpod-core`, `starpod-browser` (no workspace deps)
   2. `starpod-agent-sdk` (depends on starpod-hooks)
   3. `starpod-db`, `starpod-vault`, `starpod-memory`, `starpod-skills`, `starpod-session`, `starpod-cron`, `starpod-instances` (depend on starpod-core)
   4. `starpod-auth` (depends on starpod-core + dev: starpod-db)
   5. `starpod-agent` (depends on agent-sdk + many starpod-* crates)
   6. `starpod-telegram` (depends on starpod-agent)
   7. `starpod-gateway` (depends on most crates)
   8. `starpod` (CLI binary, depends on all)
   - Wait ~10s between tiers to allow crates.io index to propagate
   - If a publish fails with "already exists", skip it and continue
