#!/usr/bin/env python3
"""Check WORK.md (the board) and notes/reference.md against the rules in AGENTS.md.

Board row F0. Run from anywhere: `python tools/check_board.py`. Exits 1 and lists every problem found.
Python 3.8+, no packages.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BOARD = ROOT / "WORK.md"
AGENTS = ROOT / "AGENTS.md"
REFERENCE = ROOT / "notes" / "reference.md"

TIERS = ["P0", "P1", "P2", "P3", "P4", "P5"]
ID_RE = re.compile(r"^[FEGCXL]\d+[a-z0-9]*$")
WHO_ONE = r"[A-Za-z0-9-]+/[a-z0-9-]+ \(\d{4}-\d{2}-\d{2}\)"
WHO_RE = re.compile(rf"^{WHO_ONE}( \+ {WHO_ONE})*$")
DONE_WHO_RE = re.compile(r"^[A-Za-z0-9-]+/[a-z0-9-]+( \+ [A-Za-z0-9-]+/[a-z0-9-]+)*$")
DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
HASH_RE = re.compile(r"^[0-9a-f]{7,40}$")
REF_ID_RE = re.compile(r"\bR(\d+)\b")
CHANGE_RE = re.compile(r"^- \d{4}-\d{2}-\d{2} [A-Za-z0-9-]+(/[a-z0-9-]+)?: ")


def split_row(line):
    """Cells of a markdown table row, without the outer pipes. Pipes inside backticks are kept."""
    cells, cur, in_code = [], [], False
    for ch in line.strip()[1:-1]:
        if ch == "`":
            in_code = not in_code
        if ch == "|" and not in_code:
            cells.append("".join(cur).strip())
            cur = []
        else:
            cur.append(ch)
    cells.append("".join(cur).strip())
    return cells


def sections(text):
    """{heading: [lines]} for every '## ' heading."""
    out, name = {}, None
    for line in text.splitlines():
        if line.startswith("## "):
            name = line[3:].strip()
            out[name] = []
        elif name is not None:
            out[name].append(line)
    return out


def table_rows(lines):
    """Data rows of the first table in these lines (header and separator skipped)."""
    rows = [l for l in lines if l.strip().startswith("|")]
    return [split_row(r) for r in rows[2:]]


def people_from_agents():
    """GitHub names in the Areas table of AGENTS.md section 12."""
    text = AGENTS.read_text(encoding="utf-8")
    sec = text.split("## 12. Areas", 1)
    if len(sec) < 2:
        return set()
    body = sec[1].split("\n## ", 1)[0]
    return {r[0] for r in table_rows(body.splitlines()) if r and r[0]}


def reference_ids(errors):
    if not REFERENCE.exists():
        errors.append(f"{REFERENCE.relative_to(ROOT)} is missing")
        return set()
    seen = set()
    for line in REFERENCE.read_text(encoding="utf-8").splitlines():
        if not line.startswith("| R"):
            continue
        rid = split_row(line)[0]
        if not re.fullmatch(r"R\d+", rid):
            errors.append(f"reference.md: bad fact ID {rid!r}")
        elif rid in seen:
            errors.append(f"reference.md: fact ID {rid} used twice")
        seen.add(rid)
    return seen


def main():
    errors = []
    text = BOARD.read_text(encoding="utf-8")
    secs = sections(text)
    people = people_from_agents()
    areas = {"unowned", "any", "humans"} | people

    tier_names = {}
    for name in secs:
        m = re.match(r"(P\d) ", name)
        if m:
            tier_names[m.group(1)] = name
    for t in TIERS:
        if t not in tier_names:
            errors.append(f"WORK.md: no '## {t} ...' section")
    for needed in ("Done", "Changes to the board"):
        if needed not in secs:
            errors.append(f"WORK.md: no '## {needed}' section")

    all_ids, open_rows, done_ids = {}, [], set()
    for t in TIERS:
        if t not in tier_names:
            continue
        for cells in table_rows(secs[tier_names[t]]):
            if len(cells) != 6:
                errors.append(f"{t}: row {cells[:1]} has {len(cells)} cells, want 6 (ID, Task, Area, Needs, Who, Notes)")
                continue
            rid, task, area, needs, who, _notes = cells
            if not ID_RE.match(rid):
                errors.append(f"{t}: bad ID {rid!r}")
            if rid in all_ids:
                errors.append(f"{t}: ID {rid} also in {all_ids[rid]}")
            all_ids[rid] = t
            if not task:
                errors.append(f"{rid}: empty Task")
            if area not in areas:
                errors.append(f"{rid}: Area {area!r} is not unowned/any/humans or a person in AGENTS.md section 12")
            if who and not WHO_RE.match(who):
                errors.append(f"{rid}: Who {who!r} is not 'person/agent (YYYY-MM-DD)'")
            if who and area == "humans":
                errors.append(f"{rid}: a humans row is claimed by {who!r}; agents must not take humans rows")
            open_rows.append((rid, needs))

    for cells in table_rows(secs.get("Done", [])):
        if len(cells) != 6:
            errors.append(f"Done: row {cells[:1]} has {len(cells)} cells, want 6 (ID, Task, Section, Who, Date, Commit)")
            continue
        rid, _task, section, who, date, commit = cells
        if not ID_RE.match(rid):
            errors.append(f"Done: bad ID {rid!r}")
        if rid in all_ids:
            errors.append(f"Done: {rid} is also open in {all_ids[rid]}")
        if rid in done_ids:
            errors.append(f"Done: {rid} listed twice")
        done_ids.add(rid)
        if section not in TIERS:
            errors.append(f"Done {rid}: Section {section!r} is not P0-P5")
        if not DONE_WHO_RE.match(who):
            errors.append(f"Done {rid}: Who {who!r} is not 'person/agent'")
        if not DATE_RE.match(date):
            errors.append(f"Done {rid}: Date {date!r} is not YYYY-MM-DD")
        if not HASH_RE.match(commit):
            errors.append(f"Done {rid}: Commit {commit!r} is not a commit hash")

    known = set(all_ids) | done_ids
    for rid, needs in open_rows:
        for need in [n.strip() for n in needs.split(",") if n.strip()]:
            if need not in known:
                errors.append(f"{rid}: Needs {need!r}, which is not on the board")
            if need == rid:
                errors.append(f"{rid}: Needs itself")

    for line in secs.get("Changes to the board", []):
        if line.strip() and not CHANGE_RE.match(line):
            errors.append(f"Changes: line not '- YYYY-MM-DD person/agent: ...': {line[:60]!r}")

    facts = reference_ids(errors)
    if facts:
        for n in sorted({int(m) for m in REF_ID_RE.findall(text)}):
            if f"R{n}" not in facts:
                errors.append(f"WORK.md cites R{n}, which is not in notes/reference.md")

    if errors:
        print(f"check_board: {len(errors)} problem(s)")
        for e in errors:
            print(f"  - {e}")
        return 1
    open_n = len(open_rows)
    print(f"check_board: OK ({open_n} open rows, {len(done_ids)} done, {len(facts)} facts)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
