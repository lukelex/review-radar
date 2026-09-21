#!/usr/bin/env python3
"""High-fidelity design references. Run with Python 3 and rsvg-convert."""
from html import escape
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent
INK, MUTED, ACCENT, LINE = '#20283f', '#788197', '#635bdf', '#e4e7ef'
parts = []

def box(x, y, w, h, fill='white', stroke='none', r=10):
    parts.append(f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{fill}" stroke="{stroke}"/>')

def text(x, y, value, size=13, color=INK, weight=400):
    parts.append(f'<text x="{x}" y="{y}" font-size="{size}" font-weight="{weight}" fill="{color}">{escape(value)}</text>')

def pill(x, y, value, color=ACCENT, bg='#efedfc'):
    w = len(value) * 7 + 24
    box(x, y, w, 26, bg, r=6)
    text(x+12, y+18, value, 11, color, 600)
    return w

def button(x, y, value, primary=False, w=130):
    box(x, y, w, 34, ACCENT if primary else 'white', 'none' if primary else LINE, 7)
    text(x+14, y+22, value, 12, 'white' if primary else INK, 600)

VIEWS = [('Tailored to you', 'Your next move, in focus.'),
         ('Action', 'The work that needs you.'),
         ('My PRs', 'Your work, from draft to done.'),
         ('Following', 'Stay close to the conversation.'),
         ('Recent', 'A little perspective on what shipped.')]
DATA = [
    ('example / api', '142', 'Add pagination to activity endpoint', 'Review requested', 'Your review is outstanding.', 'user-002 requested your review.', 'Checks passing · Review requested', 'Low', 'First round · 1 day in review', 'Start review', '12m ago'),
    ('example / web', '87', 'Improve keyboard navigation', 'Changes requested', 'Feedback is ready for you.', 'user-003 requested changes on your PR.', '2 checks failing · Changes requested', 'High', '4 review rounds · 12 days in review', 'View feedback', '18m ago'),
    ('example / core', '61', 'Handle expired sessions', 'New activity', 'The conversation moved forward.', 'user-004 left a new comment on your PR.', 'Checks passing · Conflict to resolve', 'Moderate', '2 review rounds · 5 days in review', 'View discussion', '42m ago'),
    ('example / api', '139', 'Document response fields', 'Awaiting review', 'Over to your reviewers.', 'Your PR is awaiting review. No action needed right now.', 'Checks passing · Mergeable', 'High', '3 review rounds · Waiting 6 days', 'Open PR', '2h ago'),
    ('example / web', '82', 'Unify the component theme tokens', 'Following', 'You are involved in this PR.', 'Keep up with the latest review and discussion.', 'Checks passing · Review in progress', 'Low', 'First round · Little rework', 'Open PR', '3h ago'),
    ('example / core', '58', 'Reduce session lookup latency', 'Merged', 'This work has shipped.', 'Merged into main. Discussion remains available.', 'Merged · Completed yesterday', 'Low', '1 review round · 2 days in review', 'Open PR', 'Yesterday'),
    ('example / api', '131', 'Explore cursor-based exports', 'Closed', 'This pull request was closed.', 'Closed without merging. Discussion remains available.', 'Closed · Completed 2 days ago', 'Not assessed', 'Limited review history', 'Open PR', '2d ago'),
]

def shell(view, detail=False):
    parts.clear()
    parts.append('<svg xmlns="http://www.w3.org/2000/svg" width="1440" height="1000" viewBox="0 0 1440 1000"><g font-family="Inter, DejaVu Sans, sans-serif">')
    box(0, 0, 1440, 1000, '#f6f7fb', r=0)
    box(0, 0, 238, 1000, '#f0f2f8', r=0)
    logo = (ROOT.parent.parent / 'assets/logo.svg').read_text()
    parts.append(logo.replace('width="256" height="256"', 'x="24" y="26" width="34" height="34"'))
    text(70, 49, 'Review Radar', 17, INK, 700)
    text(27, 118, 'WORKSPACE', 10, MUTED, 700)
    for i, (name, _) in enumerate(VIEWS):
        y = 140 + i*48
        if i == view:
            box(14, y, 210, 42, '#e5e3fa', r=8)
            box(14, y+10, 3, 22, ACCENT, r=2)
        text(29, y+27, ['◎', '⚑', '◇', '◉', '◷'][i], 17, ACCENT if i == view else MUTED)
        text(60, y+26, name, 13, ACCENT if i == view else '#596279', 600 if i == view else 400)
    box(24, 427, 190, 1, LINE, r=0)
    text(28, 460, 'LOCAL TO THIS DEVICE', 10, MUTED, 600)
    text(28, 485, 'Read and snoozed items stay quiet', 10, MUTED)
    text(28, 502, 'until something meaningful changes.', 10, MUTED)
    text(28, 943, '●  github.com', 12, '#596279', 600)
    text(28, 968, 'A quieter place for pull requests.', 10, MUTED)
    box(238, 0, 1202, 84, 'white', r=0)
    box(270, 23, 505, 38, '#f6f7fb', LINE, 8)
    text(286, 48, '⌕', 20, MUTED)
    text(315, 47, 'Search title, repository, or PR number', 12, MUTED)
    pill(718, 29, 'Ctrl K', MUTED, '#e9ecf3')
    text(1044, 47, '●  Up to date', 12, '#44806b')
    button(1180, 25, '↻  Refresh', w=112)
    box(1320, 26, 32, 32, '#e8e5fa', r=16)
    text(1328, 47, 'RR', 11, ACCENT, 700)
    text(276, 130, 'WORKSPACE  /  '+VIEWS[view][0].upper(), 10, MUTED, 600)
    text(276, 174, VIEWS[view][0], 30, INK, 700)
    text(277, 203, VIEWS[view][1], 14, MUTED)
    pill(1270, 149, '14 days' if view == 4 else '4 pull requests', MUTED, '#e9ecf3')
    text(278, 249, 'PULL REQUESTS', 10, MUTED, 700)
    button(1140, 225, 'Priority + newest  ▾', w=254)
    text(277, 974, '●  Cached on this device   ·   Updated 2 minutes ago', 11, MUTED)
    text(1158, 974, 'ILLUSTRATIVE DESIGN DATA', 9, MUTED)

def card(y, data, w=1116, selected=False):
    repo, num, title, state, heading, why, health, friction, detail, action, age = data
    box(276, y+2, w, 199, '#ebedf4', r=12)
    box(276, y, w, 199, 'white', ACCENT if selected else LINE, 12)
    text(298, y+28, repo + '   #' + num, 12, MUTED, 500)
    text(276+w-88, y+28, age, 11, MUTED)
    text(298, y+58, title, 17, INK, 600)
    box(298, y+76, 3, 46, '#aba5f3', r=2)
    text(312, y+93, heading, 12, ACCENT, 600)
    text(312, y+113, why, 12, '#596279')
    text(298, y+147, health, 11, '#596279')
    text(298, y+179, 'Review friction', 10, MUTED)
    pill(394, y+162, friction, '#a35b2a' if friction == 'High' else '#596279', '#fff0e4' if friction == 'High' else '#f0f2f8')
    if w > 750:
        text(499, y+179, detail, 11, MUTED)
        button(276+w-164, y+151, action+'  ↗', True, 142)

for view, stem, indices in [(0, '01-tailored', [0,1,3]), (1,'02-action',[0,1,2]),
                            (2,'03-my-prs',[1,2,3]), (3,'04-following',[0,4]), (4,'05-recent',[5,6])]:
    shell(view)
    for n, i in enumerate(indices):
        card(279+n*219, DATA[i])
    parts.append('</g></svg>')
    (ROOT / (stem+'.svg')).write_text('\n'.join(parts))

shell(0, True)
card(279, DATA[0], 524, True)
card(498, DATA[1], 524)
card(717, DATA[3], 524)
box(822, 279, 570, 656, 'white', LINE, 12)
text(846, 311, 'PULL REQUEST DETAILS', 10, MUTED, 700)
text(1358, 311, '×', 20, MUTED)
text(846, 350, 'example / api   #142', 12, MUTED)
text(846, 386, 'Add pagination to activity endpoint', 21, INK, 600)
pill(846, 405, 'Review requested')
box(846, 452, 522, 91, '#f1effd', r=9)
text(863, 477, 'WHY THIS NEEDS YOUR ATTENTION', 10, ACCENT, 700)
text(863, 505, 'Your review is outstanding.', 15, INK, 600)
text(863, 526, 'user-002 requested your review.', 12, '#596279')
button(846, 563, 'Start review  ↗', True, 150)
button(1006, 563, 'Mark read', w=110)
button(1126, 563, 'Snooze  ▾', w=112)
button(1248, 563, 'Copy link', w=120)
text(846, 635, 'PR HEALTH', 10, MUTED, 700)
pill(846, 650, 'Checks passing', '#32745c', '#e8f5ef')
text(846, 707, 'REVIEW FRICTION', 10, MUTED, 700)
text(846, 733, 'Low   ·   First round   ·   1 day in review', 13, INK)
text(846, 780, 'RECENT ACTIVITY', 10, MUTED, 700)
for n, (title, sub) in enumerate([('Review requested', 'user-002 · 12 minutes ago'), ('Checks passed', 'All required checks · 28 minutes ago'), ('Pull request opened', 'user-002 · Yesterday')]):
    y=814+n*47
    box(847,y-9,8,8,ACCENT,r=4)
    text(869,y,title,12,INK,600)
    text(869,y+18,sub,10,MUTED)
parts.append('</g></svg>')
(ROOT/'06-detail.svg').write_text('\n'.join(parts))
for svg in sorted(ROOT.glob('*.svg')):
    subprocess.run(['rsvg-convert', str(svg), '-o', str(svg.with_suffix('.png'))], check=True)
