"""Build the static guide and PDF from docs/user-guide.md; no hosting/network."""
import argparse
import html
import os
from pathlib import Path
import re
import shutil
import tomllib

from markdown_it import MarkdownIt
from reportlab.lib import colors
from reportlab.lib.enums import TA_CENTER
from reportlab.lib.pagesizes import A4
from reportlab.lib.styles import ParagraphStyle
from reportlab.pdfbase import pdfmetrics
from reportlab.pdfbase.ttfonts import TTFont
from reportlab.platypus import (
    SimpleDocTemplate, Paragraph, Spacer, Table, TableStyle, Image, PageBreak,
    KeepTogether,
)

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "docs/user-guide.md"
REPOSITORY = "https://github.com/alcnaka/AlcMute"


def slug(text):
    return re.sub(r"[^\w\- ]", "", text.lower()).replace(" ", "-")


def link(href):
    if href == "../README.md":
        return REPOSITORY
    if href == "../CHANGELOG.md":
        return REPOSITORY + "/blob/main/CHANGELOG.md"
    return href


def source_image(src):
    path = (SOURCE.parent / src).resolve()
    if not path.is_relative_to(ROOT) or not path.is_file():
        raise ValueError(f"Missing/outside-repository image: {src}")
    return path


def prepare_tokens(parser, text):
    tokens = parser.parse(text)
    headings = []
    for i, token in enumerate(tokens):
        if token.type == "heading_open":
            title = tokens[i + 1].content
            token.attrSet("id", slug(title))
            if token.tag == "h2":
                headings.append((slug(title), title))
        for child in token.children or []:
            if child.type == "link_open":
                child.attrSet("href", link(child.attrGet("href")))
    return tokens, headings


def inline(children):
    result = []
    for child in children or []:
        if child.type in ("text", "code_inline"):
            result.append(html.escape(child.content))
        elif child.type in ("softbreak", "hardbreak"):
            result.append(" ")
        elif child.type in ("strong_open", "strong_close", "em_open", "em_close"):
            tag = "b" if child.type.startswith("strong") else "i"
            result.append(f"<{'/' if child.nesting == -1 else ''}{tag}>")
        elif child.type == "link_open":
            result.append(f'<a color="#16718d" href="{html.escape(child.attrGet("href"), quote=True)}">')
        elif child.type == "link_close":
            result.append("</a>")
        elif child.type == "image":
            continue
        else:
            raise ValueError(f"Unsupported inline Markdown: {child.type}")
    return "".join(result)


def build_pdf(tokens, headings, output, version, font_path, bold_path):
    # Embed a local TrueType Japanese font; no PDF reader/system-font dependency.
    pdfmetrics.registerFont(TTFont("Guide", str(font_path), subfontIndex=0))
    pdfmetrics.registerFont(TTFont("GuideBold", str(bold_path), subfontIndex=0))
    pdfmetrics.registerFontFamily("Guide", normal="Guide", bold="GuideBold", italic="Guide", boldItalic="GuideBold")
    body = ParagraphStyle("body", fontName="Guide", fontSize=10.5, leading=17,
                          wordWrap="CJK", textColor=colors.HexColor("#243747"), spaceAfter=6)
    styles = {1: ParagraphStyle("title", parent=body, fontName="GuideBold", fontSize=27, leading=38, spaceAfter=24),
              2: ParagraphStyle("section", parent=body, fontName="GuideBold", fontSize=18, leading=27, textColor=colors.HexColor("#126b85"), spaceBefore=18, spaceAfter=13, keepWithNext=True),
              3: ParagraphStyle("subsection", parent=body, fontName="GuideBold", fontSize=12, leading=20, spaceBefore=12, spaceAfter=8, keepWithNext=True)}
    cell = ParagraphStyle("cell", parent=body, fontSize=9.5, leading=15, spaceAfter=0)
    logo = Image(str(ROOT / "assets/app.png"), 90, 90)
    logo.hAlign = "LEFT"
    story = [logo, Spacer(1, 22),
             Paragraph("AlcMute", styles[1]), Paragraph("あるみゅーと / VRC Background Mute", body),
             Paragraph("ご利用マニュアル", styles[2]),
             Paragraph(f"ソースバージョン {version} / Windows 10・11", body),
             Paragraph("このマニュアルはリポジトリ内の利用ガイドから生成しています。公開版に未収録の変更を含む場合があります。", body),
             Spacer(1, 20), Paragraph("VRChatを操作していないときも、ショートカットや通知領域からマイクのミュートを切り替えられます。", body),
             Spacer(1, 24), Paragraph("目次", styles[3])]
    for anchor, title in headings:
        story.append(Paragraph(f'<a href="#{anchor}" color="#126b85">{html.escape(title)}</a>', body))
    story.append(PageBreak())
    i = 0
    list_stack = []
    first_section = True
    while i < len(tokens):
        t = tokens[i]
        if t.type == "heading_open":
            level = int(t.tag[1])
            if level == 1:
                i += 3
                continue
            if level == 2:
                if not first_section:
                    story.append(PageBreak())
                first_section = False
            story.append(Paragraph(f'<a name="{t.attrGet("id")}"/>' + inline(tokens[i + 1].children), styles.get(level, styles[3])))
            i += 3
            continue
        # The source's repository backlink/contents are replaced by a PDF TOC.
        if first_section:
            i += 1
            continue
        if t.type in ("bullet_list_open", "ordered_list_open"):
            list_stack.append(int(t.attrGet("start") or 1) if t.type == "ordered_list_open" else None)
        elif t.type in ("bullet_list_close", "ordered_list_close"):
            list_stack.pop()
        elif t.type == "paragraph_open":
            content = tokens[i + 1]
            images = [c for c in content.children or [] if c.type == "image"]
            if images:
                from PIL import Image as PILImage
                for image in images:
                    path = source_image(image.attrGet("src"))
                    with PILImage.open(path) as im:
                        w, h = im.size
                    scale = min(360 / w, 310 / h)
                    caption = Paragraph(html.escape(image.content), ParagraphStyle("caption", parent=body, fontSize=8, leading=13, alignment=TA_CENTER, textColor=colors.HexColor("#587080")))
                    story.append(KeepTogether([Image(str(path), w * scale, h * scale), Spacer(1, 6), caption]))
            else:
                bullet = None
                if list_stack:
                    value = list_stack[-1]
                    bullet = "-" if value is None else f"{value}."
                    if value is not None:
                        list_stack[-1] += 1
                style = body if not bullet else ParagraphStyle("item", parent=body, leftIndent=18, firstLineIndent=0, bulletIndent=0)
                story.append(KeepTogether([Paragraph(inline(content.children), style, bulletText=bullet)]))
            i += 3
            continue
        elif t.type == "fence":
            # Paragraphs wrap long paths without clipping the page.
            story.append(Table([[Paragraph(html.escape(t.content.strip()), cell)]], colWidths=[475], style=TableStyle([
                ("BACKGROUND", (0, 0), (-1, -1), colors.HexColor("#eef4f7")),
                ("BOX", (0, 0), (-1, -1), .5, colors.HexColor("#d3e2e9")),
                ("LEFTPADDING", (0, 0), (-1, -1), 12), ("BOTTOMPADDING", (0, 0), (-1, -1), 10),
                ("TOPPADDING", (0, 0), (-1, -1), 10)])))
            story.append(Spacer(1, 10))
        elif t.type == "table_open":
            rows, row = [], []
            j = i + 1
            while tokens[j].type != "table_close":
                if tokens[j].type == "tr_open":
                    row = []
                elif tokens[j].type == "inline":
                    row.append(Paragraph(inline(tokens[j].children), cell))
                elif tokens[j].type == "tr_close":
                    rows.append(row)
                j += 1
            widths = [68, 155, 105, 147] if len(rows[0]) == 4 else [235, 240]
            table = Table(rows, colWidths=widths, repeatRows=1, hAlign="LEFT")
            table.setStyle(TableStyle([
                ("BACKGROUND", (0, 0), (-1, 0), colors.HexColor("#dfedf2")),
                ("ROWBACKGROUNDS", (0, 1), (-1, -1), [colors.white, colors.HexColor("#f4f7f9")]),
                ("VALIGN", (0, 0), (-1, -1), "TOP"),
                ("LINEBELOW", (0, 0), (-1, -1), .4, colors.HexColor("#d3e2e9")),
                ("LEFTPADDING", (0, 0), (-1, -1), 8), ("RIGHTPADDING", (0, 0), (-1, -1), 8),
                ("TOPPADDING", (0, 0), (-1, -1), 8), ("BOTTOMPADDING", (0, 0), (-1, -1), 8)]))
            story.extend([table, Spacer(1, 12)])
            i = j
        elif t.type not in ("list_item_open", "list_item_close", "paragraph_close"):
            raise ValueError(f"Unsupported block Markdown: {t.type}")
        i += 1

    def decorate(canvas, doc):
        canvas.setTitle("AlcMute ご利用マニュアル")
        canvas.setAuthor("Alcnaka")
        canvas.setFont("Guide", 8)
        canvas.setFillColor(colors.HexColor("#587080"))
        canvas.drawString(60, 30, f"AlcMute | {version} | Alcnaka")
        canvas.drawRightString(A4[0] - 60, 30, str(doc.page))
        canvas.setStrokeColor(colors.HexColor("#d3e2e9"))
        canvas.line(60, 45, A4[0] - 60, 45)

    SimpleDocTemplate(str(output), pagesize=A4, leftMargin=60, rightMargin=60,
                      topMargin=42, bottomMargin=58).build(story, onFirstPage=decorate, onLaterPages=decorate)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "output")
    fonts = Path(os.environ.get("WINDIR", "C:/Windows")) / "Fonts"
    parser.add_argument("--font", type=Path, default=fonts / "YuGothM.ttc")
    parser.add_argument("--bold-font", type=Path, default=fonts / "YuGothB.ttc")
    args = parser.parse_args()
    if not args.font.is_file() or not args.bold_font.is_file():
        parser.error("Japanese TrueType fonts required; pass --font and --bold-font <path to TTF/TTC>")
    version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    markdown = MarkdownIt("commonmark", {"html": False}).enable("table")
    tokens, headings = prepare_tokens(markdown, SOURCE.read_text(encoding="utf-8"))
    site = args.output / "site"
    pdf_dir = args.output / "pdf"
    for folder in [site / "assets", pdf_dir]:
        folder.mkdir(parents=True, exist_ok=True)
    # PDF sees repository-relative image paths; HTML gets copies alongside it.
    build_pdf(tokens, headings, pdf_dir / "AlcMute-Manual.pdf", version, args.font, args.bold_font)
    for token in tokens:
        for child in token.children or []:
            if child.type == "image":
                src = child.attrGet("src")
                target = site / "assets" / source_image(src).name
                shutil.copy2(source_image(src), target)
                child.attrSet("src", "assets/" + target.name)
    shutil.copy2(ROOT / "assets/app.png", site / "assets/app.png")
    shutil.copy2(pdf_dir / "AlcMute-Manual.pdf", site / "AlcMute-Manual.pdf")
    for name in ["site.css", "site.js"]:
        shutil.copy2(Path(__file__).parent / name, site / name)
    nav = "".join(f'<a href="#{a}">{html.escape(t)}</a>' for a, t in headings)
    template = (Path(__file__).parent / "template.html").read_text(encoding="utf-8")
    result = template.replace("{{version}}", html.escape(version)).replace("{{nav}}", nav).replace("{{content}}", markdown.renderer.render(tokens, markdown.options, {}))
    (site / "index.html").write_text(result, encoding="utf-8")
    (site / ".nojekyll").touch()
    print(f"Web: {site / 'index.html'}\nPDF: {pdf_dir / 'AlcMute-Manual.pdf'}")


if __name__ == "__main__":
    main()
