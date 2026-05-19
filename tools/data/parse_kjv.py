"""Parse kaiserlik/kjv per-book JSON into verse rows and tokens."""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path

from books import ABBREV_TO_BOOK, Book
from tokenizer import Token, plain_text_from_tokens, tokenize_english

VERSE_KEY_RE = re.compile(r"^(?P<abbrev>[A-Za-z0-9]+)\|(?P<chapter>\d+)\|(?P<verse>\d+)$")

# kaiserlik/kjv embeds bg/ch/sp strings with invalid JSON escapes; we only ingest English.
# Books use either minified ("Mar|1|1":{"en":"...") or pretty-printed JSON with whitespace.
VERSE_EN_RE = re.compile(
    r'"([A-Za-z0-9]+\|(\d+)\|(\d+))":\s*\{\s*"en":\s*"((?:\\.|[^"\\])*)"\s*,',
    re.MULTILINE,
)


@dataclass(frozen=True, slots=True)
class ParsedVerse:
    book: Book
    chapter: int
    verse: int
    tokens: list[Token]
    plain_text: str


def _parse_verse_key(key: str) -> tuple[str, int, int]:
    m = VERSE_KEY_RE.match(key)
    if not m:
        raise ValueError(f"unexpected verse key: {key!r}")
    return m.group("abbrev"), int(m.group("chapter")), int(m.group("verse"))


def _decode_json_string(raw: str) -> str:
    return json.loads(f'"{raw}"')


def _chapter_block_re(abbrev: str) -> re.Pattern[str]:
    return re.compile(rf'"{re.escape(abbrev)}\|(\d+)":\s*\{{')


def _slice_braced_object(text: str, open_brace_index: int) -> str:
    """Return inner text of `{...}` starting at the opening brace index."""
    depth = 0
    i = open_brace_index
    while i < len(text):
        if text[i] == "{":
            depth += 1
        elif text[i] == "}":
            depth -= 1
            if depth == 0:
                return text[open_brace_index + 1 : i]
        i += 1
    raise ValueError("unclosed chapter object")


def _iter_chapter_blocks(text: str, abbrev: str):
    """Yield (chapter_num, inner_text) for each `Abbrev|N`: { ... }` block."""
    for match in _chapter_block_re(abbrev).finditer(text):
        chapter = int(match.group(1))
        inner = _slice_braced_object(text, match.end() - 1)
        yield chapter, inner


def parse_book_json(
    path: Path,
    *,
    chapters: set[int] | None = None,
) -> list[ParsedVerse]:
    text = path.read_text(encoding="utf-8")
    expected_abbrev = path.stem
    book = ABBREV_TO_BOOK.get(expected_abbrev)
    if book is None:
        raise ValueError(f"unknown book abbrev {expected_abbrev!r} ({path})")

    verses: list[ParsedVerse] = []
    for chapter_num, block in _iter_chapter_blocks(text, expected_abbrev):
        if chapters is not None and chapter_num not in chapters:
            continue
        for match in VERSE_EN_RE.finditer(block):
            verse_key, en_raw = match.group(1), match.group(4)
            abbrev, chapter, verse_num = _parse_verse_key(verse_key)
            if abbrev != expected_abbrev:
                continue
            # OT books nest prior chapters inside later chapter blocks; keep only
            # verses belonging to this chapter (e.g. "1Ki|2|1" under the "1Ki|2" block).
            if chapter != chapter_num:
                continue
            en = _decode_json_string(en_raw)
            tokens = tokenize_english(en)
            verses.append(
                ParsedVerse(
                    book=book,
                    chapter=chapter,
                    verse=verse_num,
                    tokens=tokens,
                    plain_text=plain_text_from_tokens(tokens),
                )
            )

    if not verses:
        raise ValueError(f"no English verses found in {path}")

    verses.sort(key=lambda v: (v.book.id, v.chapter, v.verse))
    return verses
