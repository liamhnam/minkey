<h1 align="center">⚡ Minkey</h1>

<h4 align="center">Bộ gõ tiếng Việt mã nguồn mở — viết bằng <strong>Rust</strong>, siêu nhẹ, siêu nhanh.</h4>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-2024_Edition-orange?style=flat-square&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/License-GPLv3-blue?style=flat-square" alt="License">
  <img src="https://img.shields.io/badge/Platform-Windows%2010%2F11-0078D4?style=flat-square&logo=windows" alt="Platform">
  <img src="https://img.shields.io/badge/Size-~380_KB-brightgreen?style=flat-square" alt="Size">
  <img src="https://img.shields.io/badge/RAM-~2--3_MB-brightgreen?style=flat-square" alt="RAM">
  <img src="https://img.shields.io/badge/CPU_idle-0.0%25-brightgreen?style=flat-square" alt="CPU">
</p>

<p align="center">
  <a href="#-tính-năng">Tính năng</a> •
  <a href="#-hiệu-năng">Hiệu năng</a> •
  <a href="#-kiến-trúc">Kiến trúc</a> •
  <a href="#%EF%B8%8F-biên-dịch">Biên dịch</a> •
  <a href="#-faq">FAQ</a> •
  <a href="#-giấy-phép">Giấy phép</a>
</p>

---

## Giới thiệu

**Minkey** là bộ gõ tiếng Việt mã nguồn mở, viết hoàn toàn bằng **Rust**. Giao diện Win32 thuần — không GPU, không runtime, không phụ thuộc DLL ngoài. Một file `.exe` duy nhất, copy đâu chạy đó.

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

| # | Tên |
|---|-----|
| 0 | Unicode dựng sẵn *(khuyến dùng)* |
| 1 | TCVN3 (ABC) |
| 2 | VNI Windows |
| 3 | Unicode tổ hợp |
| 4 | Vietnamese CP 1258 |

### 🧠 Quy tắc gõ

- **Bỏ dấu kiểu mới hoặc cũ**: `hoà / hòa`, `thuý / thúy` tuỳ cài đặt
- **Kiểm tra chính tả & tự phục hồi**: gõ từ sai → tự trả về phím gốc
- **Gõ nhanh Telex**: `cc`→`ch`, `gg`→`gi`, `kk`→`kh`, `nn`→`ng`, `qq`→`qu`, `pp`→`ph`, `tt`→`th`, `uu`→`ươ`
- **Phụ âm đầu thông minh**: `f`→`ph`, `j`→`gi`, `w`→`qu`
- **Phụ âm cuối thông minh**: `g`→`ng`, `h`→`nh`, `k`→`ch`
- **Phím ngoặc vuông**: `[`→`ơ`, `]`→`ư`
- Xóa dấu bằng phím `z` · Hoàn tác dấu khi gõ lặp dấu
- Tự động viết hoa đầu câu sau dấu chấm và phím Enter
- Tạm tắt kiểm tra chính tả: nhấn đúp `Ctrl`
- Tạm tắt bộ gõ: nhấn `Alt`

### 🔧 Khắc phục lỗi ứng dụng

- Loại bỏ **trùng ký tự / nhảy chữ** trên thanh địa chỉ trình duyệt (Chrome, Edge, Firefox, Brave) và ô nhập Excel
- **Bản vá Chromium** đặc biệt (`Shift + Left Arrow`)
- Hỗ trợ ứng dụng **Metro / UWP Windows Store**
- Tự động **ngắt phiên gõ khi click chuột** (`WH_MOUSE_LL`)
- Quản lý backspace chính xác cho bảng mã 2-byte (VNI, Unicode tổ hợp)

### 📝 Gõ tắt (Macro)

- Tra cứu nhị phân siêu tốc (`BTreeMap`)
- **AutoCaps**: `btw`→`by the way`, `Btw`→`By the way`, `BTW`→`BY THE WAY`
- Hoạt động cả khi ở **chế độ tiếng Anh**
- Import / Export file `.txt` tương thích với các bộ gõ phổ biến
- Lưu trực tiếp trong Registry dưới dạng binary

### 🔄 Chuyển mã (Encoding Converter)

- Chuyển đổi qua lại giữa **tất cả 5 bảng mã**
- Hỗ trợ Clipboard: Plain Text, HTML (`CF_HTML`), Rich Text (`CF_RTF`)
- Tùy chọn: chữ HOA · chữ thường · bỏ dấu · hoa đầu câu · hoa mỗi từ
- **Phím tắt toàn cục** chuyển mã nhanh

### 🤖 Tự chuyển E/V theo ứng dụng (Smart Switch)

- Ghi nhớ ngôn ngữ và bảng mã riêng theo từng tiến trình
- Tự động chuyển chế độ khi đổi cửa sổ

### 🖥️ Giao diện & Hệ thống

| Tính năng | Chi tiết |
|-----------|----------|
| **Win32 thuần** | Common Controls v6, không GPU, không runtime |
| **Per-Monitor DPI V2** | Tự co giãn theo DPI từng màn hình |
| **On-demand windows** | Tạo khi mở, hủy khi đóng — không giữ cửa sổ nền |
| **Memory Trimming** | Tự thu hồi RAM (`EmptyWorkingSet`) khi đóng cửa sổ |
| **Tray icon động** | 🔴/🟠 Tiếng Việt · 🔵 Tiếng Anh |
| **Single Instance** | Lần thứ hai mở bảng điều khiển thay vì khởi động lại |
| **Khởi động cùng Windows** | Registry Run hoặc Task Scheduler (chế độ Admin) |

---

## 🚀 Hiệu năng

| Chỉ số | Giá trị |
|--------|---------|
| **RAM (chạy nền)** | ~2 – 3 MB |
| **CPU (idle)** | 0.0% |
| **Kích thước binary** | ~380 KB |
| **Input latency** | ≈ 0 ms |

Keyboard hook (`WH_KEYBOARD_LL`) chạy trên **dedicated OS thread** với `THREAD_PRIORITY_HIGHEST`, hoàn toàn tách biệt khỏi UI thread. Profile Release tối ưu cực đại: `opt-level = "z"`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip = true`.

---

## 🏗️ Kiến trúc

```
minkey/
├── src/
│   ├── main.rs              # Entry point, single-instance guard
│   ├── lib.rs               # Crate root, module exports
│   ├── types.rs             # Enums, constants, bitmasks
│   ├── engine.rs            # VietnameseEngine — lõi xử lý phím
│   ├── tables.rs            # Bảng mã Unicode/TCVN3/VNI/CP1258
│   ├── config.rs            # AppConfig + Registry I/O
│   ├── hook.rs              # WH_KEYBOARD_LL + WH_MOUSE_LL
│   ├── tray.rs              # System Tray + popup menu
│   ├── macro_engine.rs      # MacroTable — BTreeMap, import/export
│   ├── smart_switch.rs      # Per-app language memory
│   ├── convert.rs           # Encoding converter (5 bảng mã)
│   ├── settings.rs          # Settings logic
│   ├── app.rs               # Application state container
│   └── win32_ui/
│       ├── mod.rs           # Win32 message loop
│       ├── main_window.rs   # Bảng điều khiển chính
│       ├── macro_window.rs  # Cửa sổ quản lý gõ tắt
│       ├── convert_window.rs# Cửa sổ chuyển mã
│       └── controls.rs      # Custom Win32 controls
├── tests/                   # Automated test suite
├── res/
│   └── minkey.manifest      # Windows manifest (DPI, Common Controls v6)
├── build.rs                 # Nhúng manifest vào binary
└── Cargo.toml
```

### Luồng xử lý phím

```
[OS Keyboard Event]
        │
        ▼  WH_KEYBOARD_LL  (dedicated thread · THREAD_PRIORITY_HIGHEST)
    hook.rs
        │
        ▼  process_key_event()
    engine.rs  (VietnameseEngine)
        │
        ├──► macro_engine.rs   kiểm tra gõ tắt
        ├──► tables.rs         tra bảng mã
        └──► HookState { backspace_count, new_char_count, char_data[] }
                 │
                 ▼  SendInput()
          [Ứng dụng đích nhận ký tự tiếng Việt]
```

---

## 🛠️ Biên dịch

### Yêu cầu

| Thành phần | Phiên bản |
|------------|-----------|
| Hệ điều hành | Windows 10 / 11 (x64) |
| Rust toolchain | `stable` ≥ 1.80 · target `x86_64-pc-windows-msvc` |
| Build tools | Visual Studio Build Tools 2022 (MSVC linker) |

### Cài Rust (nếu chưa có)

```powershell
winget install Rustlang.Rustup
# Khởi động lại terminal, rồi kiểm tra:
rustc --version
cargo --version
```

### Lệnh build

```bash
# Chạy toàn bộ test suite
cargo test --release

# Build bản Release tối ưu
cargo build --release
# → target\release\minkey.exe  (~380 KB, không cần DLL, chạy thẳng)
```

---

## ⌨️ Phím tắt mặc định

| Phím tắt | Chức năng |
|----------|-----------|
| `Alt + Z` | Chuyển đổi Tiếng Anh / Tiếng Việt |
| Nhấn đúp `Ctrl` | Tạm tắt / bật kiểm tra chính tả |
| `Alt` (giữ) | Tạm tắt bộ gõ trong phiên hiện tại |
| `z` cuối từ | Xóa dấu |
| `[` | Gõ `ơ` |
| `]` | Gõ `ư` |

*Phím chuyển E/V có thể thay đổi trong Bảng điều khiển → Cài đặt.*

---

## 🗝️ Cấu hình Registry

Toàn bộ cài đặt lưu tại `HKEY_CURRENT_USER\Software\TuyenMai\OpenKey`.

<details>
<summary>Danh sách các key chính</summary>

| Key | Mô tả | Mặc định |
|-----|-------|----------|
| `vLanguage` | 0 = Anh · 1 = Việt | 1 |
| `vInputType` | 0 = Telex · 1 = VNI · 2 = ST1 · 3 = ST2 | 0 |
| `vCodeTable` | 0 = Unicode … 4 = CP1258 | 0 |
| `vCheckSpelling` | Kiểm tra chính tả | 1 |
| `vUseModernOrthography` | Dấu kiểu mới (`oà`, `uý`) | 0 |
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

```bash
cargo test                    # Chạy tất cả
cargo test telex              # Chỉ test Telex
cargo test -- --nocapture     # Xem output chi tiết
```

| File | Nội dung |
|------|----------|
| `telex_test.rs` | Kiểu gõ Telex |
| `vni_test.rs` | Kiểu gõ VNI |
| `simple_telex_test.rs` | Simple Telex 1 & 2 |
| `convert_test.rs` | Chuyển mã 5 bảng mã |
| `features_test.rs` | Gõ nhanh, phụ âm, xóa dấu |
| `macro_test.rs` | Import/Export, AutoCaps |
| `smart_switch_test.rs` | Per-app language memory |
| `settings_test.rs` | Load/Save config |
| `openkey_parity_test.rs` | Kiểm tra parity hành vi gõ |

---

## ❓ FAQ

<details>
<summary><strong>Minkey có xung đột với bộ gõ khác không?</strong></summary>

Không cần gỡ bộ gõ cũ, nhưng **không nên chạy đồng thời**. Hãy tắt bộ gõ cũ trước khi chạy Minkey.
</details>

<details>
<summary><strong>Tại sao không có file .dll hay runtime?</strong></summary>

Minkey biên dịch tĩnh hoàn toàn với Rust + MSVC — mọi thứ nằm trong một file `.exe` duy nhất.
</details>

<details>
<summary><strong>Bộ gõ không hoạt động trong phần mềm chạy quyền Admin?</strong></summary>

Windows giới hạn Low-Level Hook khi ứng dụng đích có quyền cao hơn bộ gõ. Vào **Cài đặt → Chạy với quyền quản trị** để khắc phục.
</details>

<details>
<summary><strong>Minkey hỗ trợ macOS không?</strong></summary>

Các module macOS còn trong source nhưng hiện chưa build. Phiên bản macOS sẽ được phát triển riêng trong tương lai.
</details>

---

## 🤝 Đóng góp

```bash
git clone https://github.com/liamhnam/minkey.git
cd minkey
git checkout -b feature/ten-tinh-nang
cargo test --release
git push origin feature/ten-tinh-nang
```

**Quy tắc commit**: `feat:` · `fix:` · `refactor:` · `test:` · `docs:`

---

## 📜 Giấy phép

**GNU General Public License v3.0** — xem file [LICENSE](LICENSE).

---

## 👤 Tác giả

**Hồ Hoàng Nam** · [github.com/liamhnam](https://github.com/liamhnam)
