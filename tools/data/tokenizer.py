"""Tokenize kaiserlik KJV English strings with inline Strong's tags."""

from __future__ import annotations

import html
import re
from dataclasses import dataclass

TAG_RE = re.compile(r"\[(G|H)(\d+)\]")

FLAG_ITALIC = 1 << 0

# Sentinels for italic regions after HTML tags are removed (not present in source text).
_ITALIC_ON = "\x01"
_ITALIC_OFF = "\x02"
# Capturing group keeps sentinels in re.split() results.
_ITALIC_MARKERS_RE = re.compile(
    f"({re.escape(_ITALIC_ON)}|{re.escape(_ITALIC_OFF)})"
)


@dataclass(frozen=True, slots=True)
class Token:
    surface: str
    strong_key: str | None
    flags: int = 0


def normalize_strong_key(lang: str, num: str) -> str:
    return f"{lang.upper()}{int(num)}"


def strip_strongs_markers(text: str) -> str:
    return TAG_RE.sub("", text)


def strip_html_markup(text: str) -> str:
    """Remove HTML from kaiserlik KJV strings; preserve italics as inline sentinels."""
    text = html.unescape(text)
    text = re.sub(r"<br\s*/?\s*>", " ", text, flags=re.IGNORECASE)
    text = re.sub(r"<(?:em|i)\b[^>]*>", _ITALIC_ON, text, flags=re.IGNORECASE)
    text = re.sub(r"</(?:em|i)\s*>", _ITALIC_OFF, text, flags=re.IGNORECASE)
    text = re.sub(r"<[^>]+>", "", text)
    return text


def _emit_words(chunk: str, *, italic_depth: int) -> tuple[list[Token], int]:
    """Tokenize a chunk that may contain italic sentinels (no Strong's tags)."""
    tokens: list[Token] = []
    depth = italic_depth
    if not chunk:
        return tokens, depth

    for part in _ITALIC_MARKERS_RE.split(chunk):
        if part == _ITALIC_ON:
            depth += 1
            continue
        if part == _ITALIC_OFF:
            depth = max(0, depth - 1)
            continue
        if not part:
            continue
        flags = FLAG_ITALIC if depth > 0 else 0
        for word in part.split():
            tokens.append(Token(surface=word, strong_key=None, flags=flags))
    return tokens, depth


def tokenize_english(text: str) -> list[Token]:
    """Parse 'word[G123]' — tag applies to the word immediately before it."""
    text = strip_html_markup(text)
    tokens: list[Token] = []
    italic_depth = 0
    pos = 0
    for match in TAG_RE.finditer(text):
        before = text[pos : match.start()]
        pos = match.end()
        key = normalize_strong_key(match.group(1), match.group(2))
        chunk_tokens, italic_depth = _emit_words(before, italic_depth=italic_depth)
        if chunk_tokens:
            # Strong's tag applies only to the final word before the marker.
            last = chunk_tokens[-1]
            if len(chunk_tokens) > 1:
                tokens.extend(chunk_tokens[:-1])
            tokens.append(
                Token(surface=last.surface, strong_key=key, flags=last.flags)
            )
        elif tokens:
            # Tag with no preceding word in this chunk — attach to previous token.
            last = tokens[-1]
            tokens[-1] = Token(
                surface=last.surface,
                strong_key=key,
                flags=last.flags,
            )

    tail, _ = _emit_words(text[pos:], italic_depth=italic_depth)
    tokens.extend(tail)
    return tokens


def plain_text_from_tokens(tokens: list[Token]) -> str:
    if not tokens:
        return ""
    parts = [tokens[0].surface]
    for tok in tokens[1:]:
        prev = parts[-1]
        cur = tok.surface
        if cur and cur[0] in ",.;:?!)]}\"'»”’":
            parts[-1] = prev + cur
        else:
            parts.append(cur)
    return " ".join(parts)
