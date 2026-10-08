"""Extract text objects with boxes in sheet space (PDF user units, origin top left).

Written for corpus/drawings/test_drawing_1.pdf (T0.10). Assumes the simple structure of that file: one global cm, BT/Tm/Tf/Tj/ET per text object,
no rotation, one Type0 font. Box: advance width horizontally, font ascent to descent vertically.

Usage: python3 -I extract_text_boxes.py <file.pdf> > boxes.json   (needs pypdf, tested with 6.14.2)
"""
import json
import sys

import pypdf
from pypdf.generic import ContentStream

r = pypdf.PdfReader(sys.argv[1])
p = r.pages[0]
H = float(p.mediabox.height)
f = p['/Resources']['/Font']['/Type0TTF0'].get_object()
d = f['/DescendantFonts'][0].get_object()
fd = d['/FontDescriptor'].get_object()
asc, desc = float(fd['/Ascent']), float(fd['/Descent'])
dw = float(d['/DW'])
W = {}
w = d['/W']
i = 0
while i < len(w):
    c = int(w[i])
    nxt = w[i + 1]
    if isinstance(nxt, list):
        for k, v in enumerate(nxt):
            W[c + k] = float(v)
        i += 2
    else:
        for k in range(c, int(nxt) + 1):
            W[k] = float(w[i + 2])
        i += 3
cs = ContentStream(p.get_contents(), r)
ctm = None
tm = None
tfs = None
out = []
for ops, op in cs.operations:
    if op == b'cm':
        ctm = [float(x) for x in ops]
    elif op == b'Tm':
        tm = [float(x) for x in ops]
    elif op == b'Tf':
        tfs = float(ops[1])
    elif op == b'Tj':
        b = ops[0].original_bytes
        cids = [b[i] * 256 + b[i + 1] for i in range(0, len(b), 2)]
        text = ''.join(chr(c) for c in cids)
        adv = sum(W.get(c, dw) for c in cids) / 1000.0 * tfs
        a, bb, cc, dd, e, ff = tm
        assert bb == 0 and cc == 0 and a == dd, tm
        s = ctm[0]
        assert ctm[1] == 0 and ctm[2] == 0 and ctm[3] == s and ctm[4] == 0 and ctm[5] == 0
        size = a * tfs * s
        x0 = e * s
        yb = ff * s
        x1 = x0 + adv * a * s
        ytop = yb + asc / 1000 * size
        ybot = yb + desc / 1000 * size
        out.append(dict(text=text, x0=x0, x1=x1, top=H - ytop, bottom=H - ybot,
                        baseline=H - yb, size=size))
json.dump(out, sys.stdout, ensure_ascii=False, indent=0)
