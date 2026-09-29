import unicodedata, random
random.seed(1)
text = """Việt Nam có bốn nghìn năm lịch sử dựng nước và giữ nước. Người dân nơi đây cần cù, chịu khó và rất hiếu khách.
Thủ đô Hà Nội nằm bên bờ sông Hồng, thành phố Hồ Chí Minh là trung tâm kinh tế lớn nhất cả nước.
Chúng tôi đang thử nghiệm bộ gõ tiếng Việt mới, xem nó xử lý các trường hợp khó như thế nào.
Khuya rồi mà anh ấy vẫn chưa ngủ, cứ ngồi nhìn ra cửa sổ nghĩ ngợi về quê hương khuất xa.
Quyển truyện này hay quá, tôi đọc một mạch hết luôn trong một buổi chiều mưa rả rích.
Hoà bình, thuỷ thủ, khoẻ mạnh, hoạ sĩ, quả táo, thuở xưa, người ta, huấn luyện, khuỷu tay, nguyệt quế.
Giặt giũ, giường ngủ, gì cơ, giết giặc, quăng quật, quạt điện, quốc gia, quyết định, quyền lợi, quý giá.
Ngoằn ngoèo, loằng ngoằng, khoắc, oẳn tù tì, huơ tay, thuở ấy, luôn luôn, tuổi trẻ, cuối cùng, muối mặn.
Rượu ngon, hươu nai, bướu cổ, cười nói, tươi vui, người Việt, được mùa, mượn sách, sướng quá, nướng thịt.
Yêu thương, yểu điệu, yên ổn, uyên bác, khuyên bảo, chuyện trò, truyền thống, tuyệt vời, nguyễn trãi.
Đường đi, đồng hồ, đảo xa, địa lý, đọc sách, đứng dậy, đẹp đẽ, đầy đủ, điện thoại, đi đâu đấy.
Bóng đá, xích lô, phở bò, bánh mì, cà phê sữa đá, nước mắm, chè đỗ đen, bún chả, nem rán, gỏi cuốn."""
TELEX = {'̆':'w','̛':'w','́':'s','̀':'f','̉':'r','̃':'x','̣':'j'}
VNI = {'̆':'8','̛':'7','̂':'6','́':'1','̀':'2','̉':'3','̃':'4','̣':'5'}
def keys(word, vni, style):
    # style 0: all modifiers + tone at word end; style 1: modifier right after its letter, tone at end
    out, tail, tone = "", "", ""
    for ch in word:
        if ch in "đĐ":
            out += ch.replace("đ","d").replace("Đ","D")
            m = "9" if vni else ("d" if ch=="đ" else "D")
            if style==1: out += m
            else: tail += m
            continue
        d = unicodedata.normalize('NFD', ch)
        out += d[0]
        for m in d[1:]:
            if m in '̣́̀̉̃': tone = (VNI if vni else TELEX)[m]
            else:
                k = ("6" if vni else d[0].lower()) if m=='̂' else (VNI if vni else TELEX)[m]
                if style==1: out += k
                else: tail += k
    return out + tail + tone
words = text.replace("\n"," ").split(" ")
lines=[]
for it in (0,1):
    for modern in (0,1):
        for spell in (0,1):
            for style in (0,1):
                ks = []
                for w in words:
                    core = w.strip(".,")
                    ks.append(keys(core, it==1, style) + w[len(core):])
                # feed sentence-by-sentence chunks
                for i in range(0,len(ks),8):
                    lines.append(f"{it} {modern} {spell} 1 0 0 0 0 0|{' '.join(ks[i:i+8])}")
                    exp = " ".join(words[i:i+8])

open("real.txt","w").write("\n".join(lines)+"\n")
# expected output (NFC text), same chunking
exp=[]
for it in (0,1):
    for modern in (0,1):
        for spell in (0,1):
            for style in (0,1):
                for i in range(0,len(words),8): exp.append((modern," ".join(words[i:i+8])))
open("real_exp.txt","w").write("\n".join(f"{m}\t{e}" for m,e in exp)+"\n")
print(len(lines))
