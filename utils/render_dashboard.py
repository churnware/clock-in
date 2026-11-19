"""Render the dashboard in a terminal window to docs/dashboard.webp.

The animated WebP plays the cat's wave once and then holds the last
frame. Frames come from views::draw on the default schedule with the clock
pinned, so they track changes to the layout and theme.

    python utils/render_dashboard.py [--at "YYYY-MM-DD HH:MM"] [--loops N]
"""
import argparse

import frames
from _common import DOCS
from ansi import Terminal
from PIL import Image, ImageDraw

# Windows Terminal's dark background, outlined in Teams' dark purple from
# src/tui/theme.rs.
BG = (31, 31, 31)
OUTLINE = (0x46, 0x47, 0x75)
DEFAULT_FG = (204, 204, 204)
RADIUS = 10


def window(text):
    """The terminal's contents on a rounded window."""
    win = Image.new("RGBA", text.size, BG + (255,))
    win.alpha_composite(text)
    box = [0, 0, win.width - 1, win.height - 1]
    mask = Image.new("L", win.size, 0)
    ImageDraw.Draw(mask).rounded_rectangle(box, RADIUS, fill=255)
    win.putalpha(mask)
    ImageDraw.Draw(win).rounded_rectangle(box, RADIUS, outline=OUTLINE + (255,), width=1)
    return win


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--at", default="2026-11-19 10:47", help="the dashboard's date and time")
    p.add_argument("--cols", type=int, default=96)
    p.add_argument("--rows", type=int, default=27)
    p.add_argument("--loops", type=int, default=1)
    p.add_argument("--size", type=int, default=16, help="font size in px")
    a = p.parse_args()
    shots = frames.load("dump_dashboard_frames", frames.FULL_CYCLE,
                        CLOCK_IN_DEMO_AT=a.at, CLOCK_IN_COLS=a.cols, CLOCK_IN_ROWS=a.rows)
    term = Terminal(a.size, pad=(14, 12))
    images = [window(term.render(lines, DEFAULT_FG, cols=a.cols)) for lines in shots]
    frames.save(images, str(DOCS / "dashboard.webp"), a.loops)


if __name__ == "__main__":
    main()
