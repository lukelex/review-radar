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

## Ranked workspace output

The queue command reads a successful capture without making a GitHub request. It
reconstructs the normalized snapshot from SQLite payloads and search memberships,
then delegates filtering and ranking to `crates/domain`.

```sh
docker compose run --rm queue
docker compose run --rm queue --view action
docker compose run --rm queue --view my-prs --capture-id 1
docker compose run --rm queue --view action --ranking newest-activity
docker compose run --rm queue --view my-prs --ranking highest-friction
docker compose run --rm queue --record-attention true
```

It prints a stable JSON view model with the selected capture, ranked cards, top
action, relationship memberships, and ordered review/comment events. The supported
views are `tailored`, `action`, `my-prs`, `following`, and `recent`. The latest
capture is the default; `--capture-id` makes historical inspection reproducible.
This command is read-only and reports an error when there is no successful capture.

For an authored PR, queue compares review/comment event fingerprints with the
immediately previous successful capture. A non-self, non-bot substantive review,
issue comment, or review-thread comment becomes `new-feedback` only when it is
absent from that predecessor **and** has a source timestamp after the predecessor
capture. The first capture, a PR absent from its predecessor, and an event at or
before the predecessor cutoff are a deliberate no-event baseline. Pending reviews
and bot/self activity do not qualify. This conservative rule prevents a bounded
event tail from becoming an obligation merely because it was first observed.

Queue projection applies the separate local state database at
`data/review-radar-state.sqlite3`: acknowledged cards and non-expired snoozes are
omitted, and `suppressedCount` explains the difference from `sourceCount`. Each
card's `currentFingerprint` covers its latest activity and current health signals,
so a changed review, check, merge, or action signal reactivates it. The default
command does not change attention-observation state. Run with
`--record-attention true` only after a successful capture to baseline or persist
attention transitions; `notificationEligibleIds` then contains only new
non-attention → attention transitions.

`tailored` is the default ranking policy: priority bands, then newest activity.
`newest-activity` is a deliberately separate chronological policy that demonstrates
the plug-in boundary; it preserves a view's membership while changing only its
order. Ranking implementations live in `crates/domain/src/ranking.rs` and clients
should persist the policy identifier, not recreate a comparator.

`highest-friction` orders measured High/Moderate/Low assessments first (longest
review duration breaks level ties), followed by Limited history and Not assessed.
It crosses priority bands within the selected view without changing membership.
Current captures lack complete revision and ready/draft history, so they report
Limited history rather than fabricated levels. See the
[history spike](review-friction-data-spike.md) for measurements and collection gaps.

Cards also include `explanation`: personally relevant reasons, source-field
evidence, next-action destinations, and concurrent health. `reviewFriction`
contains the versioned assessment and its evidence/limitations. These fields do
not alter attention-state fingerprints or notification eligibility.

## Local state commands

The local-state command uses no GitHub token and persists only the user's device
state. Pass the `currentFingerprint` returned with the projected card, rather than
deriving a fingerprint in a client:

```sh
docker compose run --rm state acknowledge \
  --pull-request-id PR_NODE_ID --fingerprint CURRENT_FINGERPRINT
docker compose run --rm state snooze \
  --pull-request-id PR_NODE_ID --fingerprint CURRENT_FINGERPRINT \
  --until 2026-09-22T09:00:00Z
docker compose run --rm state clear-snooze --pull-request-id PR_NODE_ID
```

`acknowledge` and `snooze` require a fingerprint. If the next capture projects a
different fingerprint, the card is automatically visible again.

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
- **Feedback deltas are transient capture comparisons.** The current projection
  identifies feedback that arrived since the predecessor; it does not yet retain
  an unacknowledged event as an outstanding queue reason across later captures.
  That needs an explicit local-state design, separate from notification baselines.
- **Current state is not history.** Request/removal times and intermediate CI or
  mergeability transitions between captures are unavailable.

Next: normalize persisted payloads into Rust domain types while preserving unknown
values, search relationships, and completeness metadata.
