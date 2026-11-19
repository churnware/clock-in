"""Render ANSI-coloured output as it appears in the terminal."""
import re

from _common import find_font
from PIL import Image, ImageChops, ImageDraw, ImageFont

SGR = re.compile(r"\x1b\[([0-9;]*)m")


def parse(line):
    """Split an ANSI-coloured line into (char, rgb, bold) cells."""
    cells, colour, is_bold, pos = [], None, False, 0
    for m in SGR.finditer(line):
        cells += [(ch, colour, is_bold) for ch in line[pos:m.start()]]
        pos = m.end()
        codes = m.group(1).split(";")
        if codes in (["0"], [""]):
            colour, is_bold = None, False
        elif codes[0] == "1":
            is_bold = True
        elif codes[:2] == ["38", "2"]:
            colour = tuple(map(int, codes[2:5]))
    return cells + [(ch, colour, is_bold) for ch in line[pos:]]


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
        for bold in {c[2] for c in cells}:
            text = "".join(ch if b == bold else " " for ch, _, b in cells)
            if not text.strip():
                continue
            layer = Image.new("L", (width, self.line), 0)
            font = self.bold if bold else self.regular
            ImageDraw.Draw(layer).text((self.pad_x, 0), text, font=font, fill=255)
            mask = ImageChops.lighter(mask, layer)
        return mask

    def render(self, lines, default_fg, palette=lambda rgb: rgb, cols=None):
        """Transparent RGBA image of `lines`; uncoloured text uses `default_fg`."""
        rows = [parse(line) for line in lines]
        cols = cols or max(map(len, rows), default=0)
        width = int(self.pad_x * 2 + cols * self.advance) + 2
        img = Image.new("RGBA", (width, self.pad_y * 2 + self.line * len(rows)), (0, 0, 0, 0))
        for r, cells in enumerate(rows):
            colour = Image.new("RGB", (width, self.line))
            draw = ImageDraw.Draw(colour)
            for i, (_, rgb, _) in enumerate(cells):
                x = self.pad_x + i * self.advance
                draw.rectangle([round(x), 0, round(x + self.advance) + 1, self.line],
                               fill=palette(rgb or default_fg))
            row = colour.convert("RGBA")
            row.putalpha(self._mask(cells, width))
            img.alpha_composite(row, (0, self.pad_y + r * self.line))
        return img
