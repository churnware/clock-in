"""Render the waving lucky cat to docs/cat-{dark,light}.webp.

Each animated WebP plays the paw's beckon a few times and then holds the
last frame. Frames come from anim::cat, so they track changes to the sprite
and its colours.

    python utils/render_cat.py [--face HH:MM] [--loops N]
"""
import argparse

import frames
from _common import DOCS
from ansi import Terminal

# One full swing of the eyes and tail; the paw beckons twice in it.
CYCLE = 8

# From src/tui/theme.rs.
WHITE = (0xFF, 0xFF, 0xFF)
BRAND = (0x4B, 0x53, 0xBC)
DARK = (0x46, 0x47, 0x75)


def mix(a, b, t):
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b))


def dark(rgb):
    return rgb


def light(rgb):
    """White eyes and digits become brand ink; the pale sky and lavender
    tones deepen towards Teams' dark purple so they hold up on white."""
    return BRAND if rgb == WHITE else mix(rgb, DARK, 0.3)


def crop(images, margin=8):
    """Crop every frame to the area any frame draws on, so the canvas's blank
    columns don't push the cat off-centre."""
    boxes = [im.getchannel("A").getbbox() for im in images]
    box = (min(b[0] for b in boxes) - margin, min(b[1] for b in boxes) - margin,
           max(b[2] for b in boxes) + margin, max(b[3] for b in boxes) + margin)
    return [im.crop(box) for im in images]


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--face", default="09:00", help="time on the cat's clock")
    p.add_argument("--loops", type=int, default=3)
    p.add_argument("--size", type=int, default=32, help="font size in px")
    a = p.parse_args()
    shots = frames.load("dump_cat_frames", CYCLE, CLOCK_IN_FACE=a.face)
    term = Terminal(a.size)
    for name, palette in (("dark", dark), ("light", light)):
        images = crop([term.render(lines, WHITE, palette) for lines in shots])
        frames.save(images, str(DOCS / f"cat-{name}.webp"), a.loops)


if __name__ == "__main__":
    main()
