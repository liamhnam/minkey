import random, unicodedata, itertools
random.seed(42)
onsets = ["", "b","c","ch","d","đ","g","gh","gi","h","k","kh","l","m","n","ng","ngh","nh","p","ph","qu","r","s","t","th","tr","v","x"]
nuclei = ["a","ă","â","e","ê","i","o","ô","ơ","u","ư","y","ai","ao","au","ay","âu","ây","eo","êu","ia","iê","iu","oa","oă","oe","oi","ôi","ơi","ua","uâ","uê","ui","uô","uơ","ưa","ưi","ươ","ưu","uy","yê","oai","oay","uôi","ươi","ươu","uya","uyê","yêu","iêu","uây"]
codas = ["", "c","ch","m","n","ng","nh","p","t","i","u","o","y"]
tones = ["", "́","̀","̉","̃","̣"]

def syll():
    w = random.choice(onsets) + random.choice(nuclei) + random.choice(codas)
    t = random.choice(tones)
    if t:
        # put tone on a random vowel (engine decides the final position)
        idx = [i for i,c in enumerate(w) if unicodedata.normalize('NFD',c)[0] in 'aeiouy']
        if idx:
            i = random.choice(idx); w = w[:i+1] + t + w[i+1:]
    w = unicodedata.normalize('NFC', w)
    if random.random() < 0.15: w = w.capitalize()
    if random.random() < 0.03: w = w.upper()
    return w

TELEX = {'̆':'w','̛':'w','́':'s','̀':'f','̉':'r','̃':'x','̣':'j'}
VNI = {'̆':'8','̛':'7','̂':'6','́':'1','̀':'2','̉':'3','̃':'4','̣':'5'}
def to_keys(word, vni, tone_last):
    base, mods, tone = "", "", ""
    for ch in word:
        if ch in "đĐ":
            base += "d" if ch=="đ" else "D"; mods += "9" if vni else ("d" if ch=="đ" else "D"); continue
        d = unicodedata.normalize('NFD', ch)
        base += d[0]
        for m in d[1:]:
            if m in '̣́̀̉̃': tone = (VNI if vni else TELEX)[m]
            elif m == '̂': mods += "6" if vni else d[0]
            else: mods += (VNI if vni else TELEX)[m]
    if vni: return base + mods + tone
    # telex: type modifiers right after their vowel in natural order is complex; append at end (engine supports free order)
    return base + mods + tone if tone_last else base + tone + mods

lines = []
cfgs = []
for it in (0,1,2):
    for modern in (0,1):
        for spell in (0,1):
            for restore in (0,1):
                cfgs.append((it,modern,spell,restore))
for cfg in cfgs:
    it, modern, spell, restore = cfg
    extra = [random.randint(0,1) for _ in range(4)]  # quick telex, zfwj, quick start, quick end
    for n in range(600):
        words = [syll() for _ in range(random.randint(1,6))]
        keys = " ".join(to_keys(w, it==1, random.random()<0.7) for w in words)
        lines.append(f"{it} {modern} {spell} {restore} {extra[0]} {extra[1]} {extra[2]} {extra[3]} 0|{keys}")
# pure keystroke fuzz
alpha = "aaeeiioouuywwddssffrrxxjjzzbcghklmnpqtv  AEOUWDSF12345678906789@@[],.;'/-?"
for n in range(15000):
    it = random.choice([0,1,2,3]); modern=random.randint(0,1); spell=random.randint(0,1); restore=random.randint(0,1)
    extra=[random.randint(0,1) for _ in range(4)]
    s = "".join(random.choice(alpha) for _ in range(random.randint(1,25)))
    lines.append(f"{it} {modern} {spell} {restore} {extra[0]} {extra[1]} {extra[2]} {extra[3]} 0|{s}")
open("corpus.txt","w").write("\n".join(lines)+"\n")
print(len(lines))
