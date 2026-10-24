# Import-graph diagnostic fixture

This is synthetic source for static indexing only. Do not run these modules or
install dependencies. It contains no application scripts or hooks.

Expected source module dependencies:

- `extensionless.ts -> helpers.ts`: quoted relative import without an extension.
- `js-suffix.ts -> helpers.ts`: Node-style `.js` source specifier for a TS module.
- `ts-suffix.ts -> helpers.ts`: explicit `.ts` source specifier.
- `barrel.ts -> helpers.ts`: re-export, labelled as a module dependency.
- `workspace.ts -> packages/shared/src/entry.ts`: workspace package `exports`.

Query both `a.file,b.file` and `a.id,b.id` through `Module-[:IMPORTS]->Module`,
and list `Module.id,Module.file` to rule out endpoint/property mismatch. Inspect
these immediately after cold indexing, before incremental indexing can delete
or replace edges. A failed optional analysis stage still makes indexing fail.

The separate SCIP prerequisite probe uses only `helpers.ts` and
`extensionless.ts`, a synthetic package manifest and an explicit `tsconfig.json`.
It copies an already cached indexer plus its dependencies and Node into the
private Bubblewrap lane. This fixture does not imply compiler completeness on
any repository source projection. See the [B/C/E report](../../results/2026-10-23-bce.md)
for commands, exact inputs and observed outcomes.

From the repository root, set `CANDIDATE_CLI` to the saved exact-SHA binary,
`BENCH_SCRATCH` to private scratch, `NODE_BINARY` to the existing Node executable,
and `SCIP_PACKAGE` to the existing `@sourcegraph/scip-typescript` package directory
including its cached `node_modules`. This functionality probe installs nothing.
Reserve the harness lock and run serially; native contention excludes timings.

```sh
python3 - <<'PY'
import fcntl, json, os, shutil, sys, tempfile
from pathlib import Path
sys.path.insert(0, 'scripts/benchmark')
import bench
with open(Path(tempfile.gettempdir()) / f'afterimage-benchmark-{os.getuid()}.lock', 'a') as lock:
    fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
    with tempfile.TemporaryDirectory(dir=os.environ['BENCH_SCRATCH']) as temp:
        lane = bench.Lane(Path(temp)/'ast', Path(os.environ['CANDIDATE_CLI']), None, 90, 1)
        shutil.copytree('scripts/benchmark/fixtures/imports', lane.root/'repo', dirs_exist_ok=True)
        ast = lane.run('ast', ['index', '--no-embed'])
        for query in ['MATCH (m:Module) RETURN m.id,m.file',
                      'MATCH (a:Module)-[:IMPORTS]->(b:Module) RETURN a.file,b.file',
                      'MATCH (a:Module)-[:IMPORTS]->(b:Module) RETURN a.id,b.id']:
            print(json.dumps(lane.query('diagnostic', query)))
        lane = bench.Lane(Path(temp)/'scip', Path(os.environ['CANDIDATE_CLI']), None, 90, 1)
        for name in ['helpers.ts', 'extensionless.ts']:
            shutil.copyfile(Path('scripts/benchmark/fixtures/imports')/name, lane.root/'repo'/name)
        (lane.root/'repo/package.json').write_text('{"name":"afterimage-scip-probe","version":"0.0.0","private":true}')
        (lane.root/'repo/tsconfig.json').write_text('{"compilerOptions":{"strict":true,"target":"ES2020","module":"CommonJS"},"include":["*.ts"]}')
        shutil.copytree(os.environ['SCIP_PACKAGE'], lane.root/'scip')
        shutil.copyfile(Path(os.environ['NODE_BINARY']).resolve(), lane.root/'bin/node')
        (lane.root/'bin/node').chmod(0o500)
        tool = lane.root/'bin/scip-typescript'
        tool.write_text('#!/bin/sh\nexec /bench/bin/node /bench/scip/dist/src/main.js "$@"\n')
        tool.chmod(0o500)
        direct = lane.run('direct_scip', ['index', '--infer-tsconfig', '--output', '/bench/tmp/direct.scip'], executable='scip-typescript')
        artifact = lane.root/'tmp/direct.scip'
        print(json.dumps({'artifact_bytes': artifact.stat().st_size, 'artifact_sha256': bench.digest(artifact)}) if artifact.exists() else 'NO_ARTIFACT')
        scip = lane.run('scip', ['index', '--no-embed'])
        print(json.dumps(lane.query('imports', 'MATCH (a:Module)-[:IMPORTS]->(b:Module) RETURN a.file,b.file')))
        print(json.dumps({'ast': ast, 'direct_scip': direct, 'scip': scip}))
        raise SystemExit(1 if any(r['status'] != 'OK' for r in [ast, direct, scip]) else 0)
PY
```

The combined probe exits 1 when its AST-only index exits 1, even if the separately
provisioned SCIP command succeeds. Query output remains independent evidence.
