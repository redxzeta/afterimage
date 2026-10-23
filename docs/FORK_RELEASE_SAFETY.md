# Fork release containment

This fork retains upstream package names and installers for sync compatibility.
It does not have a separate Afterimage distribution:

- `npm/package.json` and platform packages use the upstream `@intuit` scope.
- `install.sh`, `install.ps1`, and the npm fallback download upstream Infigraph.
- The installers can register MCP configuration and run migration. Do not use
  them to test a candidate build; use direct binaries and disposable state.

The release workflow allows release creation and npm publishing only in
`intuit/infigraph` on `https://github.com`. Its driver and native build jobs
depend on release creation, so fork tag pushes skip the entire release chain.
The npm job also checks the repository independently.

The local `release.sh` refuses to proceed unless origin matches its configured
release destination. A checkout of `redxzeta/afterimage` therefore fails before
building, creating a release, or uploading anything. Missing and unrecognized
origins fail closed. Existing upstream HTTPS and SSH origins, including the
documented enterprise host/owner overrides, retain their behavior.

These guards do not authorize a release. A future fork distribution needs its
own reviewed package scope, artifact origin, installers, and release policy.
