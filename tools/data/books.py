"""Canonical book metadata aligned with kaiserlik/kjv (books.json order)."""

from __future__ import annotations

from dataclasses import dataclass

# (name, abbrev) — order matches kaiserlik/kjv/main/books.json
_KAISERLIK_BOOKS: tuple[tuple[str, str], ...] = (
    ("Genesis", "Gen"),
    ("Exodus", "Exo"),
    ("Leviticus", "Lev"),
    ("Numbers", "Num"),
    ("Deuteronomy", "Deu"),
    ("Joshua", "Jos"),
    ("Judges", "Jdg"),
    ("Ruth", "Rth"),
    ("1 Samuel", "1Sa"),
    ("2 Samuel", "2Sa"),
    ("1 Kings", "1Ki"),
    ("2 Kings", "2Ki"),
    ("1 Chronicles", "1Ch"),
    ("2 Chronicles", "2Ch"),
    ("Ezra", "Ezr"),
    ("Nehemiah", "Neh"),
    ("Esther", "Est"),
    ("Job", "Job"),
    ("Psalms", "Psa"),
    ("Proverbs", "Pro"),
    ("Ecclesiastes", "Ecc"),
    ("Song of Songs", "Sng"),
    ("Isaiah", "Isa"),
    ("Jeremiah", "Jer"),
    ("Lamentations", "Lam"),
    ("Ezekiel", "Eze"),
    ("Daniel", "Dan"),
    ("Hosea", "Hos"),
    ("Joel", "Joe"),
    ("Amos", "Amo"),
    ("Obadiah", "Oba"),
    ("Jonah", "Jon"),
    ("Micah", "Mic"),
    ("Nahum", "Nah"),
    ("Habakkuk", "Hab"),
    ("Zephaniah", "Zep"),
    ("Haggai", "Hag"),
    ("Zechariah", "Zec"),
    ("Malachi", "Mal"),
    ("Matthew", "Mat"),
    ("Mark", "Mar"),
    ("Luke", "Luk"),
    ("John", "Jhn"),
    ("Acts", "Act"),
    ("Romans", "Rom"),
    ("1 Corinthians", "1Co"),
    ("2 Corinthians", "2Co"),
    ("Galatians", "Gal"),
    ("Ephesians", "Eph"),
    ("Philippians", "Phl"),
    ("Colossians", "Col"),
    ("1 Thessalonians", "1Th"),
    ("2 Thessalonians", "2Th"),
    ("1 Timothy", "1Ti"),
    ("2 Timothy", "2Ti"),
    ("Titus", "Tit"),
    ("Philemon", "Phm"),
    ("Hebrews", "Heb"),
    ("James", "Jas"),
    ("1 Peter", "1Pe"),
    ("2 Peter", "2Pe"),
    ("1 John", "1Jo"),
    ("2 John", "2Jo"),
    ("3 John", "3Jo"),
    ("Jude", "Jde"),
    ("Revelation", "Rev"),
)


@dataclass(frozen=True, slots=True)
class Book:
    id: int
    osis: str
    abbrev: str
    name: str
    testament: str


BOOKS: tuple[Book, ...] = tuple(
    Book(
        id=i + 1,
        osis=abbrev,
        abbrev=abbrev,
        name=name,
        testament="OT" if i < 39 else "NT",
    )
    for i, (name, abbrev) in enumerate(_KAISERLIK_BOOKS)
)

ABBREV_TO_BOOK: dict[str, Book] = {b.abbrev: b for b in BOOKS}
NAME_TO_BOOK: dict[str, Book] = {b.name: b for b in BOOKS}
