# GitHub data spike

The spike is a Rust collector that retrieves bounded GitHub GraphQL snapshots and
stores them in SQLite. Docker is the supported development and execution path;
host Rust and SQLite installations are optional.

## Run

Authenticate `gh`, expose its token for this command, and run the one-shot service:

```sh
mkdir -p data
GH_TOKEN="$(gh auth token)" docker compose run --rm collector
```

The repository's `./data` directory is mounted at `/data`; the default database is
`data/review-radar.sqlite3`. Both the directory and `.env` are gitignored. Do not
put a token in `compose.yaml`, a Docker build argument, or a committed file.

Use smaller bounds while changing the query:

```sh
GH_TOKEN="$(gh auth token)" docker compose run --rm collector \
  --database /data/review-radar.sqlite3 \
  --page-size 5 --max-pages 1 --event-limit 5
```

Build and verify without contacting GitHub:

```sh
docker compose build
docker build --target builder -t review-radar-builder .
docker run --rm review-radar-builder cargo test --workspace --locked
docker run --rm review-radar-builder cargo fmt --all -- --check
```

The runtime image contains only the collector and CA certificates. SQLite is
compiled into the binary, so no database service or host package is needed.

## SQLite model

Each successful run appends one immutable capture in a single transaction:

- `captures`: viewer, capture time, configured bounds, request count, GraphQL cost,
  and remaining rate-limit information.
- `searches`: exact search query, GitHub's reported count, fetched count/pages, and
  whether collection was truncated.
- `pull_requests`: indexed summary fields plus the complete GraphQL PR object as
  JSON. The JSON retains bounded reviews, comments, review threads, review requests,
  latest commit, and check contexts for later normalization experiments.
- `search_memberships`: many-to-many links recording why each PR was discovered.

The database contains real repository names, titles, URLs, actors, node IDs, and
event data. Treat it as private local application data; never commit or publish it.
Schema setup is idempotent and uses foreign keys and WAL mode. A request or parsing
failure writes no partial capture. Captures are not atomic with respect to GitHub:
data may change while the six searches are collected.

## Generalized test seed

`tests/fixtures/github/github-snapshot.json` is generated from the latest local
capture. It keeps state, timestamps, counts, nulls, truncation indicators, event
relationships, and overlapping search memberships, but replaces all repository,
actor, team, title, URL, node-ID, commit-SHA, and check-name values with deterministic
pseudonyms. The fixture is safe to commit; the source SQLite database is not.

Regenerate it after intentionally replacing or expanding the source capture:

```sh
mkdir -p tests/fixtures/github
docker compose run --rm seed-export
docker build --target builder -t review-radar-builder .
docker run --rm review-radar-builder cargo test --workspace --locked
```

The exporter uses the latest capture unless `--capture-id ID` is passed. Alias
assignment is deterministic for a given capture but may change when its contents
change, so review the entire fixture replacement. Export validation and fixture
tests reject identities that do not match generalized forms.

Example inspection with a temporary SQLite container:

```sh
docker run --rm -v "$PWD/data:/data" keinos/sqlite3 \
  sqlite3 'file:/data/review-radar.sqlite3?immutable=1' \
  'select id, captured_at, request_count, graphql_cost from captures order by id desc limit 5;'
```

## Queries and bounds

The field selection is in `crates/github/src/query.graphql`. Every search sorts by
`updated-desc`:

| Search key | Qualifiers after `is:pr` | Meaning |
| --- | --- | --- |
| `review_requested` | `is:open review-requested:LOGIN` | Current review obligations |
| `authored` | `is:open author:LOGIN` | Authored open PRs, including drafts |
| `involved` | `is:open involves:LOGIN -author:LOGIN` | Non-authored participation |
| `review_involved` | `is:open review-involves:LOGIN -author:LOGIN` | Non-authored review participation |
| `recent` | `is:closed involves:LOGIN closed:>=CUTOFF` | Recent involved completions |
| `recent_review_involved` | `is:closed review-involves:LOGIN closed:>=CUTOFF` | Recent review-related completions |

`CUTOFF` is capture start minus 14 days in UTC. `is:closed` includes merged PRs.
Combine both participation searches for following candidates and both recent
searches for recent completions.

- Defaults are 25 PRs/page, at most 4 pages/search (100 results/search), and the
  latest 20 reviews, issue comments, review threads, and comments per thread.
- Current review requests and latest-commit check contexts are bounded at 100.
- Defaults permit at most 25 requests: one viewer query plus six searches of four
  pages. GitHub search itself exposes at most 1,000 results per search.
- `truncated` is set when another search page exists or the reported count exceeds
  the distinct fetched IDs. Nested connection page flags and total counts remain
  inside `pull_requests.payload`; nested connections are not paginated yet.
- Overlapping search results are deduplicated by GraphQL node ID while memberships
  are retained. A later search response wins when the same PR changes during a run.
- Requests are sequential with a 120-second timeout and no retry. GraphQL errors,
  including partial-data responses, abort the capture.

## Known data gaps

- **Following is only a candidate union.** `involves:` covers author, assignee,
  mention, and commenter relationships; `review-involves:` adds review activity.
  Neither is a complete manual-subscription feed.
- **Team review requests need care.** Search may include requests through one of
  the viewer's teams. Retain search membership as evidence rather than expecting a
  direct user entry in `reviewRequests`.
- **Unknown is not healthy.** Mergeability may be `UNKNOWN`; null review decisions
  and check rollups are distinct from approved or successful states. Later polls
  may resolve unknown values.
- **Merge readiness is provisional.** The snapshot does not include every ruleset,
  required-check policy, merge-queue state, or viewer permission. `CLEAN` alone is
  not proof the viewer can merge.
- **New feedback requires stored history.** Event IDs and timestamps can be compared
  across captures, but `updatedAt` is not a meaningful-event fingerprint.
- **Bounded event tails can omit relevant activity.** A new reply may belong to an
  older omitted thread. Complete event classification will need targeted pagination
  or timeline retrieval.
- **Current state is not history.** Request/removal times and intermediate CI or
  mergeability transitions between captures are unavailable.

Next: normalize persisted payloads into Rust domain types while preserving unknown
values, search relationships, and completeness metadata.
