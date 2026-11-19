"""Render ANSI-coloured output as it appears in the terminal."""
import re

from _common import find_font
from PIL import Image, ImageChops, ImageDraw, ImageFont

SGR = re.compile(r"\x1b\[([0-9;]*)m")


def parse(line):
    """Split an ANSI-coloured line into (char, fg, bg, bold) cells."""
    cells, fg, bg, is_bold, pos = [], None, None, False, 0
    for m in SGR.finditer(line):
        cells += [(ch, fg, bg, is_bold) for ch in line[pos:m.start()]]
        pos = m.end()
        codes = m.group(1).split(";")
        if codes in (["0"], [""]):
            fg, bg, is_bold = None, None, False
        elif codes[0] == "1":
            is_bold = True
        elif codes[:2] == ["38", "2"]:
            fg = tuple(map(int, codes[2:5]))
        elif codes[:2] == ["48", "2"]:
            bg = tuple(map(int, codes[2:5]))
    return cells + [(ch, fg, bg, is_bold) for ch in line[pos:]]


class Terminal:
    def __init__(self, size, pad=(8, 6)):
        mono = find_font("CascadiaMono.ttf")
        self.regular = ImageFont.truetype(mono, size)
        self.regular.set_variation_by_name("Regular")
        self.bold = ImageFont.truetype(mono, size)
        self.bold.set_variation_by_name("Bold")
        self.advance = self.regular.getlength("M")
        ascent, descent = self.regular.getmetrics()
        self.line = ascent + descent
        self.pad_x, self.pad_y = pad

    def _mask(self, cells, width):
        """Coverage mask for one row. Each weight is drawn as a whole run so
        box glyphs join exactly as the font spaces them."""
        mask = Image.new("L", (width, self.line), 0)
        for bold in {c[3] for c in cells}:
            text = "".join(ch if b == bold else " " for ch, _, _, b in cells)
            if not text.strip():
                continue
            layer = Image.new("L", (width, self.line), 0)
            font = self.bold if bold else self.regular
            ImageDraw.Draw(layer).text((self.pad_x, 0), text, font=font, fill=255)
            mask = ImageChops.lighter(mask, layer)
        return mask

    def _x(self, i):
        return round(self.pad_x + i * self.advance)

    def render(self, lines, default_fg, palette=lambda rgb: rgb, cols=None):
        """Transparent RGBA image of `lines`. Uncoloured text uses
        `default_fg`, and cells without a background stay transparent."""
        rows = [parse(line) for line in lines]
        cols = cols or max(map(len, rows), default=0)
        width = self._x(cols) + self.pad_x
        img = Image.new("RGBA", (width, self.pad_y * 2 + self.line * len(rows)), (0, 0, 0, 0))
        back = ImageDraw.Draw(img)
        for r, cells in enumerate(rows):
            y = self.pad_y + r * self.line
            for i, (_, _, bg, _) in enumerate(cells):
                if bg:
                    back.rectangle([self._x(i), y, self._x(i + 1) - 1, y + self.line - 1],
                                   fill=palette(bg) + (255,))
            colour = Image.new("RGB", (width, self.line))
            draw = ImageDraw.Draw(colour)
            for i, (_, fg, _, _) in enumerate(cells):
                draw.rectangle([self._x(i), 0, self._x(i + 1) + 1, self.line],
                               fill=palette(fg or default_fg))
            row = colour.convert("RGBA")
            row.putalpha(self._mask(cells, width))
            img.alpha_composite(row, (0, y))
        return img
