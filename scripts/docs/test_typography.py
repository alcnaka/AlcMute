import unittest
from io import BytesIO

from reportlab.lib.styles import ParagraphStyle
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.cidfonts import UnicodeCIDFont
from typography import Paragraph, NO_START, NO_END
from build import BodyParagraph
from reportlab.platypus import SimpleDocTemplate, Spacer
from pypdf import PdfReader


class JapaneseLineBreaking(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        pdfmetrics.registerFont(UnicodeCIDFont('HeiseiMin-W3'))
        pdfmetrics.registerFontFamily('HeiseiMin-W3', normal='HeiseiMin-W3', bold='HeiseiMin-W3', italic='HeiseiMin-W3', boldItalic='HeiseiMin-W3')

    def check_lines(self, markup, text, width):
        paragraph = Paragraph(markup, ParagraphStyle('test', fontName='HeiseiMin-W3', fontSize=10, leading=16, wordWrap='CJK'))
        paragraph.wrap(width, 10000)
        lines = [''.join(f.text for f in line.words) for line in paragraph.blPara.lines]
        self.assertEqual(''.join(lines), text)
        for line in lines:
            if line.strip():
                self.assertNotIn(line.lstrip()[0], NO_START, (width, line))
                self.assertNotIn(line.rstrip()[-1], NO_END, (width, line))
        for line in paragraph.blPara.lines:
            self.assertLessEqual(line.currentWidth, width + 1e-7)
        return paragraph

    def test_punctuation_brackets_and_fragment_boundaries_at_many_widths(self):
        text = '設定画面はスタートメニューの「AlcMute」、またはトレイメニューの「設定」から開けます。'
        decorated = '設定画面はスタートメニューの「<b>AlcMute</b>」<a href="https://example.com/">、または</a>トレイメニューの「設定」から開けます。'
        for width in range(80, 490, 3):
            for markup in [text, decorated]:
                with self.subTest(width=width, decorated=markup != text):
                    self.check_lines(markup, text, width)

    def test_small_kana_punctuation_runs_and_combining_characters(self):
        text = '「きゃっ」と（小さな声）で話す。えっ？！そう……が、わかった。'
        for width in range(70, 200, 3):
            self.check_lines(text, text, width)

    def test_links_and_zero_width_anchors_survive(self):
        p = self.check_lines('<a name="section"/>本文の<a href="https://example.com/">「設定」、保存</a>を確認。', '本文の「設定」、保存を確認。', 90)
        frags = [f for line in p.blPara.lines for f in line.words]
        self.assertTrue(any(getattr(f, 'link', None) for f in frags))
        self.assertTrue(any(getattr(f, 'cbDefn', None) for f in frags))

    def test_heading_moves_with_the_following_paragraph(self):
        output = BytesIO()
        style = ParagraphStyle('body', fontName='Helvetica', fontSize=10, leading=12)
        heading = ParagraphStyle('heading', parent=style, keepWithNext=True)
        SimpleDocTemplate(output, pagesize=(200, 200), topMargin=10, bottomMargin=10,
                          leftMargin=10, rightMargin=10).build([
            Spacer(1, 140), Paragraph('Security heading', heading),
            BodyParagraph('First paragraph with enough text to fill several lines and keep the heading on the same page.', style),
        ])
        pages = PdfReader(output).pages
        self.assertEqual(len(pages), 2)
        self.assertNotIn('Security heading', pages[0].extract_text())
        self.assertIn('Security heading', pages[1].extract_text())
        self.assertIn('First paragraph', pages[1].extract_text())


if __name__ == '__main__':
    unittest.main()
