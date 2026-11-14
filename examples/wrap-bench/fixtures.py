"""Write the wrap-bench fixtures into an output directory.

Usage: python3 fixtures.py <output-dir>

Uses a fixed seed, so every run writes identical files.
"""

import os
import random
import string
import sys

TARGET_BYTES = 3_900_000
SINGLE_LINE_BYTES = 1_300_000
EMOJI = ["\U0001F600", "\U0001F680", "\U0001F389"]


def main():
    if len(sys.argv) != 2:
        sys.exit("usage: fixtures.py <output-dir>")
    out = sys.argv[1]
    os.makedirs(out, exist_ok=True)

    fixed_cjk = "".join(chr(0x4E00 + i * 37 % 20000) for i in range(60))
    fixed_latin = "the quick brown fox jumps over the lazy dog " * 3

    write(out, "latin_unique.txt", lambda r: latin_line(r))
    write(out, "mixed_repeat.txt", alternate(lambda r: fixed_cjk, lambda r: fixed_latin))
    write(out, "mixed_unique.txt", alternate(cjk_line, latin_line))
    write(out, "json_cjk_unique.txt", json_line)
    write(out, "tabs_unique.txt", tabs_line)
    write(out, "emoji_unique.txt", emoji_line)
    write_single_line(out, "cjk_single_line.txt")


def write(out, name, make_line, target=TARGET_BYTES):
    rng = random.Random(name)
    size = 0
    with open(os.path.join(out, name), "w", encoding="utf-8", newline="\n") as f:
        while size < target:
            line = make_line(rng) + "\n"
            f.write(line)
            size += len(line.encode("utf-8"))


def write_single_line(out, name):
    rng = random.Random(name)
    parts = []
    size = 0
    while size < SINGLE_LINE_BYTES:
        part = cjk_text(rng, 20, 100) if rng.random() < 0.5 else words(rng, 3)
        parts.append(part)
        size += len(part.encode("utf-8")) + 1
    with open(os.path.join(out, name), "w", encoding="utf-8", newline="\n") as f:
        f.write(" ".join(parts) + "\n")


def alternate(first, second):
    state = {"n": 0}

    def make(rng):
        state["n"] += 1
        return first(rng) if state["n"] % 2 == 1 else second(rng)

    return make


def word(rng):
    return "".join(rng.choices(string.ascii_lowercase, k=rng.randint(2, 10)))


def words(rng, count):
    return " ".join(word(rng) for _ in range(count))


def ascii_words_between(rng, lo, hi):
    """Words joined by spaces, total length between lo and hi characters."""
    want = rng.randint(lo, hi)
    out = []
    length = 0
    while True:
        w = word(rng)
        extra = len(w) + (1 if out else 0)
        if length + extra > want:
            break
        out.append(w)
        length += extra
    if not out:
        out.append("ab")
    return out


def latin_line(rng):
    return " ".join(ascii_words_between(rng, 40, 200))


def cjk_text(rng, lo, hi):
    return "".join(chr(rng.randint(0x4E00, 0x9FA5)) for _ in range(rng.randint(lo, hi)))


def cjk_line(rng):
    return cjk_text(rng, 20, 100)


def json_line(rng):
    key = "_".join(word(rng) for _ in range(rng.randint(1, 3)))
    return '    "%s": "%s",' % (key, cjk_text(rng, 10, 80))


def tabs_line(rng):
    return "\t" * rng.randint(1, 4) + " ".join(ascii_words_between(rng, 40, 200))


def emoji_line(rng):
    ws = ascii_words_between(rng, 40, 200)
    ws.insert(rng.randint(0, len(ws)), rng.choice(EMOJI))
    return " ".join(ws)


if __name__ == "__main__":
    main()
