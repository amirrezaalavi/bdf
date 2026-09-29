#!/usr/bin/env python3
"""pdfrtl knowledge DB — SQLite + FTS5. No services, no server, works offline.

Why a flat SQLite file and not a vector store / RAG stack: this is a small, curated
body of knowledge (findings, pitfalls, decisions, task state) that must be greppable
and reviewable by a human, on any machine, with zero services. Full-text search is
enough; embeddings would add a runtime and a dependency for no gain.

Usage (works with `python` or `python3` — the script does not care):
    python3 scripts/db.py init
    python3 scripts/db.py note --kind pitfall --title "..." --body "..." [--source URL]
    python3 scripts/db.py search "lam-alef"
    python3 scripts/db.py task-add --kind port --title "normalizer" --phase P1 [--body ...] [--owner W1]
    python3 scripts/db.py task-list [--phase P1] [--status open]
    python3 scripts/db.py task-done 3 --ref <commit-sha-or-report-path>
    python3 scripts/db.py status
"""
from __future__ import annotations

import argparse
import datetime as dt
import pathlib
import sqlite3
import sys

DB_PATH = pathlib.Path(__file__).resolve().parent.parent / "db" / "pdfrtl.db"

SCHEMA = """
CREATE TABLE IF NOT EXISTS tasks(
  id         INTEGER PRIMARY KEY,
  kind       TEXT NOT NULL,
  title      TEXT NOT NULL,
  body       TEXT,
  status     TEXT NOT NULL DEFAULT 'open',
  phase      TEXT,
  owner      TEXT,
  ref        TEXT,
  created_at TEXT NOT NULL,
  closed_at  TEXT
);
CREATE TABLE IF NOT EXISTS knowledge(
  id         INTEGER PRIMARY KEY,
  kind       TEXT NOT NULL,
  title      TEXT NOT NULL,
  body       TEXT NOT NULL,
  source     TEXT,
  created_at TEXT NOT NULL
);
CREATE VIRTUAL TABLE IF NOT EXISTS knowledge_fts
  USING fts5(title, body, content='knowledge', content_rowid='id');
CREATE TRIGGER IF NOT EXISTS knowledge_ai AFTER INSERT ON knowledge BEGIN
  INSERT INTO knowledge_fts(rowid, title, body) VALUES (new.id, new.title, new.body);
END;
CREATE TRIGGER IF NOT EXISTS knowledge_ad AFTER DELETE ON knowledge BEGIN
  INSERT INTO knowledge_fts(knowledge_fts, rowid, title, body)
    VALUES('delete', old.id, old.title, old.body);
END;
CREATE TRIGGER IF NOT EXISTS knowledge_au AFTER UPDATE ON knowledge BEGIN
  INSERT INTO knowledge_fts(knowledge_fts, rowid, title, body)
    VALUES('delete', old.id, old.title, old.body);
  INSERT INTO knowledge_fts(rowid, title, body) VALUES (new.id, new.title, new.body);
END;
"""

VALID_KINDS = {"finding", "lesson", "pitfall", "decision", "spec"}


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def connect() -> sqlite3.Connection:
    DB_PATH.parent.mkdir(parents=True, exist_ok=True)
    con = sqlite3.connect(DB_PATH)
    con.row_factory = sqlite3.Row
    return con


def cmd_init(_args: argparse.Namespace) -> int:
    with connect() as con:
        con.executescript(SCHEMA)
    print(f"initialised {DB_PATH}")
    return 0


def cmd_note(args: argparse.Namespace) -> int:
    if args.kind not in VALID_KINDS:
        print(f"kind must be one of {sorted(VALID_KINDS)}", file=sys.stderr)
        return 2
    with connect() as con:
        con.execute(
            "INSERT INTO knowledge(kind, title, body, source, created_at) VALUES(?,?,?,?,?)",
            (args.kind, args.title, args.body, args.source, now()),
        )
    print(f"noted [{args.kind}] {args.title}")
    return 0


def cmd_search(args: argparse.Namespace) -> int:
    with connect() as con:
        rows = con.execute(
            """
            SELECT k.id, k.kind, k.title, k.body, k.source
            FROM knowledge_fts f JOIN knowledge k ON k.id = f.rowid
            WHERE knowledge_fts MATCH ?
            ORDER BY rank
            LIMIT ?
            """,
            (args.query, args.limit),
        ).fetchall()
    if not rows:
        print("no matches")
        return 1
    for row in rows:
        print(f"#{row['id']} [{row['kind']}] {row['title']}")
        print(f"    {row['body']}")
        if row["source"]:
            print(f"    source: {row['source']}")
    return 0


def cmd_task_add(args: argparse.Namespace) -> int:
    with connect() as con:
        cur = con.execute(
            """INSERT INTO tasks(kind, title, body, phase, owner, created_at)
               VALUES(?,?,?,?,?,?)""",
            (args.kind, args.title, args.body, args.phase, args.owner, now()),
        )
        task_id = cur.lastrowid
    print(f"task {task_id} added: {args.title}")
    return 0


def cmd_task_list(args: argparse.Namespace) -> int:
    query = "SELECT * FROM tasks WHERE 1=1"
    params: list[str] = []
    if args.phase:
        query += " AND phase = ?"
        params.append(args.phase)
    if args.status:
        query += " AND status = ?"
        params.append(args.status)
    query += " ORDER BY id"
    with connect() as con:
        rows = con.execute(query, params).fetchall()
    for row in rows:
        print(f"[{row['id']}] {row['status']:>6} {row['phase'] or '-':>4} {row['title']}")
    print(f"-- {len(rows)} task(s)")
    return 0


def cmd_task_done(args: argparse.Namespace) -> int:
    if not args.ref:
        print("--ref is required: a task closes with evidence (commit sha or report path)",
              file=sys.stderr)
        return 2
    with connect() as con:
        cur = con.execute(
            "UPDATE tasks SET status='done', ref=?, closed_at=? WHERE id=? AND status!='done'",
            (args.ref, now(), args.id),
        )
    if cur.rowcount == 0:
        print(f"task {args.id} not found or already done", file=sys.stderr)
        return 1
    print(f"task {args.id} done -> {args.ref}")
    return 0


def cmd_status(_args: argparse.Namespace) -> int:
    with connect() as con:
        tasks = con.execute(
            "SELECT status, COUNT(*) c FROM tasks GROUP BY status ORDER BY status"
        ).fetchall()
        kinds = con.execute(
            "SELECT kind, COUNT(*) c FROM knowledge GROUP BY kind ORDER BY kind"
        ).fetchall()
    print(f"db: {DB_PATH}")
    print("tasks: " + (", ".join(f"{r['status']}={r['c']}" for r in tasks) or "none"))
    print("knowledge: " + (", ".join(f"{r['kind']}={r['c']}" for r in kinds) or "none"))
    return 0


def cmd_export(_args: argparse.Namespace) -> int:
    """Write docs/KNOWLEDGE.md — the durable, git-committed view of this database.

    Why: the SQLite file is a working index (binary, conflict-prone if two agents write
    it), while the knowledge itself must travel with the repo and be readable in a diff.
    So: query here, commit the markdown. Run `export` before every commit that adds notes.
    """
    out = DB_PATH.parent.parent / "docs" / "KNOWLEDGE.md"
    with connect() as con:
        knowledge = con.execute(
            "SELECT kind, title, body, source, created_at FROM knowledge ORDER BY kind, id"
        ).fetchall()
        tasks = con.execute(
            "SELECT id, status, phase, owner, title, ref FROM tasks ORDER BY status, id"
        ).fetchall()

    kinds: dict[str, list[sqlite3.Row]] = {}
    for row in knowledge:
        kinds.setdefault(row["kind"], []).append(row)

    lines = [
        "# KNOWLEDGE.md — generated from db/pdfrtl.db",
        "",
        "**Generated by `scripts/db.py export`. Do not hand-edit** — add a note instead:",
        "`python3 scripts/db.py note --kind pitfall --title \"…\" --body \"…\"`",
        "then `python3 scripts/db.py export` and commit both this file and the change.",
        "",
        "This is how the *why* survives: the SQLite file is the working index, this file is",
        "the record that travels with the repository.",
        "",
    ]
    for kind in sorted(kinds):
        lines += [f"## {kind}", ""]
        for row in kinds[kind]:
            lines.append(f"### {row['title']}")
            lines.append("")
            lines.append(row["body"])
            if row["source"]:
                lines.append("")
                lines.append(f"*source:* {row['source']}")
            lines.append("")
            lines.append(f"*recorded:* {row['created_at']}")
            lines.append("")

    lines += ["## tasks", ""]
    if not tasks:
        lines.append("_(none)_")
    else:
        lines += ["| id | status | phase | owner | title | evidence |", "|---|---|---|---|---|---|"]
        for row in tasks:
            lines.append(
                f"| {row['id']} | {row['status']} | {row['phase'] or '-'} | {row['owner'] or '-'} "
                f"| {row['title']} | {row['ref'] or '-'} |"
            )
    lines.append("")

    out.write_text("\n".join(lines), encoding="utf-8")
    print(f"exported {len(knowledge)} note(s), {len(tasks)} task(s) -> {out}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="pdfrtl knowledge DB")
    sub = parser.add_subparsers(dest="cmd", required=True)

    sub.add_parser("init").set_defaults(func=cmd_init)

    p = sub.add_parser("note")
    p.add_argument("--kind", required=True)
    p.add_argument("--title", required=True)
    p.add_argument("--body", required=True)
    p.add_argument("--source", default=None)
    p.set_defaults(func=cmd_note)

    p = sub.add_parser("search")
    p.add_argument("query")
    p.add_argument("--limit", type=int, default=10)
    p.set_defaults(func=cmd_search)

    p = sub.add_parser("task-add")
    p.add_argument("--kind", required=True)
    p.add_argument("--title", required=True)
    p.add_argument("--body", default=None)
    p.add_argument("--phase", default=None)
    p.add_argument("--owner", default=None)
    p.set_defaults(func=cmd_task_add)

    p = sub.add_parser("task-list")
    p.add_argument("--phase", default=None)
    p.add_argument("--status", default="open")
    p.set_defaults(func=cmd_task_list)

    p = sub.add_parser("task-done")
    p.add_argument("id", type=int)
    p.add_argument("--ref", default=None)
    p.set_defaults(func=cmd_task_done)

    sub.add_parser("status").set_defaults(func=cmd_status)

    sub.add_parser("export").set_defaults(func=cmd_export)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
