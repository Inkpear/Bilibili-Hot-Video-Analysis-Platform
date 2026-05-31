#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""Hadoop Streaming mapper for title word count.

Compatible with Python 2.7 on CentOS 7 Hadoop nodes and Python 3 locally.
"""

from __future__ import print_function

import os
import re
import sys

try:
    import jieba  # optional on worker nodes
except Exception:  # pragma: no cover
    jieba = None


PY2 = sys.version_info[0] == 2
if PY2:
    text_type = unicode  # noqa: F821
else:
    text_type = str

STOPWORDS_FILE = "stopwords.txt"
TOKEN_RE = re.compile(u"[A-Za-z][A-Za-z0-9_+#.-]*|[0-9A-Za-z+#.-]{2,}|[\u4e00-\u9fff]+")
CJK_RE = re.compile(u"^[\u4e00-\u9fff]+$")


def to_text(value):
    if isinstance(value, text_type):
        return value
    return value.decode("utf-8", "ignore")


def write_line(value):
    if PY2:
        sys.stdout.write(value.encode("utf-8") + "\n")
    else:
        sys.stdout.buffer.write(value.encode("utf-8") + b"\n")


def iter_input():
    if PY2:
        return sys.stdin
    return sys.stdin.buffer


def load_stopwords():
    stopwords = set()
    if not os.path.exists(STOPWORDS_FILE):
        return stopwords
    with open(STOPWORDS_FILE, "rb") as f:
        for raw in f:
            word = to_text(raw).strip().lower()
            if word and not word.startswith(u"#"):
                stopwords.add(word)
    return stopwords


STOPWORDS = load_stopwords()


def fallback_cut(text):
    for chunk in TOKEN_RE.findall(text):
        chunk = chunk.strip().lower()
        if not chunk:
            continue
        if CJK_RE.match(chunk) and len(chunk) > 4:
            for size in (2, 3, 4):
                for i in range(0, len(chunk) - size + 1):
                    yield chunk[i:i + size]
        else:
            yield chunk


def cut_words(title):
    if jieba is not None:
        for word in jieba.lcut(title):
            yield word.strip().lower()
        return
    for word in fallback_cut(title):
        yield word


def keep_word(word):
    if len(word) < 2:
        return False
    if word in STOPWORDS:
        return False
    if word.isdigit():
        return False
    if not TOKEN_RE.search(word):
        return False
    return True


def emit(category, word):
    write_line(u"%s\t%s\t1" % (category, word))


def main():
    for raw in iter_input():
        line = to_text(raw).rstrip(u"\n")
        if not line:
            continue
        parts = line.split(u"\t", 1)
        if len(parts) != 2:
            continue
        category, title = parts[0].strip(), parts[1].strip()
        if not category or not title:
            continue

        words = set(word for word in cut_words(title) if keep_word(word))
        for word in sorted(words):
            emit(u"__ALL__", word)
            emit(category, word)


if __name__ == "__main__":
    main()
