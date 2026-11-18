# Releasing

A release is a tagged commit on `main`. Pushing the tag runs
`.github/workflows/release.yml`, which re-runs every check, builds the site and
publishes a GitHub Release with the built site attached as a zip. If the
repository variable `DEPLOY_PAGES` is `true`, it also deploys the site to
GitHub Pages.

## Checklist

1. `main` is green in CI and has no open release-blocking issue.
2. The version in these files is the one being released (they must agree; the
   release workflow checks the tag against `apps/web/package.json`):
   - `package.json`, `apps/web/package.json`, `packages/*/package.json`
   - `Cargo.toml` (`[workspace.package] version`) and each `crates/*/Cargo.toml`
     that sets its own `version`
3. `CHANGELOG.md` has a section for the version; replace `Unreleased` with the
   release date. The workflow uses that section as the release notes.
4. Tag and push:

   ```bash
   git tag -a v0.1.0 -m "Schematic IDE v0.1.0"
   ```

   ```bash
   git push origin v0.1.0
   ```

5. Watch the *Release* workflow. When it finishes, the release page has
   `schematic-ide-v0.1.0.zip` attached.

## Hosting the build

The build uses only relative URLs, so the contents of the zip (or of
`apps/web/dist` after `pnpm build`) can be served by any static file server,
at a domain root or under a sub-path. HTTPS (or `localhost`) is required for
the offline service worker and for clipboard access.

To try a build locally under a sub-path:

```bash
node scripts/serve-static.mjs apps/web/dist 4173 /schematic-ide/
```

### GitHub Pages

1. In the repository settings, set **Pages → Build and deployment → Source**
   to **GitHub Actions**.
2. Add a repository variable `DEPLOY_PAGES` with the value `true`
   (**Settings → Secrets and variables → Actions → Variables**).
3. Push a release tag (or run the *Release* workflow manually). The site is
   published at `https://<owner>.github.io/<repository>/`.

## After the release

- Add a new `Unreleased` section to `CHANGELOG.md`.
- Bump the version in the files listed above when work on the next version
  starts.
