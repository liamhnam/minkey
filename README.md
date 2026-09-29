<h1 align="center">
  <br>
  ⚡ Minkey
  <br>
</h1>

<h4 align="center">Bộ gõ tiếng Việt thế hệ mới — viết lại hoàn toàn bằng <strong>Rust</strong>, dựa trên nền tảng của UniKey 3.6 và kiến trúc OpenKey.</h4>

<p align="center">
  <a href="https://www.rust-lang.org">
    <img src="https://img.shields.io/badge/Rust-2024_Edition-orange?style=flat-square&logo=rust" alt="Rust Edition">
  </a>
  <a href="LICENSE">
    <img src="https://img.shields.io/badge/License-GPLv3-blue?style=flat-square" alt="License">
  </a>
  <img src="https://img.shields.io/badge/Platform-Windows%2010%2F11-0078D4?style=flat-square&logo=windows" alt="Platform">
  <img src="https://img.shields.io/badge/Binary_Size-~380_KB-green?style=flat-square" alt="Binary Size">
  <img src="https://img.shields.io/badge/RAM_at_idle-~2--3_MB-green?style=flat-square" alt="RAM Usage">
  <img src="https://img.shields.io/badge/CPU_at_idle-0.0%25-green?style=flat-square" alt="CPU Usage">
</p>

<p align="center">
  <a href="#-tính-năng">Tính năng</a> •
  <a href="#-hiệu-năng">Hiệu năng</a> •
  <a href="#-kiến-trúc">Kiến trúc</a> •
  <a href="#%EF%B8%8F-biên-dịch--cài-đặt">Biên dịch</a> •
  <a href="#-câu-hỏi-thường-gặp">FAQ</a> •
  <a href="#-giấy-phép">Giấy phép</a>
</p>

---

## Giới thiệu

**Minkey** là bộ gõ tiếng Việt mã nguồn mở, được viết lại hoàn toàn bằng **Rust** — dựa trên nền tảng thuật toán của **UniKey 3.6** và kiến trúc phần mềm của **OpenKey**. Mục tiêu là giữ **100% tính tương thích hành vi** với OpenKey gốc (phím tắt, Registry, định dạng gõ tắt, bảng mã), đồng thời đạt được:

- **An toàn bộ nhớ tuyệt đối** (Rust memory safety, zero leaks)
- **Siêu nhẹ**: ~380 KB file thực thi, ~2–3 MB RAM khi chạy nền
- **Giao diện Win32 thuần** — không GPU, không runtime nặng nề

---

## ✨ Tính năng

### 🎹 Kiểu gõ
| # | Tên | Ví dụ |
|---|-----|-------|
| 0 | **Telex** | `vieetj` → `việt` |
| 1 | **VNI** | `vie65t7` → `việt` |
| 2 | **Simple Telex 1** | Telex đơn giản kiểu 1 |
| 3 | **Simple Telex 2** | Telex đơn giản kiểu 2 |

### 🗂️ Bảng mã
| # | Tên | Ghi chú |
|---|-----|---------|
| 0 | **Unicode dựng sẵn** | Chuẩn quốc tế, khuyến dùng |
| 1 | **TCVN3 (ABC)** | Tương thích tài liệu cũ |
| 2 | **VNI Windows** | Bảng mã VNI |
| 3 | **Unicode tổ hợp** | Compound Unicode |
| 4 | **Vietnamese CP 1258** | Windows Locale |

### 🧠 Quy tắc & Thuật toán gõ (tương thích UniKey 3.6 / OpenKey)

- **Bỏ dấu kiểu mới hoặc cũ**: `hoà / hòa`, `thuý / thúy` tuỳ cài đặt
- **Kiểm tra chính tả & tự phục hồi**: gõ từ sai → tự động trả về phím gốc
- **Gõ nhanh Telex**: `cc`→`ch`, `gg`→`gi`, `kk`→`kh`, `nn`→`ng`, `qq`→`qu`, `pp`→`ph`, `tt`→`th`, `uu`→`ươ`
- **Phụ âm đầu thông minh**: `f`→`ph`, `j`→`gi`, `w`→`qu`
- **Phụ âm cuối thông minh**: `g`→`ng`, `h`→`nh`, `k`→`ch`
- **Phím ngoặc vuông**: `[`→`ơ`, `]`→`ư`
- **Xóa dấu** bằng phím `z`, **hoàn tác dấu** khi gõ lặp dấu
- **Tự động viết hoa** đầu câu sau dấu chấm và phím Enter
- **Tạm tắt kiểm tra chính tả**: nhấn đúp `Ctrl`
- **Tạm tắt bộ gõ**: nhấn `Alt`

### 🔧 Khắc phục lỗi ứng dụng

- Xóa triệt để **trùng ký tự / nhảy chữ** trên thanh địa chỉ trình duyệt (Chrome, Edge, Firefox, Brave) và ô nhập Excel
- **Bản vá Chromium** đặc biệt (`Shift + Left Arrow`)
- Hỗ trợ ứng dụng **Metro / UWP Windows Store**
- Tự động **ngắt phiên gõ khi click chuột** (`WH_MOUSE_LL`)
- Quản lý đồng bộ số lượng **backspace chính xác** cho bảng mã 2-byte (VNI, Unicode tổ hợp)

### 📝 Công cụ Gõ tắt (Macro Manager)

- **Tra cứu nhị phân** siêu tốc (`BTreeMap`) với auto-complete
- **AutoCaps thông minh**: `btw`→`by the way`, `Btw`→`By the way`, `BTW`→`BY THE WAY`
- Gõ tắt ngay cả khi ở **chế độ tiếng Anh**
- **Tương thích 100%** định dạng file `.txt` của UniKey và OpenKey (Import / Export)
- Lưu trữ nhị phân trực tiếp trong Registry: `HKCU\Software\TuyenMai\OpenKey\macroData`

### 🔄 Công cụ Chuyển mã (Encoding Converter)

- Chuyển đổi qua lại giữa **tất cả 5 bảng mã**
- Chuyển đổi **Clipboard thông minh**: hỗ trợ Plain Text, HTML (`CF_HTML`), Rich Text (`CF_RTF`)
- Tùy chọn: chữ **HOA**, chữ **thường**, **bỏ dấu**, **hoa đầu câu**, **hoa mỗi từ**
- **Phím tắt toàn cục** chuyển mã nhanh (Quick Convert Hotkey)

### 🤖 Tự chuyển E/V theo ứng dụng (Smart Switch Key)

- **Ghi nhớ ngôn ngữ** (Anh/Việt) và bảng mã riêng biệt theo từng tiến trình ứng dụng
- Tự động chuyển chế độ khi đổi cửa sổ (`EVENT_SYSTEM_FOREGROUND`)
- Dữ liệu lưu trực tiếp trong Registry dưới dạng binary

### 🖥️ Giao diện & Hệ thống

| Tính năng | Chi tiết |
|-----------|----------|
| **Win32 thuần** | Common Controls v6, không GPU, không runtime nặng |
| **Per-Monitor DPI V2** | Tự co giãn theo DPI từng màn hình |
| **Tạo cửa sổ on-demand** | Tạo khi mở, hủy khi đóng; không giữ cửa sổ khi chạy nền |
| **Memory Trimming** | Tự động thu hồi RAM (`EmptyWorkingSet`) khi đóng cửa sổ |
| **Tray icon động** | 🔴/🟠 Tiếng Việt — 🔵 Tiếng Anh |
| **Single Instance** | An toàn — lần thứ hai mở bảng điều khiển và thoát |
| **Khởi động cùng Windows** | Registry Run hoặc Task Scheduler (nếu cần Admin) |

---

## 🚀 Hiệu năng

> Đây là những con số thực đo trên hệ thống Windows 11, chạy nền tại system tray.

| Chỉ số | Giá trị |
|--------|---------|
| **RAM (chạy nền)** | ~2 – 3 MB |
| **CPU (idle)** | 0.0% |
| **Kích thước binary** | ~380 KB (Release) |
| **Input latency** | ≈ 0 ms (dedicated OS thread, `THREAD_PRIORITY_HIGHEST`) |

**Tại sao siêu nhẹ?**

- Profile Release cấu hình cực kỳ tối ưu: `opt-level = "z"`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip = true`
- Không phụ thuộc runtime C/C++, không dùng Electron/Qt/GTK
- Keyboard hook (`WH_KEYBOARD_LL`) chạy trên **dedicated thread độc lập** — tách biệt hoàn toàn với UI thread

---

## 🏗️ Kiến trúc

```
minkey/
├── src/
│   ├── main.rs              # Entry point, single-instance guard
│   ├── lib.rs               # Crate root, module exports
│   ├── types.rs             # Enums, constants, bitmasks (port từ OpenKey)
│   ├── engine.rs            # VietnameseEngine — lõi xử lý phím (1722 dòng)
│   ├── tables.rs            # Bảng mã Unicode/TCVN3/VNI/CP1258 (lookup tables)
│   ├── config.rs            # AppConfig + Registry I/O (tương thích OpenKey)
│   ├── hook.rs              # WH_KEYBOARD_LL + WH_MOUSE_LL (Windows)
│   ├── hook_macos.rs        # CGEventTap (macOS, reserved)
│   ├── tray.rs              # System Tray + popup menu (Windows)
│   ├── tray_macos.rs        # NSStatusBar (macOS, reserved)
│   ├── macro_engine.rs      # MacroTable — BTreeMap, import/export .txt
│   ├── smart_switch.rs      # SmartSwitchTable — per-app language memory
│   ├── convert.rs           # Encoding converter (5 bảng mã × 2 chiều)
│   ├── settings.rs          # Cửa sổ Settings (Win32)
│   ├── dialog.rs            # Common dialogs helper
│   ├── app.rs               # Application state container
│   └── win32_ui/
│       ├── mod.rs           # Win32 app loop & message pump
│       ├── main_window.rs   # Bảng điều khiển chính
│       ├── macro_window.rs  # Cửa sổ quản lý gõ tắt
│       ├── convert_window.rs# Cửa sổ chuyển mã
│       └── controls.rs      # Custom Win32 controls (toggle, card, sidebar)
├── tests/
│   ├── telex_test.rs        # Test kiểu gõ Telex
│   ├── vni_test.rs          # Test kiểu gõ VNI
│   ├── simple_telex_test.rs # Test Simple Telex
│   ├── convert_test.rs      # Test chuyển mã bảng mã
│   ├── features_test.rs     # Test các tính năng nâng cao
│   ├── macro_test.rs        # Test macro engine
│   ├── smart_switch_test.rs # Test smart switch key
│   ├── settings_test.rs     # Test config load/save
│   └── openkey_parity_test.rs # Kiểm tra parity 100% với OpenKey
├── res/
│   └── minkey.manifest      # Windows manifest (Common Controls v6, Per-Monitor DPI V2)
├── build.rs                 # Nhúng manifest vào binary (MSVC linker)
├── bundle_macos.sh          # Script đóng gói .app cho macOS
└── Cargo.toml
```

### Luồng xử lý phím

```
[OS Keyboard Event]
       │
       ▼ WH_KEYBOARD_LL (dedicated thread, THREAD_PRIORITY_HIGHEST)
  hook.rs
       │
       ▼ process_key_event()
  engine.rs (VietnameseEngine)
       │
       ├─► macro_engine.rs  (kiểm tra gõ tắt)
       ├─► tables.rs        (tra bảng mã)
       └─► HookState { backspace_count, new_char_count, char_data[] }
              │
              ▼ SendInput() / PostMessage()
         [Ứng dụng đích nhận ký tự tiếng Việt]
```

---

## 🛠️ Biên dịch & Cài đặt

### Yêu cầu

| Thành phần | Phiên bản |
|------------|-----------|
| **Hệ điều hành** | Windows 10 / 11 (x64) |
| **Rust toolchain** | `stable` ≥ 1.80 (`rustup` + target `x86_64-pc-windows-msvc`) |
| **Build tools** | Visual Studio Build Tools 2022 (MSVC linker) |

### Cài đặt Rust (nếu chưa có)

```powershell
# Cài Rust qua rustup
winget install Rustlang.Rustup
# Sau đó restart terminal và kiểm tra:
rustc --version
cargo --version
```

### Biên dịch

```bash
# Chạy toàn bộ automated test suite
cargo test --release

# Build bản Release tối ưu (~380 KB, không console)
cargo build --release
```

File thực thi sau khi build:

```
target\release\minkey.exe
```

> **Không cần cài đặt thêm DLL hay runtime.** Copy file `minkey.exe` đến bất kỳ đâu và chạy thẳng.

### Build nhanh (Debug, có console log)

```bash
cargo build
# → target\debug\minkey.exe
```

---

## ⌨️ Phím tắt mặc định

| Phím tắt | Chức năng |
|----------|-----------|
| `Alt + Z` | Chuyển đổi Tiếng Anh / Tiếng Việt |
| Nhấn đúp `Ctrl` | Tạm tắt / bật kiểm tra chính tả |
| Nhấn `Alt` (giữ) | Tạm tắt bộ gõ trong phiên gõ hiện tại |
| `z` (cuối từ) | Xóa dấu |
| `[` | Gõ `ơ` |
| `]` | Gõ `ư` |

*Phím chuyển E/V có thể thay đổi trong Bảng điều khiển → tab Cài đặt.*

---

## 🗝️ Cấu hình Registry

Minkey lưu toàn bộ cấu hình tại:

```
HKEY_CURRENT_USER\Software\TuyenMai\OpenKey
```

**100% tương thích** với schema Registry của OpenKey — bao gồm cả dữ liệu Macro và Smart Switch Key. Người dùng chuyển từ OpenKey sang Minkey **không mất bất kỳ cài đặt nào**.

<details>
<summary>Danh sách các key Registry chính</summary>

| Key | Mô tả | Mặc định |
|-----|-------|----------|
| `vLanguage` | Ngôn ngữ (0=Anh, 1=Việt) | 1 |
| `vInputType` | Kiểu gõ (0=Telex, 1=VNI, 2=ST1, 3=ST2) | 0 |
| `vCodeTable` | Bảng mã (0=Unicode…4=CP1258) | 0 |
| `vCheckSpelling` | Kiểm tra chính tả | 1 |
| `vUseModernOrthography` | Đặt dấu kiểu mới (`oà`, `uý`) | 0 |
| `vQuickTelex` | Gõ nhanh Telex | 0 |
| `vSwitchKeyStatus` | Phím chuyển E/V | `Alt+Z` |
| `vUseMacro` | Bật gõ tắt | 1 |
| `vUseSmartSwitchKey` | Tự chuyển E/V theo app | 1 |
| `vFixRecommendBrowser` | Sửa lỗi gợi ý trình duyệt | 1 |
| `vFixChromiumBrowser` | Bản vá Chromium | 0 |
| `vSupportMetroApp` | Hỗ trợ Metro/UWP | 0 |
| `vUpperCaseFirstChar` | Tự viết hoa đầu câu | 0 |
| `macroData` | Dữ liệu gõ tắt (binary) | — |
| `smartSwitchData` | Dữ liệu Smart Switch (binary) | — |

</details>

---

## 🧪 Tests

Bộ test bao phủ toàn diện các quy tắc gõ tiếng Việt:

```bash
cargo test                   # Chạy tất cả tests
cargo test telex             # Chỉ test kiểu gõ Telex
cargo test openkey_parity    # Kiểm tra parity 100% với OpenKey
cargo test -- --nocapture    # Xem output chi tiết
```

| Test file | Nội dung |
|-----------|----------|
| `telex_test.rs` | 50+ trường hợp gõ Telex |
| `vni_test.rs` | 30+ trường hợp gõ VNI |
| `simple_telex_test.rs` | Simple Telex 1 & 2 |
| `convert_test.rs` | Chuyển mã qua lại 5 bảng mã |
| `features_test.rs` | Gõ nhanh, phụ âm thông minh, xóa dấu |
| `macro_test.rs` | Import/Export, AutoCaps, tra cứu |
| `smart_switch_test.rs` | Per-app language memory |
| `settings_test.rs` | Load/Save config |
| `openkey_parity_test.rs` | So sánh kết quả 1:1 với OpenKey |

---

## ❓ Câu hỏi thường gặp

<details>
<summary><strong>Minkey có xung đột với bộ gõ khác không?</strong></summary>

Không cần gỡ bộ gõ cũ, nhưng **không nên chạy cùng lúc** hai bộ gõ. Hãy tắt UniKey / OpenKey / EVKey trước khi chạy Minkey.

</details>

<details>
<summary><strong>Tại sao không có file .dll / runtime?</strong></summary>

Minkey được biên dịch tĩnh hoàn toàn với Rust + MSVC. Tất cả code được liên kết trực tiếp vào một file `.exe` duy nhất, không cần thư viện bên ngoài nào.

</details>

<details>
<summary><strong>Cách import danh sách gõ tắt từ UniKey / OpenKey?</strong></summary>

Mở Minkey → **Gõ tắt** → **Import** → Chọn file `.txt` xuất từ UniKey hoặc OpenKey. Định dạng file 100% tương thích.

</details>

<details>
<summary><strong>Tại sao bộ gõ không hoạt động trong một số phần mềm chạy quyền Admin?</strong></summary>

Windows giới hạn Low-Level Hook khi ứng dụng đích chạy quyền Admin còn bộ gõ thì không. Vào **Cài đặt → Chạy với quyền quản trị** để khắc phục.

</details>

<details>
<summary><strong>Minkey có hỗ trợ macOS không?</strong></summary>

Các module macOS (`hook_macos.rs`, `tray_macos.rs`) còn trong source nhưng **hiện chưa build**. Phiên bản macOS sẽ được phát triển riêng theo hướng khác trong tương lai.

</details>

---

## 🤝 Đóng góp

Mọi Pull Request, Issue và góp ý đều được chào đón!

```bash
# Fork repo → clone về máy
git clone https://github.com/liamhnam/minkey.git
cd minkey

# Tạo branch mới
git checkout -b feature/ten-tinh-nang

# Chạy tests trước khi commit
cargo test --release

# Commit và push
git push origin feature/ten-tinh-nang
```

**Quy tắc commit message**:
- `feat:` tính năng mới
- `fix:` sửa lỗi
- `refactor:` cải thiện code không thêm tính năng
- `test:` bổ sung tests
- `docs:` cập nhật tài liệu

---

## 📜 Giấy phép

Minkey được phát hành theo giấy phép **[GNU General Public License v3.0 (GPLv3)](LICENSE)**.

---

## 🙏 Ghi công

| Vai trò | Người |
|---------|-------|
| **Thuật toán gốc** | Nguyễn Tấn Phát — [UniKey 3.6](https://www.unikey.org) |
| **Tác giả OpenKey** | Mai Vũ Tuyên — [tuyenvm/OpenKey](https://github.com/tuyenvm/OpenKey) |
| **Rust Remake** | Liam |

> *Minkey được xây dựng trên nền tảng thuật toán của UniKey 3.6 — bộ gõ tiếng Việt huyền thoại của người Việt.*
