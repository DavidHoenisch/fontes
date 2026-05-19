"""SQLite helpers for content database construction."""

from __future__ import annotations

import sqlite3
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
CONTENT_SCHEMA = REPO_ROOT / "schema" / "content.sql"
USER_SCHEMA = REPO_ROOT / "schema" / "user.sql"


def connect(path: Path) -> sqlite3.Connection:
    path.parent.mkdir(parents=True, exist_ok=True)
    conn = sqlite3.connect(path)
    conn.execute("PRAGMA foreign_keys = ON")
    return conn


def init_content_db(conn: sqlite3.Connection) -> None:
    conn.executescript(CONTENT_SCHEMA.read_text(encoding="utf-8"))


def init_user_db(conn: sqlite3.Connection) -> None:
    conn.executescript(USER_SCHEMA.read_text(encoding="utf-8"))
