"""Parse Open Scriptures Strong's dictionary JS files."""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from pathlib import Path

XREF_RE = re.compile(r"[GH]\d+")

@dataclass(frozen=True, slots=True)
class StrongEntry:
    key: str
    lang: str
    lemma: str | None
    translit: str | None
    definition: str
    kjv_gloss: str | None
    derivation: str | None
    xrefs: tuple[str, ...]


def _normalize_key(raw: str) -> str:
    m = re.fullmatch(r"([GH])(\d+)", raw.strip(), re.IGNORECASE)
    if not m:
        raise ValueError(f"invalid strong key: {raw!r}")
    return f"{m.group(1).upper()}{int(m.group(2))}"


def _extract_xrefs(derivation: str | None) -> tuple[str, ...]:
    if not derivation:
        return ()
    seen: set[str] = set()
    out: list[str] = []
    for match in XREF_RE.finditer(derivation):
        key = _normalize_key(match.group(0))
        if key not in seen:
            seen.add(key)
            out.append(key)
    return tuple(out)


def _load_js_object(path: Path) -> dict:
    text = path.read_text(encoding="utf-8")
    start = text.index("{")
    end = text.rindex("}") + 1
    return json.loads(text[start:end])


def parse_strongs_dictionary(path: Path, lang: str) -> list[StrongEntry]:
    raw = _load_js_object(path)
    entries: list[StrongEntry] = []
    for raw_key, fields in raw.items():
        key = _normalize_key(raw_key)
        derivation = fields.get("derivation")
        definition = fields.get("strongs_def") or fields.get("definition") or ""
        if not definition.strip():
            definition = fields.get("kjv_def") or key
        entries.append(
            StrongEntry(
                key=key,
                lang=lang,
                lemma=fields.get("lemma"),
                translit=fields.get("translit"),
                definition=definition.strip(),
                kjv_gloss=fields.get("kjv_def"),
                derivation=derivation,
                xrefs=_extract_xrefs(derivation),
            )
        )
    return entries
