"""Generate class presentation PPTX — revised v3."""
import re, os
from pptx import Presentation
from pptx.util import Inches, Pt
from pptx.dml.color import RGBColor
from pptx.enum.text import PP_ALIGN

BASE = os.path.dirname(os.path.abspath(__file__))
CKPT = os.path.join(BASE, "checkpoints")

# ── Palette ───────────────────────────────────────────────────────────────────
BG      = RGBColor(0x0F, 0x17, 0x2A)
PANEL   = RGBColor(0x16, 0x24, 0x3E)
ACCENT  = RGBColor(0x4A, 0x9E, 0xFF)
ACCENT2 = RGBColor(0x56, 0xD3, 0xA0)
WHITE   = RGBColor(0xFF, 0xFF, 0xFF)
LGRAY   = RGBColor(0xCC, 0xD6, 0xE8)
YELLOW  = RGBColor(0xFF, 0xD7, 0x5E)
RED     = RGBColor(0xFF, 0x6B, 0x6B)
ORANGE  = RGBColor(0xFF, 0xA0, 0x40)
PURPLE  = RGBColor(0xA0, 0x6F, 0xFF)

prs = Presentation()
prs.slide_width  = Inches(13.33)
prs.slide_height = Inches(7.5)
BLANK = prs.slide_layouts[6]

_NUM = re.compile(r'([+\-]?\d[\d,\.]*(?:%|×|ns|ms)?)')

def lerp_rgb(c1, c2, t):
    return RGBColor(*(int(a + t*(b-a)) for a, b in zip(c1, c2)))

# ── Primitives ────────────────────────────────────────────────────────────────
def bg(slide, color=BG):
    fill = slide.background.fill; fill.solid(); fill.fore_color.rgb = color

def rect(slide, l, t, w, h, fill_color, line_color=None):
    s = slide.shapes.add_shape(1, Inches(l), Inches(t), Inches(w), Inches(h))
    s.fill.solid(); s.fill.fore_color.rgb = fill_color
    if line_color: s.line.color.rgb = line_color
    else:          s.line.fill.background()
    return s

def box(slide, l, t, w, h, text, size=18, bold=False, color=WHITE,
        align=PP_ALIGN.LEFT, italic=False):
    tf = slide.shapes.add_textbox(Inches(l), Inches(t), Inches(w), Inches(h))
    tf.word_wrap = True
    p = tf.text_frame.paragraphs[0]; p.alignment = align
    r = p.add_run(); r.text = text
    r.font.size = Pt(size); r.font.bold = bold
    r.font.italic = italic; r.font.color.rgb = color
    return tf

def img(slide, path, l, t, w, h=None):
    if not os.path.exists(path): return
    kw = dict(left=Inches(l), top=Inches(t), width=Inches(w))
    if h: kw['height'] = Inches(h)
    slide.shapes.add_picture(path, **kw)

def hline(slide, y, color=ACCENT, thickness=Pt(1.5)):
    c = slide.shapes.add_connector(1, Inches(0.4), Inches(y), Inches(12.9), Inches(y))
    c.line.color.rgb = color; c.line.width = thickness

# ── Rich text (numbers → YELLOW) ─────────────────────────────────────────────
def _rich_runs(para, text, base, num_color=YELLOW, size=16, bold=False, italic=False):
    for part in _NUM.split(text):
        if not part: continue
        r = para.add_run(); r.text = part
        r.font.size = Pt(size); r.font.bold = bold; r.font.italic = italic
        r.font.color.rgb = num_color if _NUM.fullmatch(part) else base

def rich_box(slide, l, t, w, h, text, size=18, bold=False,
             base=WHITE, num_color=YELLOW, align=PP_ALIGN.LEFT):
    tf = slide.shapes.add_textbox(Inches(l), Inches(t), Inches(w), Inches(h))
    tf.word_wrap = True
    p = tf.text_frame.paragraphs[0]; p.alignment = align
    _rich_runs(p, text, base=base, num_color=num_color, size=size, bold=bold)

def bullet_box(slide, l, t, w, h, items, size=16, color=LGRAY, rich=False, num_color=YELLOW):
    tf = slide.shapes.add_textbox(Inches(l), Inches(t), Inches(w), Inches(h))
    tf.word_wrap = True; tf.text_frame.auto_size = None
    first = True
    for item in items:
        level, text = item if isinstance(item, tuple) else (0, item)
        para = tf.text_frame.paragraphs[0] if first else tf.text_frame.add_paragraph()
        first = False; para.level = level; para.space_after = Pt(3)
        bullet = "▸ " if level == 0 else "  • "
        if rich:
            r0 = para.add_run(); r0.text = bullet
            r0.font.size = Pt(size); r0.font.color.rgb = color
            _rich_runs(para, text, base=color, num_color=num_color, size=size)
        else:
            r = para.add_run(); r.text = bullet + text
            r.font.size = Pt(size); r.font.color.rgb = color

# ── Slide templates ───────────────────────────────────────────────────────────
def section_header(slide, title, subtitle=None):
    bg(slide); rect(slide, 0, 0, 0.07, 7.5, ACCENT)
    box(slide, 0.5, 2.5, 12, 1.2, title, size=40, bold=True, color=WHITE, align=PP_ALIGN.CENTER)
    if subtitle:
        box(slide, 0.5, 3.9, 12, 0.8, subtitle, size=20, color=LGRAY, align=PP_ALIGN.CENTER)

def content_slide(slide, title, body_fn):
    bg(slide)
    rect(slide, 0, 0, 13.33, 1.05, PANEL)
    rect(slide, 0, 1.02, 13.33, 0.05, ACCENT)
    box(slide, 0.45, 0.18, 12.4, 0.75, title, size=26, bold=True, color=WHITE)
    body_fn(slide)

# ── Diagram node ──────────────────────────────────────────────────────────────
def node(slide, l, t, w, h, label, sub="", fill=ACCENT, tc=BG, lsize=13, ssize=10):
    rect(slide, l, t, w, h, fill)
    if sub:
        box(slide, l+0.06, t+0.05, w-0.12, h*0.52,
            label, size=lsize, bold=True, color=tc, align=PP_ALIGN.CENTER)
        box(slide, l+0.06, t+h*0.54, w-0.12, h*0.43,
            sub,   size=ssize, color=tc, align=PP_ALIGN.CENTER, italic=True)
    else:
        box(slide, l+0.06, t+0.06, w-0.12, h-0.12,
            label, size=lsize, bold=True, color=tc, align=PP_ALIGN.CENTER)


# ═══════════════════════════════════════════════════════════════════════════════
# 1 — Title
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK); bg(sl)
rect(sl, 0, 0, 0.10, 7.5, ACCENT)
box(sl, 0.6, 0.8,  12.1, 0.7,
    "Department of Modeling & Simulation  ·  Old Dominion University",
    size=14, color=LGRAY)
box(sl, 0.6, 1.55, 12.1, 1.7,
    "Learning LLVM Pass Sequences\nvia Reinforcement Learning",
    size=38, bold=True, color=WHITE)
box(sl, 0.6, 3.2,  12.1, 0.55, "with Autoregressive Policies",
    size=24, color=ACCENT2)
box(sl, 0.6, 4.0,  6,    0.45, "Evan Black", size=18, bold=True, color=WHITE)
box(sl, 0.6, 4.5,  6,    0.4,  "eblac013@odu.edu", size=15, color=LGRAY)
box(sl, 0.6, 6.8,  12.1, 0.4,
    "Reinforcement Learning  ·  LLVM  ·  Pass Ordering  ·  PPO  ·  Transformer",
    size=13, color=ACCENT, italic=True)


# ═══════════════════════════════════════════════════════════════════════════════
# 2 — Motivation
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK)
def _s2(sl):
    bullet_box(sl, 0.45, 1.25, 7.0, 5.6, [
        (0, "LLVM exposes hundreds of individual optimization passes"),
        (1, "inlining, vectorization, loop unrolling, dead-code elim, …"),
        (0, "Order matters: inline before const-prop → more constants to fold"),
        (0, "-O3 uses a fixed pipeline tuned for the average program"),
        (1, "Suboptimal for any specific program"),
        (0, "Finding the best pipeline is NP-hard (phase-ordering problem)"),
        (0, "ML offers a principled alternative:"),
        (1, "Map IR features → pass choices"),
        (1, "Train on real execution-time feedback"),
        (1, "Generalise to unseen programs"),
    ], size=17, rich=True)
    rect(sl, 7.7, 1.3, 5.2, 5.5, PANEL)
    box(sl, 7.9, 1.5, 4.9, 0.45, "The Goal", size=17, bold=True, color=ACCENT2)
    box(sl, 7.9, 2.0, 4.9, 1.2,
        "Beat -O3 on individual programs by learning a per-program pass sequence.",
        size=16, color=WHITE)
    box(sl, 7.9, 3.4, 4.9, 0.45, "Prior Work", size=17, bold=True, color=ACCENT2)
    bullet_box(sl, 7.9, 3.9, 4.9, 2.6, [
        (0, "AutoPhase — LSTM + PPO, targets IR size"),
        (0, "CompilerGym — standard RL env for LLVM"),
        (0, "MLGO — learned inlining at Google scale"),
        (0, "MILEPOST — nearest-neighbour on static features"),
        (1, "Our focus: wall-clock speedup, offline corpus + autoregressive policy"),
    ], size=14, color=LGRAY)
content_slide(sl, "Motivation & Problem", _s2)


# ═══════════════════════════════════════════════════════════════════════════════
# 3 — MDP Formulation
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK)
def _s3(sl):
    # State card
    rect(sl, 0.35, 1.2, 3.95, 5.9, PANEL)
    rect(sl, 0.35, 1.2, 3.95, 0.52, ACCENT)
    box(sl, 0.45, 1.25, 3.75, 0.42, "State  sₜ", size=19, bold=True, color=BG)
    box(sl, 0.45, 1.82, 3.75, 0.38, "48-dim IR delta vector  fₜ ∈ ℝ⁴⁸",
        size=13, bold=True, color=YELLOW)
    box(sl, 0.45, 2.24, 3.75, 0.32, "3 chunk-pairs × 16 dims each:",
        size=12, color=LGRAY, italic=True)
    box(sl, 0.45, 2.62, 3.75, 0.3, "Opcode categories (12):",
        size=12, bold=True, color=ACCENT2)
    for i, c in enumerate(["memory · int-arith · float-arith",
                            "bitwise · compare · cast",
                            "call · control · phi/select",
                            "vector · block-boundary · other"]):
        box(sl, 0.55, 2.96+i*0.31, 3.6, 0.3, c, size=11, color=LGRAY, italic=True)
    box(sl, 0.45, 4.23, 3.75, 0.3, "Metadata rates (4):",
        size=12, bold=True, color=ACCENT2)
    box(sl, 0.55, 4.55, 3.6, 0.3, "tbaa · llvm.loop · alias.scope · noalias",
        size=11, color=LGRAY, italic=True)
    box(sl, 0.45, 4.95, 3.75, 0.4, "Δ = chunk(c+1) − chunk(c)  for c ∈ {0,1,2}",
        size=11, color=WHITE)
    box(sl, 0.45, 5.44, 3.75, 0.55,
        "Invariant to function size;\nsensitive to loop-rotate, licm, sroa structural shifts",
        size=11, color=LGRAY)

    # Action card
    rect(sl, 4.5, 1.2, 4.2, 5.9, PANEL)
    rect(sl, 4.5, 1.2, 4.2, 0.52, ACCENT2)
    box(sl, 4.6, 1.25, 4.0, 0.42, "Action  aₜ", size=19, bold=True, color=BG)
    box(sl, 4.6, 1.82, 4.0, 0.38, "29 total:  28 passes  +  1 Stop",
        size=13, bold=True, color=YELLOW)
    box(sl, 4.6, 2.26, 4.0, 0.3, "Example passes:",
        size=12, bold=True, color=ACCENT)
    for i, (name, desc) in enumerate([
        ("inline",       "removes call overhead; exposes callee"),
        ("sroa",         "scalar replace aggregates → registers"),
        ("mem2reg",      "alloca → SSA (prerequisite for most)"),
        ("instcombine",  "fold/simplify instruction patterns"),
        ("simplifycfg",  "clean dead blocks, merge branches"),
        ("loop-rotate",  "canonicalize loops for LICM"),
        ("licm",         "hoist loop-invariant code out"),
        ("loop-unroll",  "replicate loop body N times"),
        ("gvn",          "global value numbering / CSE"),
        ("Stop",         "terminate early → compile binary"),
    ]):
        y = 2.62 + i*0.31
        box(sl, 4.6,  y, 1.25, 0.3, name, size=11, bold=True, color=ACCENT2)
        box(sl, 5.88, y, 2.72, 0.3, desc, size=11, color=LGRAY, italic=True)

    # Reward card
    rect(sl, 8.9, 1.2, 4.08, 5.9, PANEL)
    rect(sl, 8.9, 1.2, 4.08, 0.52, YELLOW)
    box(sl, 9.0, 1.25, 3.88, 0.42, "Reward  r", size=19, bold=True, color=BG)
    box(sl, 9.0, 1.82, 3.88, 0.38, "Terminal speedup vs -O3",
        size=13, bold=True, color=WHITE)
    box(sl, 9.0, 2.28, 3.88, 0.42, "r = (t_O3 − t_policy) / t_O3",
        size=14, bold=True, color=YELLOW)
    box(sl, 9.0, 2.76, 3.88, 0.32, "Positive = faster than -O3",
        size=12, color=ACCENT2, italic=True)
    rect(sl, 9.0, 3.18, 3.78, 3.62, RGBColor(0x0A, 0x10, 0x1C))
    box(sl, 9.1, 3.25, 3.6, 0.35, "Concrete example — fft:",
        size=12, bold=True, color=ACCENT)
    box(sl, 9.1, 3.65, 3.6, 0.34, "t_O3  = 46,509 ns", size=13, color=LGRAY)
    box(sl, 9.1, 4.02, 3.6, 0.34, "t_policy = 41,852 ns", size=13, color=LGRAY)
    rect(sl, 9.1, 4.43, 3.58, 0.03, LGRAY)
    box(sl, 9.1, 4.52, 3.6, 0.38, "r = (46,509 − 41,852) / 46,509", size=12, color=WHITE)
    box(sl, 9.1, 4.94, 3.6, 0.40, "  = 4,657 / 46,509", size=12, color=WHITE)
    box(sl, 9.1, 5.39, 3.6, 0.45, "  = +10.0%  speedup",
        size=16, bold=True, color=ACCENT2)
    box(sl, 9.0, 6.5, 3.88, 0.4,
        "Trimmed mean of 201 timed runs\n(10% trim, 5 warm-up, CLOCK_MONOTONIC)",
        size=10, color=LGRAY, italic=True)
content_slide(sl, "MDP Formulation", _s3)


# ═══════════════════════════════════════════════════════════════════════════════
# 4 — Section: Dataset & EDA
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK)
section_header(sl, "Dataset & Exploratory Data Analysis",
               "950k random sequences · 38 benchmarks · bench-cache")


# ═══════════════════════════════════════════════════════════════════════════════
# 5 — Dataset Landscape (bench-cache overview)
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK)
def _s5(sl):
    # Stats panel (left)
    rect(sl, 0.35, 1.2, 4.3, 5.95, PANEL)
    box(sl, 0.45, 1.27, 4.1, 0.42, "Bench-Cache Statistics",
        size=16, bold=True, color=ACCENT)
    stats = [
        ("Total sequences",    "950,000"),
        ("Benchmarks in pool", "38"),
        ("Training benchmarks","6"),
        ("Sequences / fn",     "25,000"),
        ("Sequence lengths",   "1 – 20"),
        ("Pass menu size",     "28 + Stop"),
        ("Baselines per fn",   "-O0, -O2, -O3"),
        ("Cache format",       "on-disk key-value"),
    ]
    for i, (k, v) in enumerate(stats):
        y = 1.78 + i*0.62
        rect(sl, 0.45, y, 4.1, 0.58, BG if i%2==0 else RGBColor(0x12, 0x1D, 0x35))
        box(sl, 0.55, y+0.09, 2.1, 0.4, k, size=12, color=LGRAY)
        box(sl, 2.65, y+0.09, 1.8, 0.4, v, size=12, bold=True, color=YELLOW)

    box(sl, 0.45, 6.75, 4.1, 0.35,
        "★ placeholder stats — update once dataset.png regenerated",
        size=10, color=RGBColor(0x66,0x66,0x88), italic=True)

    # Dataset figure
    ds_img = os.path.join(BASE, "dataset.png")
    if os.path.exists(ds_img):
        img(sl, ds_img, 4.8, 1.2, 8.2)
    else:
        rect(sl, 4.8, 1.2, 8.2, 5.95, RGBColor(0x0A, 0x10, 0x1C))
        box(sl, 4.8, 3.5, 8.2, 0.8,
            "dataset.png  (regenerate with:\ncargo run -- plot-dataset --data dataset.jsonl)",
            size=13, color=RGBColor(0x55,0x55,0x77), align=PP_ALIGN.CENTER, italic=True)
content_slide(sl, "Training Landscape — Bench-Cache Overview", _s5)


# ═══════════════════════════════════════════════════════════════════════════════
# 6 — Benchmark Pool  (heat-coloured table)
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK)
def _s6(sl):
    bullet_box(sl, 0.45, 1.25, 5.55, 2.05, [
        (0, "Pool: 38 self-contained C benchmarks"),
        (1, "Sorting, numerical, data structures, codecs, recursive"),
        (0, "25,000 random sequences per benchmark → ~950k entries"),
        (0, "3 baselines (-O0, -O2, -O3) recorded per benchmark"),
    ], size=16, rich=True)
    box(sl, 0.45, 3.38, 5.55, 0.42, "6 Training Benchmarks — selection criteria:",
        size=15, bold=True, color=ACCENT2)
    bullet_box(sl, 0.45, 3.86, 5.55, 2.5, [
        (0, "At least one random sequence beats -O3"),
        (0, "High CV across sequences (rugged reward landscape)"),
        (0, "Diverse computational profiles"),
    ], size=15, rich=True)

    spd_vals = [27.2, 24.3, 12.7, 10.0, 4.7, 9.2]
    cv_vals  = [13.9, 42.7, 46.0, 35.5, 36.6, 18.4]
    cold_g = (0x66, 0xDD, 0x88); hot_g = (0x00, 0xFF, 0x66)
    cold_o = (0xCC, 0xAA, 0x44); hot_o = (0xFF, 0xD7, 0x00)

    rows = [
        ("Benchmark",      "O3 (ns)",   "Best (ns)", "vs O3",   "CV"),
        ("interpreter",    "37,918",    "27,591",    "−27.2%",  "13.9%"),
        ("kmp_search",     "886,408",   "671,195",   "−24.3%",  "42.7%"),
        ("polynomial_eval","417,235",   "364,235",   "−12.7%",  "46.0%"),
        ("fft",            "46,509",    "41,852",    "−10.0%",  "35.5%"),
        ("array_reduction","97,184",    "92,619",    " −4.7%",  "36.6%"),
        ("binary_tree",    "117,990",   "107,087",   " −9.2%",  "18.4%"),
    ]
    col_x = [6.3, 7.9, 9.3, 10.7, 11.9]
    col_w = [1.55, 1.35, 1.35, 1.15, 1.1]
    for ri, row in enumerate(rows):
        y = 1.25 + ri*0.83
        if ri == 0:
            rect(sl, 6.25, y, 6.75, 0.8, ACCENT)
            for cell, cx, cw in zip(row, col_x, col_w):
                box(sl, cx, y+0.12, cw, 0.56, cell, size=13, bold=True,
                    color=BG, align=PP_ALIGN.CENTER)
        else:
            di = ri - 1
            rect(sl, 6.25, y, 6.75, 0.8, PANEL if ri%2==1 else BG)
            st = (spd_vals[di]-4.7)/(27.2-4.7)
            ct = (cv_vals[di]-13.9)/(46.0-13.9)
            sc = lerp_rgb(cold_g, hot_g, st)
            cc = lerp_rgb(cold_o, hot_o, ct)
            for ci, (cell, cx, cw) in enumerate(zip(row, col_x, col_w)):
                fc = WHITE if ci==0 else (sc if ci==3 else (cc if ci==4 else LGRAY))
                box(sl, cx, y+0.12, cw, 0.56, cell, size=13,
                    bold=(ci in (3,4)), color=fc, align=PP_ALIGN.CENTER)
    box(sl, 6.25, 6.55, 3.3, 0.4,
        "Green = speedup magnitude", size=12, color=ACCENT2, italic=True)
    box(sl, 9.6,  6.55, 3.4, 0.4,
        "Yellow = landscape ruggedness (CV)", size=12, color=YELLOW, italic=True)
content_slide(sl, "Benchmark Pool & Training Benchmark Selection", _s6)


# ═══════════════════════════════════════════════════════════════════════════════
# 7 — IR Features
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK)
def _s7(sl):
    bullet_box(sl, 0.45, 1.25, 5.8, 3.5, [
        (0, "Policy observes a 48-dim IR feature vector fₜ"),
        (0, "Split IR into C=4 equal positional chunks"),
        (0, "Per chunk: opcode histogram (12 categories) + metadata rates (4 kinds)"),
        (1, "categories: memory, int-arith, float-arith, bitwise, compare, cast,"),
        (1, "call, control, phi/select, vector, block-boundary, other"),
        (1, "metadata: tbaa, llvm.loop, alias.scope, noalias"),
        (0, "3 adjacent-chunk deltas → 3 × 16 = 48-dim vector"),
        (0, "Key properties:"),
        (1, "Invariant to function size"),
        (1, "Responds to loop-rotate, licm, sroa — invisible to global counts"),
    ], size=15, rich=True)
    feat_op   = os.path.join(BASE, "features_op.png")
    feat_meta = os.path.join(BASE, "features_meta.png")
    if os.path.exists(feat_op):   img(sl, feat_op,   6.3, 1.25, 6.7)
    if os.path.exists(feat_meta): img(sl, feat_meta, 6.3, 4.3,  6.7)
content_slide(sl, "IR Feature Representation", _s7)


# ═══════════════════════════════════════════════════════════════════════════════
# 8 — Pass Menu & EDA  (fixed layout)
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK)
def _s8(sl):
    bullet_box(sl, 0.45, 1.25, 5.9, 2.2, [
        (0, "28-pass primary menu + Stop token (29 total)"),
        (0, "Inclusion criteria:"),
        (1, "Appears in -O3 inner loop or is a prerequisite"),
        (1, "Demonstrable impact on C compute code"),
        (1, "Covers a distinct optimization category"),
        (0, "LLVM 20 nesting handled transparently:"),
        (1, "inline → cgscc(inline)"),
        (1, "licm → loop-mssa(licm)  ·  loop-rotate → loop(loop-rotate)"),
    ], size=15, rich=True)

    box(sl, 0.45, 3.55, 5.9, 0.4, "Top passes by EDA analysis:",
        size=15, bold=True, color=ACCENT2)
    rows2 = [
        ("Pass",         "Top-Decile Enrich.", "Geo-Mean Speedup"),
        ("inline",       "2.35×",              "1.26×"),
        ("simplifycfg",  "1.98×",              "1.12×"),
        ("sroa",         "1.87×",              "1.09×"),
        ("mem2reg",      "1.76×",              "1.08×"),
    ]
    for ri, row in enumerate(rows2):
        y = 4.02 + ri*0.53
        bg_c = ACCENT if ri==0 else (PANEL if ri%2 else BG)
        rect(sl, 0.45, y, 5.9, 0.5, bg_c)
        for cell, cx, cw in zip(row, [0.5, 2.05, 3.9], [1.5, 1.8, 2.0]):
            fc = BG if ri==0 else (YELLOW if cx > 2 else WHITE)
            box(sl, cx, y+0.05, cw, 0.4, cell, size=13, bold=(ri==0), color=fc)

    bullet_box(sl, 6.5, 1.25, 6.5, 5.7, [
        (0, "inline is the top pass-enabler:"),
        (1, "Removing call overhead exposes callee bodies to const-prop, DCE, loop transforms"),
        (1, "Enrichment 2.35× = appears in top-decile at 2.35× baseline rate"),
        (0, "simplifycfg + sroa + mem2reg unlock downstream analyses"),
        (1, "SSA promotion (mem2reg) required before most scalar optimizations"),
        (0, "Sequence length distribution ~uniform across 1–20"),
        (0, "Best-found sequence median length: 17–18"),
        (1, "No spike at the 20-pass cap → K=20 horizon is sufficient"),
        (1, "Stop token viable as early-termination mechanism"),
        (0, "Bench-cache lookup rate grows during training:"),
        (1, "Policy re-generates previously benchmarked sequences → instant reward"),
    ], size=15, rich=True)
content_slide(sl, "Pass Menu & EDA Highlights", _s8)


# ═══════════════════════════════════════════════════════════════════════════════
# 9 — Section: Architecture & Training
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK)
section_header(sl, "Policy Architecture & Training",
               "Auto-TFX  ·  Auto-GRU  ·  PPO")


# ═══════════════════════════════════════════════════════════════════════════════
# 10 — Architecture Diagrams  (fixed: larger boxes, cleaner labels)
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK)
def _s10(sl):
    # ── Auto-TFX (left 6.1") ─────────────────────────────────────────────────
    rect(sl, 0.3, 1.2, 6.1, 0.48, ACCENT)
    box(sl, 0.35, 1.24, 6.0, 0.38, "Auto-TFX  (Causal Transformer)",
        size=16, bold=True, color=BG, align=PP_ALIGN.CENTER)

    # Token sequence row — 5 boxes, each 1.1 wide, 0.85 tall
    tok_data = [
        ("W·fₜ",    "IR proj\n48→128",      ACCENT),
        ("E[a₀]",   "action\nembed",         RGBColor(0x3A,0x8E,0xEF)),
        ("E[a₁]",   "action\nembed",         RGBColor(0x2A,0x7E,0xDF)),
        ("…",       "",                       PANEL),
        ("E[aₜ₋₁]", "action\nembed",         RGBColor(0x1A,0x6E,0xCF)),
    ]
    for i, (lbl, sub, fill) in enumerate(tok_data):
        node(sl, 0.32 + i*1.16, 1.82, 1.08, 0.82,
             lbl, sub, fill=fill, tc=WHITE if fill != PANEL else LGRAY,
             lsize=13, ssize=10)

    box(sl, 1.8, 2.7, 2.5, 0.3, "↓  full attention (no mask)  ↓",
        size=11, color=LGRAY, align=PP_ALIGN.CENTER)

    node(sl, 0.32, 3.04, 5.88, 0.78,
         "Transformer Encoder",
         "2 layers · 4 heads · d=128 · feed-forward 256 · dropout 0.1",
         fill=RGBColor(0x1E,0x40,0x7A), tc=WHITE, lsize=14, ssize=11)

    box(sl, 0.7,  3.88, 2.0, 0.28, "↓  token 0", size=11, color=LGRAY, align=PP_ALIGN.CENTER)
    box(sl, 3.6,  3.88, 2.0, 0.28, "↓  token t", size=11, color=LGRAY, align=PP_ALIGN.CENTER)

    node(sl, 0.32, 4.2, 2.5, 0.72, "Value Head", "reads token 0\n→ V(sₜ)",
         fill=ACCENT2, tc=BG, lsize=13, ssize=11)
    node(sl, 3.7,  4.2, 2.5, 0.72, "Policy Head", "reads token t\n→ logits (29)",
         fill=ORANGE, tc=BG, lsize=13, ssize=11)

    rect(sl, 0.32, 5.06, 5.88, 0.82, RGBColor(0x0A,0x10,0x1C))
    bullet_box(sl, 0.4, 5.1, 5.72, 0.76, [
        (0, "No causal mask — decodes 1 step at a time, full sequence always visible"),
        (0, "Batched replay: K forward passes (one per step position) instead of N×K"),
    ], size=11, color=LGRAY)

    # ── Auto-GRU (right 6.4") ─────────────────────────────────────────────────
    rect(sl, 6.9, 1.2, 6.1, 0.48, ACCENT2)
    box(sl, 6.95, 1.24, 6.0, 0.38, "Auto-GRU  (Recurrent)",
        size=16, bold=True, color=BG, align=PP_ALIGN.CENTER)

    # Init row
    box(sl, 6.92, 1.82, 6.0, 0.28,
        "Step 0 — seed hidden state from initial IR:", size=12, bold=True, color=ACCENT)
    node(sl, 6.92, 2.14, 1.2, 0.72, "f₀", "48-dim\nIR feat",
         fill=ACCENT, tc=BG, lsize=14, ssize=10)
    box(sl, 8.18, 2.38, 0.38, 0.28, "→", size=15, color=LGRAY, align=PP_ALIGN.CENTER)
    node(sl, 8.6,  2.14, 1.55, 0.72, "Linear\nproject", "",
         fill=RGBColor(0x1E,0x40,0x7A), tc=WHITE, lsize=13)
    box(sl, 10.2, 2.38, 0.38, 0.28, "→", size=15, color=LGRAY, align=PP_ALIGN.CENTER)
    node(sl, 10.63, 2.14, 2.1, 0.72, "h₀", "initial hidden state",
         fill=ACCENT2, tc=BG, lsize=15, ssize=11)

    # Recurrent step row
    box(sl, 6.92, 3.02, 6.0, 0.28,
        "Step t — recurrent update:", size=12, bold=True, color=ACCENT)
    node(sl, 6.92, 3.34, 1.2, 0.65, "fₜ",   "IR feat", fill=ACCENT, tc=BG, lsize=14, ssize=10)
    node(sl, 6.92, 4.04, 1.2, 0.65, "aₜ₋₁", "action",  fill=RGBColor(0x3A,0x8E,0xEF), tc=BG, lsize=14, ssize=10)
    box(sl, 8.18, 3.76, 0.6, 0.28, "concat\n  ↓", size=10, color=LGRAY, align=PP_ALIGN.CENTER)
    node(sl, 8.84, 3.44, 1.55, 0.65, "GRU Cell", "",
         fill=RGBColor(0x1E,0x40,0x7A), tc=WHITE, lsize=13)
    box(sl, 10.44, 3.64, 0.38, 0.28, "→", size=15, color=LGRAY, align=PP_ALIGN.CENTER)
    node(sl, 10.87, 3.44, 1.87, 0.65, "hₜ", "hidden state",
         fill=ACCENT2, tc=BG, lsize=15, ssize=11)
    box(sl, 8.6, 4.18, 1.7, 0.26, "↑ hₜ₋₁  (recurrent)", size=10,
        color=ORANGE, align=PP_ALIGN.CENTER, italic=True)

    # Output heads
    box(sl, 6.92, 4.55, 2.5, 0.28, "↓  hₜ  →  output heads:", size=12, color=LGRAY)
    node(sl, 6.92, 4.87, 2.5, 0.72, "Value Head", "→ V(sₜ)",
         fill=ACCENT2, tc=BG, lsize=13, ssize=12)
    node(sl, 9.67, 4.87, 2.5, 0.72, "Policy Head", "→ logits (29)",
         fill=ORANGE, tc=BG, lsize=13, ssize=12)

    rect(sl, 6.92, 5.73, 6.08, 0.65, RGBColor(0x0A,0x10,0x1C))
    bullet_box(sl, 7.0, 5.76, 5.92, 0.60, [
        (0, "Recency bias: recent IR state has more influence — free inductive prior"),
        (0, "Compresses full episode history into a fixed-width hidden vector"),
    ], size=11, color=LGRAY)

    box(sl, 0.3, 6.52, 12.73, 0.38,
        "Both architectures re-encode current IR features fₜ at every step — neither operates on a stale snapshot.",
        size=12, color=LGRAY, italic=True, align=PP_ALIGN.CENTER)
content_slide(sl, "Policy Architecture Diagrams", _s10)


# ═══════════════════════════════════════════════════════════════════════════════
# 11 — Pass Selection Walkthrough  (fixed: header moved down, steps start lower)
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK)
def _s11_walk(sl):
    # Context banner — taller, clearly visible
    rect(sl, 0.35, 1.2, 12.6, 0.7, PANEL)
    rect(sl, 0.35, 1.2, 12.6, 0.07, YELLOW)
    box(sl, 0.45, 1.32, 12.4, 0.46,
        "Example: interpreter  ·  -O3 baseline = 37,918 ns  ·  "
        "Greedy policy result: ~27,138 ns  →  +27.7% speedup",
        size=15, bold=True, color=YELLOW, align=PP_ALIGN.CENTER)

    # 6 step columns — start at y=2.05 to give clearance
    steps = [
        ("Step 0",    "fₜ: mem↑\ncall↑\ncontrol↑",    "inline",         "remove call\noverhead",           ACCENT),
        ("Step 1",    "fₜ: mem↑\ncfg↑\nint-arith↓",   "simplifycfg",    "clean dead\nblocks/branches",     RGBColor(0x3A,0x8E,0xEF)),
        ("Step 2",    "fₜ: mem↓↓\nint-arith↑\nSSA↑",  "sroa",           "stack→regs\n(mem↓ heavy)",         RGBColor(0x2A,0xAE,0x7F)),
        ("Step 3",    "fₜ: mem≈0\nconsts\nvisible",    "mem2reg",        "alloca→SSA\npromotes vals",        RGBColor(0x5A,0x9E,0x4F)),
        ("Step 4",    "fₜ: stable\nΔ small\nper step", "instcombine",    "fold/simplify\npatterns",          RGBColor(0x9A,0x7E,0x2F)),
        ("5–19",      "fₜ: minor\nΔ per step\n(safe)", "instcombine\n×15","safe: rarely\nhurts, occ. wins", RGBColor(0x55,0x55,0x77)),
    ]
    bw = 1.95; gap = 0.12; sx = 0.35; ty = 2.05
    for i, (step, feat, action, effect, col) in enumerate(steps):
        x = sx + i*(bw+gap)
        # Step label
        rect(sl, x, ty, bw, 0.38, col)
        box(sl, x+0.05, ty+0.05, bw-0.1, 0.28, step, size=12, bold=True,
            color=BG, align=PP_ALIGN.CENTER)
        # IR snapshot
        rect(sl, x, ty+0.41, bw, 0.78, RGBColor(0x0A,0x10,0x1C))
        box(sl, x+0.05, ty+0.46, bw-0.1, 0.68, feat, size=10, color=LGRAY,
            align=PP_ALIGN.CENTER, italic=True)
        # Arrow + action label
        box(sl, x+0.3, ty+1.24, bw-0.6, 0.26, "↓ picks", size=10,
            color=LGRAY, align=PP_ALIGN.CENTER)
        rect(sl, x, ty+1.54, bw, 0.52, col)
        box(sl, x+0.04, ty+1.59, bw-0.08, 0.42, action, size=11, bold=True,
            color=BG, align=PP_ALIGN.CENTER)
        # Effect
        rect(sl, x, ty+2.1, bw, 0.7, PANEL)
        box(sl, x+0.04, ty+2.16, bw-0.08, 0.58, effect, size=10, color=WHITE,
            align=PP_ALIGN.CENTER, italic=True)
        # Connector arrow
        if i < len(steps)-1:
            box(sl, x+bw+0.01, ty+1.7, gap+0.09, 0.3, "→", size=14,
                color=LGRAY, align=PP_ALIGN.CENTER)

    # Result banner
    rect(sl, 0.35, ty+2.93, 12.6, 0.57, RGBColor(0x0A,0x22,0x14))
    rect(sl, 0.35, ty+2.93, 12.6, 0.06, ACCENT2)
    box(sl, 0.45, ty+3.02, 12.4, 0.4,
        "Compile → benchmark  ·  measured = 27,138 ns  ·  "
        "r = (37,918 − 27,138) / 37,918 = +27.7%  ✓",
        size=14, bold=True, color=ACCENT2, align=PP_ALIGN.CENTER)

    # Annotation row
    rect(sl, 0.35, ty+3.62, 12.6, 1.15, PANEL)
    box(sl, 0.45, ty+3.68, 5.9, 0.36,
        "Why repeat instcombine?", size=13, bold=True, color=YELLOW)
    bullet_box(sl, 0.45, ty+4.08, 5.85, 0.65, [
        (0, "Q(Stop)=current speedup; Q(pass)=E[future] — uncertain, possibly higher"),
        (0, "Penalty p=0.025 per no-op is tiny vs ~28% gain — rational to continue"),
    ], size=11, color=LGRAY, rich=True)
    box(sl, 6.6, ty+3.68, 6.2, 0.36,
        "Why is inline so dominant?", size=13, bold=True, color=ACCENT2)
    bullet_box(sl, 6.6, ty+4.08, 6.15, 0.65, [
        (0, "interpreter has a tight bytecode dispatch loop — call overhead dominates"),
        (0, "After inline: callee body is visible → const-prop + DCE fire hard"),
    ], size=11, color=LGRAY)
content_slide(sl, "Concrete Pass-Selection Walkthrough", _s11_walk)


# ═══════════════════════════════════════════════════════════════════════════════
# 12 — PPO Training  (expanded formula with 3 coloured panels)
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK)
def _s12(sl):
    # Left: loop overview
    bullet_box(sl, 0.45, 1.25, 5.65, 2.95, [
        (0, "On-policy collect → update loop"),
        (0, "Collect 256 episodes per epoch (Rayon parallel)"),
        (0, "4 PPO inner epochs on mini-batches of 128"),
        (0, "KL early-stop at 0.05 aborts inner loop if update too large"),
        (0, "AdamW · cosine LR 1e-3 → ~0 over 100 epochs"),
        (0, "100 epochs total per run"),
    ], size=15, rich=True)

    # Hyperparams table (left bottom)
    hparams = [
        ("d_model","128"),  ("Layers","2"),     ("Heads","4"),
        ("FF dim","256"),   ("Dropout","0.1"),  ("Horizon K","20"),
        ("Actions","29"),   ("Ep/epoch","256"), ("Mini-batch","128"),
    ]
    box(sl, 0.45, 4.3, 5.65, 0.38, "Hyperparameters", size=14, bold=True, color=ACCENT)
    for i, (k, v) in enumerate(hparams):
        col = i%3; row = i//3
        x = 0.45 + col*1.88; y = 4.76 + row*0.52
        rect(sl, x, y, 1.85, 0.49, PANEL if row%2==0 else BG)
        box(sl, x+0.07, y+0.06, 1.0, 0.37, k, size=11, color=LGRAY)
        box(sl, x+1.07, y+0.06, 0.75, 0.37, v, size=11, bold=True, color=YELLOW)

    # ── Right: 3 coloured formula panels ─────────────────────────────────────
    box(sl, 6.35, 1.2, 6.65, 0.4,
        "L  =  −E[ L_clip ]  +  c_v · E[ L_value ]  −  c_e · E[ L_entropy ]",
        size=14, bold=True, color=WHITE, align=PP_ALIGN.CENTER)

    # Panel 1 — CLIP (blue)
    panels = [
        (ACCENT,  "CLIP Term  — policy gradient",
         "L_clip = min( ρₜ · Âₜ ,  clip(ρₜ, 1−ε, 1+ε) · Âₜ )",
         [
             ("ρₜ = π_θ(aₜ|sₜ) / π_old(aₜ|sₜ)", "probability ratio new/old policy"),
             ("Âₜ = Rₜ − V_θ(sₜ)", "normalised advantage (batch-normalised to 0 mean, unit var)"),
             ("ε = 0.2", "clip threshold — hard limit on policy drift per update"),
         ],
         "Bounds how far the updated policy can deviate from the collection policy,\n"
         "keeping log-prob ratios valid across multiple inner gradient steps."),
        (ORANGE,  "Value Term  —  c_v = 0.5",
         "L_value = ( V_θ(sₜ) − Rₜ )²",
         [
             ("V_θ(sₜ)", "value head prediction (reads token 0 in TFX, hₜ in GRU)"),
             ("Rₜ", "per-step return (episode or weighted formulation)"),
             ("EV → 1", "explained variance convergence = calibrated baseline"),
         ],
         "Trains value head to predict episode returns accurately;\n"
         "EV ≈ 1 confirms tight baseline → half of policy gradient reinforces, half penalises."),
        (ACCENT2, "Entropy Bonus  —  c_e = 0.03",
         "L_entropy = H( π_θ(·|sₜ) )  [Shannon entropy]",
         [
             ("H(π)", "encourages spread over all 29 actions"),
             ("c_e = 0.03", "small enough not to dominate; large enough to prevent collapse"),
             ("KL ≤ 0.05", "additional per-mini-batch early-stop guard"),
         ],
         "Prevents entropy collapse (policy locking onto a single pass too early);\n"
         "entropy decays from max → ~30% of max across training."),
    ]
    for pi, (col, title, formula, rows, note) in enumerate(panels):
        y = 1.72 + pi * 1.75
        rect(sl, 6.35, y, 6.65, 1.72, PANEL)
        rect(sl, 6.35, y, 0.07, 1.72, col)
        box(sl, 6.5, y+0.07, 6.42, 0.34, title, size=13, bold=True, color=col)
        box(sl, 6.5, y+0.44, 6.42, 0.38, formula, size=13, bold=True, color=YELLOW)
        for ri, (sym, desc) in enumerate(rows):
            box(sl, 6.52, y+0.85+ri*0.22, 1.4, 0.22, sym, size=10, bold=True, color=col)
            box(sl, 7.95, y+0.85+ri*0.22, 5.0, 0.22, desc, size=10, color=LGRAY, italic=True)
content_slide(sl, "PPO Training Setup", _s12)


# ═══════════════════════════════════════════════════════════════════════════════
# 13 — Return Formulations
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK)
def _s13(sl):
    rect(sl, 0.45, 1.2, 5.9, 2.9, PANEL)
    rect(sl, 0.45, 1.2, 5.9, 0.5, ACCENT)
    box(sl, 0.55, 1.25, 5.7, 0.4, "Episode Return", size=19, bold=True, color=BG)
    rich_box(sl, 0.55, 1.75, 5.7, 0.5, "Rₜ = r  ∀ t", size=18, base=WHITE)
    bullet_box(sl, 0.55, 2.35, 5.7, 1.5, [
        (0, "Terminal speedup assigned uniformly to all steps"),
        (0, "Unbiased maximum-likelihood estimate"),
        (0, "No assumptions about which steps were responsible"),
    ], size=14, color=LGRAY, rich=True)

    rect(sl, 6.9, 1.2, 6.0, 2.9, PANEL)
    rect(sl, 6.9, 1.2, 6.0, 0.5, ACCENT2)
    box(sl, 7.0, 1.25, 5.8, 0.4, "Instruction-Weighted Return", size=19, bold=True, color=BG)
    rich_box(sl, 7.0, 1.75, 5.8, 0.6,
             "Rₜ = r·|ΔIₜ|/Σ|ΔIₛ| + 0.05·sign(−ΔIₜ) − 0.025·𝟙[noopₜ]",
             size=13, base=WHITE)
    bullet_box(sl, 7.0, 2.45, 5.8, 1.4, [
        (0, "Redistributes credit proportional to instruction reduction"),
        (0, "No-op penalty 0.025; direction bonus 0.05"),
        (0, "Sum of first term = r (total reward conserved)"),
    ], size=14, color=LGRAY, rich=True)

    rect(sl, 0.45, 4.25, 12.4, 2.15, RGBColor(0x1A,0x1A,0x2E))
    rect(sl, 0.45, 4.25, 12.4, 0.42, RGBColor(0x44,0x44,0x66))
    box(sl, 0.55, 4.3, 12.2, 0.32, "IR-Step Return  (Ablation)",
        size=15, bold=True, color=LGRAY)
    bullet_box(sl, 0.55, 4.75, 12.2, 1.4, [
        (0, "Per-step normalised instruction-count delta as reward — bypasses benchmarking entirely"),
        (0, "Dense, low-noise signal: verifies model architecture + PPO loop independently of sparse runtime reward"),
        (0, "Not a policy evaluated for speedup; training results are an infrastructure verification only"),
    ], size=14, color=LGRAY, rich=True)

    box(sl, 0.45, 6.55, 12.4, 0.52,
        "No-op threshold: |ΔI| < 0.01 AND L1(Δfeatures) < 0.05  "
        "(dual threshold handles loop-rotate/licm restructuring without instruction-count change)",
        size=12, color=LGRAY, italic=True)
content_slide(sl, "Return Formulations  &  Credit Assignment", _s13)


# ═══════════════════════════════════════════════════════════════════════════════
# 14 — Section: Results
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK)
section_header(sl, "Results", "Auto-TFX + Episode Return")


# ═══════════════════════════════════════════════════════════════════════════════
# 15 — Training Curves
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK)
def _s15(sl):
    train_img = os.path.join(CKPT, "auto-tfx-episode-256ep", "train_plots.png")
    if not os.path.exists(train_img):
        train_img = os.path.join(CKPT, "auto-tfx-episode-256ep.png")
    if os.path.exists(train_img): img(sl, train_img, 0.45, 1.2, 7.8)
    bullet_box(sl, 8.4, 1.25, 4.7, 5.7, [
        (0, "Mean speedup: −0.40 → +0.149"),
        (0, "Explained Variance → ≈ 1"),
        (1, "Value head learns accurate return predictions"),
        (1, "48-dim delta features sufficient for value function"),
        (0, "Entropy: max → ~30% of max"),
        (1, "Progressive concentration, no collapse"),
        (0, "No-op % falls across training"),
        (1, "Policy learns to prefer impactful passes"),
        (0, "interpreter and kmp_search lead per-function"),
        (1, "Consistent with high random-search ceiling"),
    ], size=15, rich=True)
content_slide(sl, "Auto-TFX + Episode Return — Training Curves", _s15)


# ═══════════════════════════════════════════════════════════════════════════════
# 16 — Evaluation Results
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK)
def _s16(sl):
    eval_img = os.path.join(CKPT, "auto-tfx-episode-256ep", "eval.png")
    if not os.path.exists(eval_img):
        eval_img = os.path.join(BASE, "eval.png")
    if os.path.exists(eval_img): img(sl, eval_img, 0.45, 1.2, 7.5)
    rows = [
        ("Function",       "rand mean","rand best","greedy",  "samp best"),
        ("array_reduction","−0.791",   "−0.062",   "+0.092",  "+0.104"),
        ("binary_tree",    "−0.082",   "+0.082",   "+0.077",  "+0.086"),
        ("fft",            "−0.677",   "−0.002",   "+0.063",  "+0.072"),
        ("interpreter",    "−0.260",   "+0.266",   "+0.277",  "+0.283"),
        ("kmp_search",     "−0.107",   "+0.191",   "+0.277",  "+0.278"),
        ("poly_eval",      "−0.181",   "+0.140",   "+0.144",  "+0.145"),
    ]
    col_x = [8.1, 9.65, 10.65, 11.5, 12.3]
    col_w = [1.5, 0.95, 0.85, 0.78, 0.9]
    for ri, row in enumerate(rows):
        y = 1.25 + ri*0.74
        rect(sl, 8.05, y, 5.1, 0.7, ACCENT if ri==0 else (PANEL if ri%2 else BG))
        for ci, (cell, cx, cw) in enumerate(zip(row, col_x, col_w)):
            if ri == 0: fc = BG
            elif ci in (3,4): fc = ACCENT2 if cell.startswith("+") else RED
            elif ci == 1 and any(cell.startswith(p) for p in ("−0.6","−0.7","−0.8")): fc = RED
            else: fc = LGRAY
            align = PP_ALIGN.LEFT if ci==0 else PP_ALIGN.CENTER
            box(sl, cx, y+0.07, cw, 0.56, cell, size=12,
                bold=(ri==0 or (ri>0 and ci in (3,4))), color=fc, align=align)
    rect(sl, 8.05, 6.45, 5.1, 0.72, RGBColor(0x0A,0x22,0x14))
    box(sl, 8.1, 6.52, 5.0, 0.38,
        "Greedy beats -O3 on all 6 functions  ·  mean +15.5%",
        size=14, bold=True, color=ACCENT2, align=PP_ALIGN.CENTER)
content_slide(sl, "Auto-TFX + Episode Return — Evaluation vs -O3", _s16)


# ═══════════════════════════════════════════════════════════════════════════════
# 17 — Benchmarking Harness Rigor
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK)
def _s17(sl):
    box(sl, 0.45, 1.25, 5.9, 0.42, "Benchmarking Harness", size=17, bold=True, color=ACCENT)
    bullet_box(sl, 0.45, 1.73, 5.9, 2.3, [
        (0, "201 timed iterations per measurement"),
        (1, "10% trimmed mean — removes 10 outliers each tail"),
        (1, "5 warm-up runs before timing begins"),
        (0, "CLOCK_MONOTONIC — monotonic wall-clock, nanosecond resolution"),
        (0, "Serial baselines collected once under stable single-threaded conditions"),
        (0, "Episode collection: parallelised across all CPU cores via Rayon"),
    ], size=14, color=LGRAY, rich=True)
    rect(sl, 0.45, 4.12, 5.9, 2.65, PANEL)
    box(sl, 0.55, 4.18, 5.7, 0.38, "Parallel Noise Characterisation",
        size=15, bold=True, color=ACCENT2)
    bullet_box(sl, 0.55, 4.62, 5.7, 2.0, [
        (0, "bench-noise: for each binary, run solo then 16 Rayon workers simultaneously"),
        (0, "Parallel overhead: small and consistent across all 6 benchmarks"),
        (0, "Noise margin 1.01 on -O3 baseline during training:"),
        (1, "Policy must beat -O3 by > noise floor to receive positive reward"),
        (0, "Prevents noise-driven false positives in the reward signal"),
    ], size=13, color=LGRAY, rich=True)

    box(sl, 6.7, 1.25, 6.2, 0.42, "Reproducibility Verification  (diagnose)",
        size=17, bold=True, color=ACCENT)
    bullet_box(sl, 6.7, 1.73, 6.2, 2.85, [
        (0, "Problem: training speedups collected under parallel timing noise"),
        (0, "diagnose re-benchmarks top sequences serially under controlled conditions"),
        (1, "20 independent serial benchmark runs per sequence"),
        (1, "Reports mean, std, and full distribution"),
        (0, "Result: cached training speedups correlate strongly with re-measured values"),
        (0, "Per-sequence distributions are tight → reward signal is reproducible"),
        (0, "Dominant sequences from interpreter and kmp_search — consistent with EDA ceiling"),
    ], size=14, color=LGRAY, rich=True)

    for path, l in [
        (os.path.join(CKPT, "bench_noise-fft.png"), 6.7),
        (os.path.join(CKPT, "bench_noise-kmp.png"), 9.8),
    ]:
        img(sl, path, l, 4.72, 2.95)
    diag_img = os.path.join(CKPT, "diagnose.png")
    if os.path.exists(diag_img):
        img(sl, diag_img, 6.7, 4.72, 6.2)
    elif not os.path.exists(os.path.join(CKPT, "bench_noise-fft.png")):
        rect(sl, 6.7, 4.72, 6.2, 2.55, RGBColor(0x0A,0x10,0x1C))
        box(sl, 6.7, 5.55, 6.2, 0.8,
            "bench_noise-{fft,kmp}.png · diagnose.png\n"
            "[cargo run -- bench-noise  /  cargo run -- plot-diagnose]",
            size=12, color=RGBColor(0x55,0x55,0x77),
            align=PP_ALIGN.CENTER, italic=True)
content_slide(sl, "Benchmarking Harness Rigor", _s17)


# ═══════════════════════════════════════════════════════════════════════════════
# 18 — Discussion
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK)
def _s18(sl):
    bullet_box(sl, 0.45, 1.25, 5.9, 5.8, [
        (0, "Value head (EV ≈ 1) validates feature representation"),
        (1, "48-dim delta features + action history sufficient to predict speedup"),
        (1, "Batch-normalised advantages → ~50% reinforce / ~50% penalise"),
        (0, "Episode return is theoretically clean:"),
        (1, "Only externally measured quantity is r; uniform is unbiased"),
        (1, "Value function disentangles per-step credit"),
        (0, "Instruction-weighted shaping trades bias for lower variance"),
        (1, "Total reward conserved; only redistribution changes"),
        (1, "Helps value head early when policy makes many no-ops"),
        (0, "Transformer vs GRU: complementary memory hypotheses"),
        (1, "GRU: recency bias — free inductive prior, low parameter cost"),
        (1, "TFX: non-local attention — learns which step-pairs matter"),
    ], size=15, rich=True)
    rect(sl, 6.5, 1.25, 6.45, 5.8, PANEL)
    box(sl, 6.6, 1.3, 6.25, 0.42, "Stop-Token Behaviour", size=17, bold=True, color=YELLOW)
    bullet_box(sl, 6.6, 1.8, 6.25, 3.5, [
        (0, "Policy never learns to Stop early — always uses all K=20 steps"),
        (0, "Rational given credit-assignment structure:"),
        (1, "Q(Stop) = current speedup"),
        (1, "Q(any pass) = E[future speedup] — uncertain, possibly higher"),
        (0, "No-op penalty helps at the margin but doesn't fix the asymmetry:"),
        (1, "Cost of stopping: unbounded foregone gain"),
        (1, "Cost of one more pass: at most p = 0.025"),
        (0, "Many top sequences: 5–7 repetitions of dominant pass"),
        (1, "Repeating is safe and occasionally lucky"),
    ], size=13, color=LGRAY, rich=True)
    box(sl, 6.6, 5.42, 6.25, 0.38, "Potential Fix:", size=14, bold=True, color=ACCENT2)
    bullet_box(sl, 6.6, 5.85, 6.25, 0.9, [
        (0, "Per-step length cost, or Stop bonus ≥ E[marginal return from one more pass]"),
    ], size=13, color=LGRAY, rich=True)
content_slide(sl, "Discussion", _s18)


# ═══════════════════════════════════════════════════════════════════════════════
# 19 — Limitations & Future Work
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK)
def _s19(sl):
    bullet_box(sl, 0.45, 1.25, 5.9, 2.8, [
        (0, "Limitations"),
        (1, "Benchmark timing noise limits per-episode credit precision"),
        (1, "Training set of 6 functions is small"),
        (1, "Pass parameters not tuned (loop-unroll count always default)"),
        (1, "Effective horizon = K; Stop token unused"),
    ], size=16, rich=True)
    bullet_box(sl, 0.45, 4.3, 5.9, 2.8, [
        (0, "Ongoing Work"),
        (1, "Architecture comparison: Auto-GRU vs Auto-TFX"),
        (1, "Return comparison: episode vs instruction-weighted"),
        (1, "Hold-out pool evaluation on all 38 functions"),
    ], size=16, rich=True)
    bullet_box(sl, 6.5, 1.25, 6.45, 5.7, [
        (0, "Future Directions"),
        (1, "Per-step length cost or Stop bonus"),
        (1, "Extend pass menu to cover more LLVM 20 transforms"),
        (1, "Transfer to larger, real-world C/C++ programs"),
        (1, "Beam search or MCTS at inference time"),
        (1, "Combine offline bench-cache with online RL updates"),
        (1, "Pass hyperparameter tuning as part of the action space"),
        (1, "Cross-architecture generalisation (ARM, RISC-V)"),
    ], size=16, rich=True)
content_slide(sl, "Limitations & Future Work", _s19)


# ═══════════════════════════════════════════════════════════════════════════════
# 20 — Conclusion  (clean — no pending-run mentions)
# ═══════════════════════════════════════════════════════════════════════════════
sl = prs.slides.add_slide(BLANK); bg(sl)
rect(sl, 0, 0, 0.10, 7.5, ACCENT)
box(sl, 0.6, 0.5, 12.1, 0.7, "Conclusion", size=34, bold=True, color=WHITE)
hline(sl, 1.35, ACCENT, Pt(1))

bullet_box(sl, 0.6, 1.52, 12.0, 5.1, [
    (0, "Autoregressive RL policies learn to beat -O3 on every training benchmark"),
    (1, "Auto-TFX + episode return: 6–28% speedup, mean 15.5% across 6 functions"),
    (0, "Explained Variance ≈ 1 confirms the IR delta representation and learned value baseline"),
    (1, "48-dim delta features + action history are sufficient to predict terminal speedup per-step"),
    (0, "Two architectures (Auto-GRU, Auto-TFX) × two return formulations provide systematic comparison"),
    (0, "Stop-token analysis reveals a structural incentive to exhaust the horizon"),
    (1, "Rational response to terminal reward + unbounded expected future gain"),
    (1, "Design fix: per-step length cost or Stop bonus calibrated to marginal return"),
    (0, "Key insight: IR state must capture structural position within the function body"),
    (1, "Delta representation encodes how instruction composition shifts across positional chunks"),
    (1, "Structural changes like loop-rotate and licm are invisible to global count vectors"),
], size=17, color=WHITE, rich=True)

box(sl, 0.6, 6.6, 12.1, 0.55,
    "Learning LLVM Pass Sequences via Reinforcement Learning with Autoregressive Policies  "
    "·  Evan Black  ·  ODU",
    size=13, color=LGRAY, italic=True)


# ── Save ──────────────────────────────────────────────────────────────────────
out = os.path.join(BASE, "presentation.pptx")
prs.save(out)
print(f"Saved: {out}  ({len(prs.slides)} slides)")
