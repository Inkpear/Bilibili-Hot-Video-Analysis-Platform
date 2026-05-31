#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""Hadoop Streaming reducer for title word count.

Compatible with Python 2.7 on CentOS 7 Hadoop nodes and Python 3 locally.
"""

import sys


PY2 = sys.version_info[0] == 2
if PY2:
    text_type = unicode  # noqa: F821
else:
    text_type = str


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


def main():
    current_key = None
    current_count = 0

    for raw in iter_input():
        line = to_text(raw).rstrip(u"\n")
        if not line:
            continue
        parts = line.split(u"\t")
        if len(parts) != 3:
            continue
        category, word, count_raw = parts
        try:
            count = int(count_raw)
        except ValueError:
            continue

        key = (category, word)
        if current_key is None:
            current_key = key
            current_count = count
            continue

        if key == current_key:
            current_count += count
        else:
            write_line(u"%s\t%s\t%d" % (current_key[0], current_key[1], current_count))
            current_key = key
            current_count = count

    if current_key is not None:
        write_line(u"%s\t%s\t%d" % (current_key[0], current_key[1], current_count))


if __name__ == "__main__":
    main()
