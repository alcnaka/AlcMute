"""Check generated local links, embedded fonts, and Markdown/PDF text coverage."""
from html.parser import HTMLParser
from pathlib import Path
import re
import os
import unicodedata
from urllib.parse import unquote, urlsplit

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
    pages = list(SITE.rglob("*.html"))
    assert len(pages) >= 12, "Both guide languages must be built"
    parsed = {}
    for path in pages:
        page = Page()
        page.feed(path.read_text(encoding="utf-8"))
        assert len(page.ids) == len(set(page.ids)), f"Duplicate anchors: {path}"
        parsed[path.resolve()] = page
    base = os.environ.get("DOCS_BASE_PATH", "/").rstrip("/")
    for path, page in parsed.items():
        for link in page.links:
            url = urlsplit(link)
            if url.scheme or url.netloc or link == "#":
                continue
            target_path = unquote(url.path)
            if target_path.startswith("/"):
                assert not base or target_path == base or target_path.startswith(base + '/'), f"Outside base: {link}"
                target = SITE / target_path[len(base):].lstrip("/")
            else:
                target = path.parent / target_path if target_path else path
            if target.is_dir():
                target /= 'index.html'
            assert target.is_file(), f"Missing local target: {path} -> {link}"
            if url.fragment and target.suffix == '.html':
                assert unquote(url.fragment) in parsed[target.resolve()].ids, f"Missing anchor: {link}"
    for english in [False, True]:
        verify_pdf(english)
    for locale in ["", "en/"]:
        index = (SITE / locale / "index.html").read_text(encoding="utf-8")
        assert 'AlcMute-Manual' in index, "Introduction must link to the PDF"
        assert '<html lang="' + ('en' if locale else 'ja') + '"' in index
    print(f"Verified local links and anchors across {len(pages)} pages in Japanese and English")


def verify_pdf(english):
    pdf = ROOT / "output/pdf" / ("AlcMute-Manual-en.pdf" if english else "AlcMute-Manual.pdf")
    assert pdf.read_bytes() == (SITE / pdf.name).read_bytes(), "Web PDF differs"
    reader = PdfReader(pdf)
    assert len(reader.pages) >= 2
    extracted = normalized("".join(p.extract_text() for p in reader.pages))
    parser = MarkdownIt("commonmark", {"html": False}).enable("table")
    tokens = parser.parse((ROOT / "docs" / ("user-guide.en.md" if english else "user-guide.md")).read_text(encoding="utf-8"))
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
