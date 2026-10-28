"""One-off extraction for the supplied Bremen mathematics print form.

Requires PyMuPDF (development only). Coordinates are PDF points from the
TOP LEFT of the unrotated page. Export converts y to PDF bottom-left.
The generated JSON is reviewed and checked in; runtime needs no Python.
"""
import hashlib
import json
import re
from pathlib import Path
import pymupdf

ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / 'entwicklungsuebersicht_mathe_DRUCKVERSION_270925.pdf'
SECTIONS = {
 5: ('Zahl und Operation', 'Mengen- und Zahlvorstellungen', 'Im Zahlenraum orientieren'),
 6: ('Zahl und Operation', 'Mengen- und Zahlvorstellungen', 'Im Zahlenraum orientieren'),
 7: ('Zahl und Operation', 'Rechenoperationen', 'Rechenverfahren anwenden'),
 8: ('Zahl und Operation', 'Rechenoperationen', 'Rechenverfahren anwenden'),
 9: ('Zahl und Operation', 'Rechnen in Kontexten', 'Sachaufgaben lösen'),
 10: ('Größen und Messen', 'Größenvorstellungen', 'Größenvorstellungen besitzen'),
 11: ('Größen und Messen', 'Größen in Kontexten', 'Sachaufgaben mit Größen lösen'),
 12: ('Raum und Form', 'Räumliches Vorstellungsvermögen', 'Räumliches Vorstellungsvermögen besitzen'),
 13: ('Raum und Form', 'Geometrische Figuren', 'Ebene Figuren und Körper erkennen, benennen und darstellen'),
 14: ('Raum und Form', 'Geometrische Abbildungen', 'Symmetrien erkennen, benennen und darstellen'),
 15: ('Daten und Zufall', 'Daten', 'Daten erfassen, darstellen und auswerten'),
 16: ('Daten und Zufall', 'Zufall', 'Wahrscheinlichkeiten einschätzen'),
}

def clean(text):
    return re.sub(r'\s+', ' ', text.replace('\x07', '').replace('\u00ad', '')).strip()

def source_band(page, x, y):
    # The source marks target stages with two gray tones. Read the actual
    # background instead of inferring it from a stage number.
    for drawing in page.get_drawings():
        fill = drawing['fill']
        if (fill and drawing['rect'].width > 400
                and drawing['rect'].contains(pymupdf.Point(x, y))
                and max(fill) - min(fill) < .05):
            return 'light' if sum(fill) / len(fill) > .85 else 'dark'
    return 'white'

result = []
doc = pymupdf.open(SOURCE)
for page_no, (domain, section, subtitle) in SECTIONS.items():
    page = doc[page_no - 1]
    lines = []
    for block in page.get_text('dict')['blocks']:
        for line in block.get('lines', []):
            text = clean(''.join(s['text'] for s in line['spans']))
            if text:
                lines.append((line['bbox'], text, line['spans']))
    # Left table border is split into one vertical segment per level.
    bands = sorted((min(item[1].y,item[2].y)-1, max(item[1].y,item[2].y)+.1)
        for drawing in page.get_drawings() for item in drawing['items']
        if item[0] == 'l' and abs(item[1].x-item[2].x) < .1
        and 44 < item[1].x < 50 and abs(item[1].y-item[2].y) > 2)
    levels = [(bbox[1], text) for bbox,text,_ in lines
              if bbox[0] < 50 and (text == 'BASIS' or re.fullmatch(r'\d{2}',text))]
    text_lines = []
    for block in page.get_text('dict', clip=pymupdf.Rect(90,0,580,800))['blocks']:
        for line in block.get('lines', []):
            t = clean(''.join(s['text'] for s in line['spans']))
            if t:
                text_lines.append((line['bbox'], t, line['spans']))
    starts = sorted([(bbox,text,spans) for bbox,text,spans in text_lines
        if 80 < bbox[0] < 105 and re.match(r'^(kann|kennt|versteht)\b', text)], key=lambda x:x[0][1])
    # Each competence begins with one of the three verbs. Wrapped lines belong
    # to the preceding competence, including lines in a separate text block.
    for idx, (bbox,text,spans) in enumerate(starts):
        band = next(((a,b) for a,b in bands if a <= spans[0]['origin'][1]-3.1 < b), None)
        level_candidates = [(y,t) for y,t in levels if band and band[0]-1 <= y <= band[1]+1]
        if not level_candidates:
            raise ValueError(f'Missing level at page {page_no}: {text}; bands={bands}')
        level = level_candidates[0][1]
        if level == 'BASIS': level = 'Basis'
        next_y = starts[idx+1][0][1] if idx+1 < len(starts) else band[1]
        end_y = min(next_y-1,band[1]-1)
        continuation = sorted([(bb,t) for bb,t,_ in text_lines
            if 80 < bb[0] < 105 and bbox[1]+2 < bb[1] < end_y
            and not re.match(r'^(kann|kennt|versteht)\b',t)], key=lambda x:x[0][1])
        text = clean(' '.join([text]+[t for _,t in continuation]))
        symbols = [t for bb,t,_ in lines if 55 < bb[0] < 80 and abs(bb[1]-bbox[1]) < 2]
        result.append(dict(id=f'mathe-p{page_no:02}-{idx+1:03}', domain=domain,
            section=section, subtitle=subtitle, level=level, label=text,
            band=source_band(page, 105, spans[0]['origin'][1]-3.1),
            symbol=''.join(re.split(r'\b(?:kann|kennt|versteht)\b', s)[0].strip() for s in symbols), page=page_no, x=82.0,
            y=round(spans[0]['origin'][1]-3.1,3), page_height=round(page.rect.height,3)))

payload = dict(template='bremen-mathematik-2025', source=SOURCE.name,
    sha256=hashlib.sha256(SOURCE.read_bytes()).hexdigest(), coordinate_system='points-top-left',
    criteria=result)
(ROOT/'backend'/'criteria.json').write_text(json.dumps(payload,ensure_ascii=False,indent=2)+'\n')
print(f'{len(result)} criteria extracted')
for p in SECTIONS:
    items=[r for r in result if r['page']==p]
    print(p, len(items), sorted(set(r['level'] for r in items)))
