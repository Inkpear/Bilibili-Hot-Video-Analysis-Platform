#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""Hadoop Streaming mapper for word count with bvid dimension.

Input:  bvid \t category \t text
Output: bvid \t category \t word \t 1

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

STOPWORDS_FILE = sys.argv[1] if len(sys.argv) > 1 else "stopwords.txt"
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


def cut_words(text):
    if jieba is not None:
        for word in jieba.lcut(text):
            yield word.strip().lower()
        return
    for word in fallback_cut(text):
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


def emit(bvid, category, word):
    write_line(u"%s\t%s\t%s\t1" % (bvid, category, word))


def main():
    for raw in iter_input():
        line = to_text(raw).rstrip(u"\n")
        if not line:
            continue
        parts = line.split(u"\t", 2)
        if len(parts) != 3:
            continue
        bvid, category, text = parts[0].strip(), parts[1].strip(), parts[2].strip()
        if not bvid or not text:
            continue

        words = set(word for word in cut_words(text) if keep_word(word))
        for word in sorted(words):
            emit(bvid, category, word)
            if category != u"__ALL__":
                emit(bvid, u"__ALL__", word)


if __name__ == "__main__":
    main()
