"""Check generated local links, embedded fonts, and Markdown/PDF text coverage."""
from html.parser import HTMLParser
from pathlib import Path
import re
import unicodedata
from urllib.parse import unquote

from markdown_it import MarkdownIt
from pypdf import PdfReader

ROOT = Path(__file__).resolve().parents[2]
SITE = ROOT / "output/site"


class Page(HTMLParser):
    def __init__(self):
        super().__init__()
        self.ids = []
        self.links = []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if "id" in attrs:
            self.ids.append(attrs["id"])
        for attr in ("href", "src"):
            if attr in attrs:
                self.links.append(attrs[attr])


def normalized(text):
    return re.sub(r"\s+", "", unicodedata.normalize("NFKC", text))


def main():
    page = Page()
    page.feed((SITE / "index.html").read_text(encoding="utf-8"))
    assert len(page.ids) == len(set(page.ids)), "Duplicate HTML anchors"
    for link in page.links:
        if link == "#" or link.startswith(("https://", "http://")):
            continue
        if link.startswith("#"):
            assert unquote(link[1:]) in page.ids, f"Missing anchor: {link}"
        else:
            assert (SITE / link).is_file(), f"Missing local asset: {link}"
    pdf = ROOT / "output/pdf/AlcMute-Manual.pdf"
    assert pdf.read_bytes() == (SITE / pdf.name).read_bytes(), "Web PDF differs"
    reader = PdfReader(pdf)
    assert len(reader.pages) >= 2
    extracted = normalized("".join(p.extract_text() for p in reader.pages))
    parser = MarkdownIt("commonmark", {"html": False}).enable("table")
    tokens = parser.parse((ROOT / "docs/user-guide.md").read_text(encoding="utf-8"))
    body = False
    checked = 0
    for token in tokens:
        if token.type == "heading_open" and token.tag == "h2":
            body = True
        if body and token.type == "inline":
            text = "".join(" " if c.type in ("softbreak", "hardbreak") else c.content
                           for c in token.children or [] if c.type in ("text", "code_inline", "softbreak", "hardbreak", "image"))
            assert normalized(text) in extracted, f"Missing PDF text: {text[:100]}"
            checked += 1
    for p in reader.pages:
        for font in p["/Resources"]["/Font"].values():
            descriptor = font.get_object().get("/FontDescriptor")
            if descriptor:
                assert "/FontFile2" in descriptor.get_object(), "Font is not embedded"
    print(f"Verified: local links/anchors, matching PDFs, {len(reader.pages)} pages, {checked} source text blocks, embedded TrueType fonts")


if __name__ == "__main__":
    main()
