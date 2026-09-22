# Design system and language

Review Radar is a calm, actionable workspace for pull requests. Its design should
help someone answer three questions quickly: **Why is this here? What changed?
What should I do next?**

This document is the baseline for future visual and interaction decisions. It
records the current light-theme direction and guidance for extending it; it is
not a claim that every behavior below has been implemented.

## Sources of truth

- [Product plan](project-plan.md): view membership, priority, and product semantics.
- [Architecture](architecture.md): shared-core and native-shell boundaries.
- [Attention and review friction](attention-and-review-friction.md): evidence and
  explanation semantics.
- [High-fidelity references](mockups/high/README.md): six proposed screens and
  screenshots of the actual Qt implementation. Example data is illustrative.
- [`Style.qml`](../apps/linux-qt/qml/Style.qml): implemented shared color tokens
  and presentation helpers.
- [`docs/assets/logo.svg`](assets/logo.svg): canonical brand asset.

When a mockup and the product contract disagree, preserve the product contract.
Treat screenshots as dated references, not an alternative specification of
business logic.

## Design principles

1. **Explain before decorating.** Personal relevance and the next action should
   be understandable without opening details.
2. **Keep urgency distinct from difficulty.** Attention, PR health, lifecycle,
   and accumulated review friction are separate signals. High friction alone
   does not mean the user owes an action.
3. **Be quiet by default.** Use neutral surfaces, restrained color, and a clear
   hierarchy. Reserve strong emphasis for selection, the next action, and
   genuinely important state changes.
4. **Preserve context.** Keep the queue visible beside details when space allows.
   Retain selection by PR identity when refreshed data still contains that PR.
5. **Be honest about evidence.** Missing history is not low friction; cached data
   is not a successful refresh; unknown checks are not passing checks.
6. **Feel native.** Preserve platform focus, scrolling, menus, clipboard, URL
   opening, and keyboard behavior. Future SwiftUI and WinUI clients should share
   the information hierarchy without copying every Qt pixel.

## Brand

Use the repository SVG directly for the sidebar mark and application icon. It
contains a periwinkle PR branch within a dark indigo radar, with an amber
attention signal.
Preserve its proportions, colors, and built-in rounded background. Do not replace
it with a Unicode radar symbol or redraw it separately in each client.

The logo's indigo and periwinkle intentionally align with the interface's
indigo interaction accent; amber remains a small, dedicated attention cue.
Neither replaces the semantic status colors used in cards. Keep the adjacent
product name **Review Radar** in title case. The current sidebar mark is 34 × 34
logical pixels; render it sharply at the display's scale factor. Decorative marks
beside the product name should not create duplicate screen-reader announcements.

## Color system

| Token | Value | Purpose |
| --- | --- | --- |
| `canvas` | `#f6f7fb` | Workspace background |
| `sidebar` | `#f0f2f8` | Navigation and neutral badge backgrounds |
| Surface | `#ffffff` | Cards, detail panel, toolbar, controls |
| `ink` | `#20283f` | Titles and primary content |
| `secondary` | `#596279` | Supporting content |
| `muted` | `#788197` | Timestamps and tertiary metadata |
| `accent` | `#635bdf` | Primary actions, selected outlines, focus |
| `tint` | `#efedfc` | Attention summaries and accent badges |
| `line` | `#e4e7ef` | Quiet borders and separators |
| Selected navigation | `#e5e3fa` | Active workspace background |
| Amber foreground / background | `#a35b2a` / `#fff0e4` | High-friction and caution treatments |
| Positive foreground / background | `#287a55` / `#e8f6ee` | Passing, approved, and mergeable values |
| Caution foreground / background | `#94631b` / `#fff4dc` | Pending or waiting values |
| Negative foreground / background | `#b33f4a` / `#fdecef` | Failing, conflicting, or requested-change values |
| Friction low foreground / background | `#176b70` / `#e5f4f3` | Low assessed review friction |
| Friction medium foreground / background | `#855b16` / `#fff3d6` | Medium assessed review friction |
| Friction high foreground / background | `#a34420` / `#fceadf` | High assessed review friction |
| Friction unknown foreground / background | `#566176` / `#edf0f5` | Missing or insufficient friction evidence |

Pair color with text or another explicit indicator. Indigo can identify an
interaction or attention summary; it is not evidence of a failing PR. Amber
friction badges must say what they describe. Merged, closed, and open states
retain explicit labels.

Health values are rendered as separate labeled chips so their meaning is
scannable without reading a sentence: `Review`, `Checks`, and `Merge` retain
neutral labels while the values use positive, caution, negative, or neutral
foreground/background pairs. Each colored value also carries a text label and
symbol; color is never the only indicator.

Reserve the negative/red treatment for explicit check failures or errors.
`Changes requested` and merge conflicts are amber follow-up states, using a
revision and warning-triangle symbol respectively; they are visible without
claiming the same urgency as a failed check.

Review friction has a dedicated teal-to-burnt-orange scale, separate from health
status colors. Its badge explicitly says `Friction`, retains the level, and uses
one, two, or three stepped marks for low, moderate, or high. Missing or incomplete
history retains its explicit status (such as `Limited history`) with a question
mark and the slate treatment; it must never receive the low treatment.

New reusable colors should become semantic tokens in `Style.qml`, rather than
unrelated per-component hex values. The current palette is a light-theme
baseline. A future dark theme needs its own contrast review, not an inversion of
these colors. Muted colors are for tertiary information; verify contrast before
using them for essential text or small interactive labels.

## Typography, spacing, and shape

The current Linux shell uses DejaVu Sans, a reliably available sans-serif. Other
native clients may use their platform UI font. Prefer weight and spacing over
excessive font sizes or color variation.

| Element | Current baseline |
| --- | --- |
| Workspace title | 30px, bold |
| Detail title | 22px, demi-bold |
| PR card title | 17px, demi-bold |
| Body and controls | 12–13px |
| Metadata | 11–12px |
| Section captions | 10px, bold, modest letter spacing |

Use sentence case for readable content. Uppercase is reserved for short section
captions. Keep titles and explanations wrapping; do not truncate the reason for
attention. Long identifiers may elide, with the complete value available in
details or a tooltip. Render repository-provided text as plain text.

- Work on an 8px spacing rhythm, with 4px adjustments where useful.
- Use 20px card/detail padding and 20–32px workspace margins.
- Use approximately 12px surface corners, 7–8px control corners, and 6px badges.
- Controls are currently 36–38px tall. Keep hit areas usable independently of
  the visible icon size and respect platform scaling.
- Prefer borders and background changes over heavy shadows.
- A short hover color transition (currently 100ms) is enough. Avoid continuous
  decorative animation; any progress motion should have a non-motion alternative.

## Workspace structure

Keep these navigation names and their stable IDs:

| Label | ID | User question |
| --- | --- | --- |
| Tailored to you | `tailored` | What should I do next? |
| Action | `action` | What needs my attention? |
| My PRs | `my-prs` | How is my work progressing? |
| Following | `following` | What is happening around my work? |
| Recent | `recent` | What finished in the last 14 days? |

The sidebar provides stable navigation; the top toolbar provides local search
and refresh. The workspace heading, current-view count, and sort control sit
above the ranked list. Counts are scoped to the dataset or search they describe;
do not invent global totals, sum overlapping views, or show a fabricated account.

The current desktop layout has a 238px sidebar, reduced to 194px in narrower
windows, and an 84px toolbar. Below 1250px window width, selected details replace
the list instead of squeezing two unreadable columns together. The present
minimum window is 860 × 600. These are implementation baselines, not constraints
on other platforms.

### PR cards and details

Each card should communicate, in this order:

1. Repository, PR number, and latest activity age.
2. Title.
3. Personal explanation and evidence-backed reasons.
4. Health, lifecycle, and review friction, clearly distinguished.
5. A matching next action and an affordance to inspect details.

Selection uses a clear outline; it must not be confused with attention-required
state. Details expose the full explanation, health, friction contributors and
limitations, captured activity, and the browser/read/snooze/copy actions. Keep
details independently scrollable. Explain that captured activity may be bounded
and provide access to the full GitHub conversation.

All five views currently reuse the same card component. Priority group headings
and a denser authored-PR table are future refinements. They must preserve the
shared core's ordering and meaning, including when the user chooses Highest
friction sorting.

### Modals and shortcut guide

The shortcut guide is an in-app surface, not a default toolkit dialog. Use the
same white surface, quiet border, 12–14px corners, 20–24px spacing, muted section
captions, and clear title hierarchy as the workspace. Render keys as
compact bordered keycaps with readable labels, group actions by user intent, and
include a clear close action. Keep the guide quiet and scannable; it should teach
the existing interaction model without introducing new shortcuts.

`RadarModal.qml` supplies the shared centered surface, 24px body padding,
22px title, subdued overlay, scrollable body, and persistent action footer.
Shortcuts use grouped keycaps and Done; confirmations use Cancel and a specific
action label, initially focusing Cancel. Escape dismisses without applying an
action. Future preferences should reuse this shell with grouped settings and
explicit Save/Cancel semantics when editing a draft; preferences are not yet
implemented. Keep the header and footer visible when the body needs scrolling.

Rendered references (illustrative data, 860 × 640):
[shortcuts](mockups/high/shortcuts-modal.png) and
[confirmation](mockups/high/confirmation-modal.png).

## Interaction and data states

- Search is local and filters title, repository, or PR number. Distinguish an
  empty view from a search with no matches; offer a clear-search action.
- Prefer showing the last successful local capture while fresh data is fetched.
  A refresh should not blank useful cached content.
- Separate local loading from background syncing. With no usable data, show a
  waiting state; with cached data, keep it usable and disclose refresh failures.
- Show the latest successful capture time separately from ongoing work or errors.
  Do not imply “up to date” merely because a process is idle.
- Clear detail selection when the selected PR leaves the rendered dataset, and
  update its content when the same PR receives new data.
- **Mark read** and **Snooze** affect this device. New meaningful activity may
  reactivate the PR. Do not describe these as GitHub notification mutations.
- Provide immediate, modest feedback for completed local actions, such as
  **Copied ✓**. Do not announce success before an operation succeeds.

The shell renders already-ranked data and sends explicit commands. View changes,
visual grouping, and presentation helpers must not introduce new GitHub queries,
classification rules, ranking policies, or persistence in QML.

## Content and language

Use short, specific, nonjudgmental language. Address the user directly when the
relationship is known. Explain evidence rather than assigning blame or guessing
intent. Product explanations remain owned by the shared domain layer.

| Prefer | Avoid |
| --- | --- |
| “Your review is outstanding.” | “You are blocking this PR.” |
| “Changes requested on your PR.” | “Your PR is bad.” |
| “Your PR is awaiting review.” | “Nothing is happening.” |
| “Limited history” | “Low friction” when evidence is missing |
| “Showing cached data. Could not refresh GitHub.” | “Up to date” after a failed refresh |
| “No matching pull requests” | A blank list with no explanation |

Use **Why this needs your attention** for outstanding actions and **Why this is
here** for waiting or informational items. Preserve **Review friction** as the
label for accumulated difficulty; never use it as a synonym for failing checks.

Action labels describe their destination: **Start review**, **View feedback**,
**View discussion**, or **Open PR**. Use **Mark read**, **Snooze**, **Copy link**,
and **Refresh** consistently. External-link arrows supplement a meaningful label.
Use **GitHub**, **PR**, and **pull request** with consistent capitalization.

Relative ages are appropriate in lists; details should provide an explicit date
and time. Loading, empty, stale, and error copy should explain the state and a
useful next step without making the interface noisy.

## Accessibility and keyboard behavior

- Keep native focus and keyboard activation. Every interactive element needs a
  visible focus indicator and an accessible name, including icon-only controls.
- Preserve **Ctrl+K** for search, **Ctrl+R** for refresh, and **Escape** to close
  details, then clear search. Adapt modifiers to platform conventions elsewhere.
- Holding **Ctrl** reveals subtle `1`–`5` workspace hints at the left edge of
  navigation items; `Ctrl+1` through `Ctrl+5` select the corresponding workspace.
- Do not rely on color, hover, or a tooltip alone for essential meaning.
- Support long titles, long repository names, larger text, display scaling, and
  scrolling without overlapping actions or losing the primary explanation.
- Distinguish selected navigation, selected PR, hover, and keyboard focus.

## Extending the system

Reuse `RadarButton.qml`, `Badge.qml`, `PrCard.qml`, `DetailPanel.qml`, and
`Style.qml` before creating new variants. Promote recurring treatments into
shared components or tokens. Keep platform-specific behavior in native shells.

For significant design changes:

1. State the user question and data contract the change serves.
2. Update the relevant reference generator and images in `docs/mockups/high/`.
3. Check populated, empty, loading, stale/error, and narrow-window states.
4. Verify keyboard access, long content, and selection during model replacement.
5. Run the Qt build and relevant workspace tests; capture actual rendered
   screenshots when changing layout. Use illustrative or sanitized data.
6. Update this document when changing shared tokens, terminology, or interaction
   conventions. Label proposed behavior separately from implemented behavior.

The visual system should evolve through these shared decisions, rather than
accumulating independent styles for individual screens.
