"""Japanese PDF line breaking, shared by body text, headings, captions and tables.

Keep source characters and ReportLab fragments (fonts, links and anchors) intact.
Choose legal break positions across fragment boundaries instead of fixing prose
or inserting nonbreaking characters into Markdown.
"""
import unicodedata

from reportlab.platypus import Paragraph as BaseParagraph
from reportlab.platypus.paragraph import (
    ParaLines, cjkU, makeCJKParaLine, _handleBulletWidth,
)

NO_START = frozenset("、。，．・：；？！‼⁇⁈⁉)]}）］｝〕〉》」』】〙〗〟’”»›ぁぃぅぇぉっゃゅょゎァィゥェォッャュョヮヵヶ々ーゝゞヽヾ゛゜,.:;!?")
NO_END = frozenset("([{（［｛〔〈《「『【〘〖〝‘“«‹")


def can_break(left, right, keep_latin=True):
    if not left or not right:
        return True
    if left in NO_END or right in NO_START:
        return False
    if unicodedata.category(right).startswith('M') or right in ('\ufe0e', '\ufe0f', '\u200d') or left == '\u200d':
        return False
    if left == right and left in ('…', '‥', '―'):
        return False
    if keep_latin and left.isascii() and right.isascii() and not left.isspace() and not right.isspace():
        return False
    return True


class Paragraph(BaseParagraph):
    def breakLinesCJK(self, maxWidths):
        if hasattr(self, 'blPara') and getattr(self, '_splitpara', False):
            return self.blPara
        widths = list(maxWidths) if isinstance(maxWidths, (list, tuple)) else [maxWidths]
        _handleBulletWidth(self.bulletText, self.style, widths)
        self.height = 0
        glyphs = []
        for frag in self.frags:
            text = frag.text
            if isinstance(text, bytes):
                text = text.decode('utf-8')
            glyphs.extend(cjkU(char, frag, 'utf-8') for char in text or [''])
        if not glyphs:
            return super().breakLinesCJK(maxWidths)
        # Empty fragments are PDF anchors; ignore them only when inspecting
        # adjacent characters, never when emitting a line.
        left, right = [''] * (len(glyphs) + 1), [''] * (len(glyphs) + 1)
        for i, glyph in enumerate(glyphs):
            left[i + 1] = str(glyph) or left[i]
        for i in range(len(glyphs) - 1, -1, -1):
            right[i] = str(glyphs[i]) or right[i + 1]
        legal = [can_break(a, b) for a, b in zip(left, right)]
        emergency = [can_break(a, b, False) for a, b in zip(left, right)]
        lines, start = [], 0
        calc_bounds = getattr(self, 'autoLeading', getattr(self.style, 'autoLeading', '')) not in ('', 'off')
        while start < len(glyphs):
            limit = widths[min(len(lines), len(widths) - 1)]
            end, used, preferred, fallback = start, 0, None, None
            while end < len(glyphs):
                width = glyphs[end].width
                if hasattr(width, 'normalizedValue'):
                    width = width.normalizedValue(limit)
                if used + width > limit + 1e-8:
                    break
                used += width
                end += 1
                if legal[end]:
                    preferred = end
                if emergency[end]:
                    fallback = end
            if end < len(glyphs):
                end = preferred or fallback or start + 1
                # An unbreakable punctuation/combining sequence can be wider
                # than a tiny column. Keep it intact and guarantee progress.
                while end < len(glyphs) and not emergency[end]:
                    end += 1
            used = sum(float(g.width) for g in glyphs[start:end])
            lines.append(makeCJKParaLine(glyphs[start:end], limit, used, limit - used, False, calc_bounds))
            start = end
        return ParaLines(kind=1, lines=lines)
