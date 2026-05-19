"""Tests for KJV tokenizer / HTML stripping."""

from __future__ import annotations

import unittest

from tokenizer import FLAG_ITALIC, strip_html_markup, tokenize_english


class TokenizerTests(unittest.TestCase):
    def test_strip_em_tags(self) -> None:
        self.assertEqual(
            strip_html_markup("whose name <em>was</em> John."),
            "whose name \x01was\x02 John.",
        )

    def test_tokenize_italic_and_strong(self) -> None:
        tokens = tokenize_english("In the <em>beginning</em> was the Word[G3056].")
        beginning = next(t for t in tokens if t.surface == "beginning")
        self.assertEqual(beginning.flags, FLAG_ITALIC)
        word = next(t for t in tokens if t.strong_key == "G3056")
        self.assertEqual(word.surface, "Word")

    def test_no_html_in_output(self) -> None:
        raw = "He was not that Light, but <em>was sent</em> to bear witness[G3140]."
        for tok in tokenize_english(raw):
            self.assertNotIn("<", tok.surface)
            self.assertNotIn(">", tok.surface)


if __name__ == "__main__":
    unittest.main()
