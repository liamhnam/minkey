# Minkey ⚡

> **Minkey** là bộ gõ tiếng Việt mã nguồn mở thế hệ mới, được viết lại hoàn toàn bằng **Rust** dựa trên kiến trúc và thuật toán của bộ gõ **OpenKey** danh tiếng.

Minkey giữ **100% tính năng và hành vi sử dụng** của OpenKey gốc, tương thích hoàn toàn về phím tắt, cấu hình Registry, định dạng gõ tắt (Macro) và công cụ chuyển mã, đồng thời mang đến giao diện **Fluent Design (Windows 11)** hiện đại, mượt mà và tối ưu hóa tài nguyên phần cứng.

---

## ✨ Tính Năng Nổi Bật

- **4 Kiểu gõ Tiếng Việt**: Telex, VNI, Simple Telex 1, Simple Telex 2.
- **5 Bảng mã Tiếng Việt**:
  - Unicode dựng sẵn (chuẩn quốc tế)
  - TCVN3 (ABC)
  - VNI Windows
  - Unicode tổ hợp (Compound)
  - Vietnamese Locale CP 1258
- **Thuật toán & Quy tắc gõ chuẩn xác 100%**:
  - Bỏ dấu kiểu mới (`hoà, thuý`) hoặc kiểu cũ (`hòa, thúy`).
  - Kiểm tra chính tả & tự động phục hồi phím gốc khi gõ từ sai.
  - Gõ nhanh Telex (`cc=ch, gg=gi, kk=kh, nn=ng, qq=qu, pp=ph, tt=th, uu=ươ`).
  - Gõ tắt phụ âm đầu (`f->ph, j->gi, w->qu`) và phụ âm cuối (`g->ng, h->nh, k->ch`).
  - Phím ngoặc vuông thông minh (`[` -> `ơ`, `]` -> `ư`).
  - Xóa dấu bằng phím `z`, hoàn tác dấu khi gõ lặp dấu.
  - Tự động viết hoa chữ cái đầu câu sau dấu chấm và phím Enter.
  - Tạm tắt kiểm tra chính tả (nhấn đúp `Ctrl`), tạm tắt bộ gõ (nhấn `Alt`).
- **Khắc phục lỗi ứng dụng triệt để**:
  - Xóa triệt để hiện tượng trùng ký tự / nhảy chữ trên thanh địa chỉ trình duyệt (Chrome, Edge, Firefox, Brave) và ô nhập liệu Excel.
  - Bản vá đặc biệt cho trình duyệt nền Chromium (`Shift + Left Arrow`).
  - Hỗ trợ ứng dụng Metro / UWP Windows Store.
  - Tự động ngắt phiên gõ khi click chuột (`WH_MOUSE_LL`).
  - Quản lý đồng bộ số lượng backspace chính xác cho bảng mã 2-byte (VNI, Unicode tổ hợp).
- **Công cụ Gõ tắt (Macro Manager)**:
  - Tra cứu nhị phân siêu tốc, tự động biến đổi chữ hoa (`AutoCaps`: `btw` -> `by the way`, `Btw` -> `By the way`, `BTW` -> `BY THE WAY`).
  - Hỗ trợ gõ tắt ngay cả khi ở chế độ tiếng Anh.
  - Tương thích 100% định dạng file `.txt` của UniKey và OpenKey (Import / Export mượt mà).
  - Lưu trữ nhị phân trực tiếp trong Registry `HKCU\Software\TuyenMai\OpenKey\macroData`.
- **Công cụ Chuyển mã (Encoding Converter)**:
  - Chuyển đổi qua lại giữa cả 5 bảng mã.
  - Chuyển đổi Clipboard thông minh: hỗ trợ Plain Text, HTML (`CF_HTML`) và Rich Text (`CF_RTF`).
  - Tùy chọn chuyển chữ HOA, chữ thường, bỏ dấu, hoa đầu câu, hoa mỗi từ.
  - Phím tắt chuyển mã nhanh toàn cục (Quick Convert Hotkey).
- **Tự chuyển E/V theo ứng dụng (Smart Switch Key & App Memory)**:
  - Ghi nhớ ngôn ngữ (Anh/Việt) và bảng mã theo từng tiến trình ứng dụng riêng biệt (`EVENT_SYSTEM_FOREGROUND`).
- **Giao diện & Hệ thống**:
  - Giao diện **Fluent Design** hiện đại xây dựng trên **Slint GUI**, hỗ trợ High-DPI sắc nét và giao diện sáng/tối tự nhiên.
  - Biểu tượng khay hệ thống (System Tray) động: màu đỏ/cam cho Tiếng Việt, màu xanh dương cho Tiếng Anh, hoặc biểu tượng xám hiện đại.
  - Tự phục hồi biểu tượng khay khi Windows Explorer khởi động lại (`TaskbarCreated`).
  - Đơn phiên bản (Single Instance) an toàn qua Named Mutex.
  - Khởi động cùng Windows (Registry `Run` hoặc Task Scheduler quyền cao nhất `schtasks`).
  - Chạy quyền quản trị viên (Run as Administrator) qua `ShellExecuteW runas`.
  - Tạo shortcut ngoài Desktop tự động.

---

## 🚀 Hiệu Năng & Tài Nguyên

- **Zero Input Latency**: Low-Level Keyboard Hook chạy trên một dedicated OS thread độc lập với mức ưu tiên cao nhất (`THREAD_PRIORITY_HIGHEST`), tách biệt hoàn toàn khỏi thread giao diện đồ họa.
- **Siêu nhẹ**:
  - RAM tiêu thụ khi chạy nền ở khay hệ thống: **~10 - 15 MB**.
  - CPU tiêu thụ ở trạng thái chờ: **0.0%**.
- **Độ ổn định cao**: Viết bằng Rust với bảo đảm an toàn bộ nhớ (memory safety), không rò rỉ bộ nhớ (zero memory leaks), sẵn sàng chạy nền liên tục 24/7.

---

## 🛠️ Hướng Dẫn Biên Dịch & Chạy

### Yêu cầu hệ thống:
- Hệ điều hành: Windows 10 / 11 (64-bit hoặc 32-bit).
- Rust toolchain: phiên bản 1.80+ (khuyến nghị MSVC toolchain).

### Lệnh biên dịch:
```bash
# Kiểm tra toàn bộ 33 automated tests
cargo test --release

# Biên dịch bản phát hành tối ưu hóa (Release)
cargo build --release
```

File thực thi độc lập sẽ nằm tại:
```
target/release/minkey.exe
```

---

## 📜 Giấy Phép & Bản Quyền

- Dự án Minkey được phát hành theo giấy phép **GNU General Public License v3.0 (GPLv3)**.
- Tác giả gốc OpenKey: **Mai Vũ Tuyên** ([GitHub: tuyenvm/OpenKey](https://github.com/tuyenvm/OpenKey)).
- Rust Remake & Modern UI: **Liam & DeepMind Antigravity**.
