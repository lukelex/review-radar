#!/usr/bin/env python3
"""Rebuild the illustrative SVG/PNG mockups; requires rsvg-convert."""

from html import escape
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent
parts = []


def text(x, y, value, size=12, weight=400, color="#333"):
    parts.append(f'<text x="{x}" y="{y}" font-size="{size}" font-weight="{weight}" fill="{color}">{escape(value)}</text>')


def box(x, y, w, h, fill="#fff", stroke="#ccc"):
    parts.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="4" fill="{fill}" stroke="{stroke}"/>')


def button(x, y, label, w=136):
    box(x, y, w, 30, stroke="#888")
    text(x + 12, y + 20, label, 12, 600)


def start(index, title, subtitle, selected, sort="Priority + newest"):
    parts.clear()
    parts.append('<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="1120" viewBox="0 0 1200 1120">')
    parts.append(f'<title>{escape(title)} — Review Radar wireframe</title><g font-family="DejaVu Sans, sans-serif">')
    box(0, 0, 1200, 1120, "#fafafa", "#fafafa")
    text(26, 32, "REVIEW RADAR / WIREFRAMES", 13, 700)
    text(875, 32, f"{index:02} / 06    STATIC CONCEPT", color="#777")
    box(24, 52, 1152, 987, stroke="#999")
    box(25, 53, 1150, 62, "#f5f5f5")
    text(46, 91, "Review Radar", 20, 700)
    box(278, 68, 424, 31)
    text(290, 89, "Search title, repository, or PR number", color="#777")
    text(807, 89, "Updated 2 min ago", color="#666")
    button(970, 68, "Refresh", 87)
    text(1080, 89, "user-001")
    box(25, 115, 226, 923, "#f7f7f7")
    text(44, 151, "WORKSPACE", 11, 700, "#777")
    for i, (name, count) in enumerate(zip(["Tailored to you", "Action", "My PRs", "Following", "Recent"], [12, 5, 6, 4, 7])):
        y = 194 + 49 * i
        if name == selected:
            box(37, y - 26, 201, 40, "#dedede", "#aaa")
        text(49, y, name, 14, 700 if name == selected else 400)
        text(210, y, str(count))
    text(46, 471, "Snoozed   2", 13)
    text(46, 494, "Return to queue when due", 10, color="#777")
    text(46, 950, "github.com", 12, 600)
    text(46, 973, "One account / local state", 10, color="#777")
    text(278, 154, title, 24, 700)
    text(278, 181, subtitle, color="#666")
    if sort:
        button(278, 200, f"Sort: {sort} ▾", 250)
        alternate = "Default view order" if sort == "Highest friction" else "Highest friction"
        text(548, 220, f"Also available: {alternate}", color="#666")


def finish(stem, note):
    text(27, 1070, note, 12, color="#555")
    text(27, 1093, "Illustrative data. Friction levels and history are design targets, not computed metrics. Labels work without color.", 10, color="#777")
    parts.append('</g></svg>')
    svg = ROOT / f"{stem}.svg"
    svg.write_text("\n".join(parts) + "\n")
    subprocess.run(["rsvg-convert", str(svg), "-o", str(ROOT / f"{stem}.png")], check=True)


REVIEW = dict(repo="example/api #142 · OPEN", title="Add pagination to activity endpoint",
              why="user-002 requested your review. Your review is outstanding.",
              health="Review requested · Checks passing", age="Requested 12m ago", action="Start review",
              friction="Low · First review round · 1 day in review · Little rework")
CHANGES = dict(repo="example/web #87 · OPEN · Your PR", title="Improve keyboard navigation",
               why="user-003 requested changes on your PR. Review and address the feedback.",
               health="Changes requested · 2 checks failing · Mergeability unknown", age="Review received 18m ago", action="View feedback",
               friction="High · 4 review rounds · 12 days in review · Substantial rework")
COMMENT = dict(repo="example/core #61 · OPEN · Your PR", title="Handle expired sessions",
               why="user-004 left a new comment on your PR. Read and respond as needed.",
               health="New comment · Conflict to resolve · Checks passing", age="Comment added 42m ago", action="View discussion",
               friction="Moderate · 2 review rounds · 5 days in review · Some rework")
WAITING = dict(repo="example/api #139 · OPEN · Your PR", title="Document response fields",
               why="Your PR is awaiting review. No action needed from you right now.",
               health="Waiting · Checks passing · Mergeable", age="Latest activity 2h ago", action="Open PR",
               friction="High · 3 review rounds · 16 days in review · Waiting 6 days", attention=False)


def card(y, data, historical=False):
    attention = data.get("attention", True)
    box(278, y, 870, 172, "#fff" if attention else "#fafafa")
    if attention:
        box(278, y, 4, 172, "#555", "#555")
    text(294, y + 21, data["title"], 16, 600)
    text(294, y + 41, data["repo"], 11, color="#666")
    text(294, y + 65, "WHY THIS NEEDS YOUR ATTENTION" if attention else "WHY THIS IS HERE", 10, 700)
    text(294, y + 85, data["why"], 13, 600)
    text(294, y + 109, data["health"], 12, color="#555")
    prefix = "Historical review friction" if historical else "Review friction"
    text(294, y + 132, f'{prefix}: {data["friction"]}', 11, 600)
    text(294, y + 155, data["age"], 11, color="#666")
    button(996, y + 138, data["action"])


start(1, "Tailored to you", "Every PR earns its place: your next action, plus the friction accumulated along the way.", "Tailored to you")
text(278, 252, "1 / YOUR REVIEW REQUESTED · 2 PRs", 11, 700)
card(262, REVIEW)
text(278, 455, "2 / YOUR PRS NEEDING ACTION · 3 PRs", 11, 700)
card(465, CHANGES)
card(647, COMMENT)
text(278, 840, "3 / YOUR PRS AWAITING REVIEW · 3 PRs", 11, 700)
card(850, WAITING)
finish("01-tailored", "Default: priority bands, then newest activity. Following and Recent continue below. High friction can coexist with waiting.")

start(2, "Action", "5 PRs need you. Friction explains accumulated difficulty; the reason explains your next step.", "Action")
text(278, 252, "REVIEW OBLIGATIONS · 2 PRs", 11, 700)
card(262, REVIEW)
text(278, 455, "YOUR PRS NEEDING ACTION · 3 PRs", 11, 700)
card(465, CHANGES)
card(647, COMMENT)
box(278, 840, 870, 159, "#f5f5f5")
text(294, 866, "Selected PR: Handle expired sessions", 14, 700)
button(294, 883, "Mark read")
button(442, 883, "Snooze…")
button(590, 883, "Copy link")
text(294, 942, "Read and snooze are local. New meaningful events reactivate the PR.")
text(294, 967, "Friction alone does not create an obligation or trigger a notification.", color="#666")
finish("02-action", "Reasons and all health issues stay visible together. Snoozed items remain discoverable in the sidebar.")

start(3, "My PRs", "All your open PRs. Highest friction surfaces work that may need help getting unstuck.", "My PRs", "Highest friction")
text(278, 252, "6 PRs · Known levels first; limited history last · This sort crosses the usual groups", 11, color="#666")
box(278, 267, 870, 32, "#ededed")
text(294, 288, "PULL REQUEST / WHY THIS IS HERE", 10, 700)
text(800, 288, "HEALTH / REVIEW FRICTION", 10, 700)
rows = [
    ("#139 Document response fields", "example/api · Open · Latest activity 2h ago", "Waiting for review; no action needed from you now.", "Awaiting review · Checks passing", "High · 3 rounds · 16 days", "Waiting 6 days"),
    ("#87 Improve keyboard navigation", "example/web · Open · Latest activity 18m ago", "user-003 requested changes. Address the feedback.", "Changes requested · 2 checks failing", "High · 4 rounds · 12 days", "Substantial rework"),
    ("#61 Handle expired sessions", "example/core · Open · Latest activity 42m ago", "New comment to read; a conflict also needs resolution.", "New comment · Conflict", "Moderate · 2 rounds · 5 days", "Some rework"),
    ("#52 Simplify request handling", "example/api · Open · Latest activity 1h ago", "Approved and checks pass. Confirm merge requirements.", "Approved · Passing · Ready*", "Low · 1 round · 2 days", "Little rework"),
    ("#120 Add cache metrics", "example/core · Open · Latest activity 5h ago", "Waiting for review and checks; monitor progress.", "Awaiting review · Checks pending", "Limited history", "Review start and churn unknown"),
    ("#94 Explore search shortcuts", "example/web · Draft · Latest activity 1d ago", "Your draft is here to track work before review.", "Draft · No checks · Merge unknown", "Not assessed", "Review has not started"),
]
for i, row in enumerate(rows):
    y = 299 + i * 109
    box(278, y, 870, 109, "#fff" if i % 2 == 0 else "#fafafa")
    text(294, y + 24, row[0], 14, 600)
    text(294, y + 45, row[1], 11, color="#666")
    text(294, y + 75, row[2], 12, 600)
    text(800, y + 24, row[3], 11)
    text(800, y + 51, row[4], 12, 700)
    text(800, y + 75, row[5], 11, color="#666")
text(294, 981, "* Readiness is provisional. Open the selected PR to confirm GitHub requirements.", 11, color="#666")
text(294, 1005, "High ties use longest review time, then newest activity. Select a row for evidence and actions.", 11, color="#666")
finish("03-my-prs", "Friction is independent of urgency: a waiting PR can rank above a PR needing action in this explicit alternate sort.")

start(4, "Following", "PRs you are involved in, excluding your own. Relevance does not always imply an obligation.", "Following", "Newest activity")
following = [
    dict(repo="example/core #72 · OPEN", title="Add request tracing", why="A PR you are involved in has new activity. Catch up when useful.", health="For awareness · Checks passing", age="Latest activity 8m ago", action="View activity", friction="Moderate · 2 review rounds · 6 days in review · Some rework", attention=False),
    REVIEW,
    dict(repo="example/web #93 · DRAFT", title="Explore compact navigation", why="You are involved in this draft. New activity is available to read.", health="For awareness · Draft · Checks pending", age="Latest activity 1h ago", action="View activity", friction="Not assessed · Review has not started", attention=False),
    dict(repo="example/core #68 · OPEN", title="Improve retry behavior", why="You are involved in this PR. No new activity since yesterday.", health="For awareness · Checks unknown", age="Latest activity 1d ago", action="Open PR", friction="Limited history · Review rounds and churn unknown", attention=False),
]
for i, data in enumerate(following):
    card(250 + 184 * i, data)
text(294, 1006, "Review obligations also appear in Action. View counts overlap.", 11, color="#666")
finish("04-following", "Participation explains inclusion. Use more specific relationship claims only when supported by collected evidence.")

start(5, "Recent", "Closed or merged in the last 14 days. Friction is historical context, not an outstanding action.", "Recent", "Newest activity")
text(278, 253, "ALL 7   /   MERGED 5   /   CLOSED 2", 11, 700)
recent = [
    dict(repo="example/api #130 · MERGED", title="Return structured validation errors", why="Your PR was merged today. This completes your work.", health="For awareness · Merged", age="Merged 2h ago", action="View PR", friction="High · 4 review rounds · 14 days in review · Substantial rework", attention=False),
    dict(repo="example/core #64 · CLOSED", title="Prototype alternate cache storage", why="A PR you were involved in closed yesterday without merging.", health="For awareness · Closed without merge", age="Closed 1d ago", action="View PR", friction="Limited history · Review rounds and churn unknown", attention=False),
    dict(repo="example/web #82 · MERGED", title="Improve focus outlines", why="A PR you were involved in was merged 2 days ago.", health="For awareness · Merged", age="Merged 2d ago", action="View PR", friction="Low · 1 review round · 2 days in review · Little rework", attention=False),
]
for i, data in enumerate(recent):
    card(267 + 184 * i, data, historical=True)
box(278, 840, 870, 143, "#f5f5f5")
text(294, 870, "Completion ends the outstanding action, not the story of the PR.", 14, 600)
text(294, 901, "Review time and friction stop accumulating at completion.")
text(294, 926, "Open detail to inspect the review journey and its evidence.")
text(294, 951, "Old review requests and changes requested are history, not current actions.", color="#666")
finish("05-recent", "Merged and closed remain distinct. Historical friction does not reactivate a completed PR.")

start(6, "Action / PR detail", "Understand the next step and the accumulated struggle, with evidence for both.", "Action", sort=None)
box(278, 207, 270, 813, "#fafafa")
text(294, 234, "QUEUE / 5 · PRIORITY + NEWEST", 11, 700)
for i, (title, why, friction) in enumerate([
    ("#142 Add pagination", "Your review is outstanding.", "Low friction · 1 round · 1 day"),
    ("#87 Keyboard navigation", "Changes requested; checks fail.", "High friction · 4 rounds · 12 days"),
    ("#61 Expired sessions", "New comment; conflict to resolve.", "Moderate friction · 2 rounds · 5 days"),
]):
    y = 253 + 140 * i
    box(290, y, 246, 124, "#dedede" if i == 1 else "#fff")
    text(302, y + 25, title, 13, 700)
    text(302, y + 52, "WHY YOU: " + why, 10)
    text(302, y + 80, friction, 10, 600)
    text(302, y + 104, "Select to inspect reasons and evidence", 10, color="#666")
box(564, 207, 584, 813)
text(582, 234, CHANGES["repo"], 11, color="#666")
text(582, 263, CHANGES["title"], 20, 700)
box(582, 283, 548, 108, "#f5f5f5")
text(596, 306, "WHY THIS NEEDS YOUR ATTENTION", 11, 700)
text(596, 330, "user-003 requested changes on your PR 18 minutes ago.", 13, 600)
text(596, 354, "Address the feedback. Two failing checks also need inspection.", 12)
text(596, 376, "Changes requested · 2 checks failing · Mergeability unknown", 11, color="#666")
button(582, 405, "View feedback", 128)
button(720, 405, "View checks", 120)
button(850, 405, "Open PR", 110)
button(582, 445, "Mark read", 128)
button(720, 445, "Snooze…", 120)
button(850, 445, "Copy link", 110)
box(582, 493, 548, 207, "#f5f5f5")
text(596, 518, "HIGH REVIEW FRICTION", 13, 700)
text(596, 542, "Why: repeated review cycles, prolonged review, substantial rework.", 11)
text(596, 569, "4 review rounds", 12, 700)
text(780, 569, "Feedback → revision → re-review", 11)
text(596, 594, "12 days in review", 12, 700)
text(780, 594, "Draft time excluded · 7 days waiting", 11)
text(596, 619, "Substantial rework", 12, 700)
text(780, 619, "Changes during review, not initial size", 11)
text(596, 649, "Evidence: review and revision history over the review period.", 11)
text(596, 675, "Illustrative assessment · Limited history must be labelled when present.", 10, color="#666")
text(582, 730, "ACTIVITY / NEWEST FIRST", 11, 700)
for y, age, title, detail in [
    (757, "18 min ago · user-003", "Changes requested · Round 4", "Review feedback received. View feedback for the original review."),
    (832, "35 min ago · Continuous integration", "2 checks failed", "Unit tests and keyboard checks. View checks for failure details."),
    (907, "1 hour ago · user-001", "Revision pushed", "Updated code following the previous round of feedback."),
]:
    text(596, y, age, 11, color="#666")
    text(596, y + 21, title, 13, 600)
    text(596, y + 41, detail, 10)
text(582, 987, "Partial timeline shown. Open GitHub for the full discussion.", 11, color="#666")
finish("06-detail", "Attention explains what to do now. Review friction explains accumulated difficulty; inspect its evidence without a numeric score.")
