#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$DIR"

echo "⚡ Đang biên dịch Minkey cho macOS (Release mode)..."
cargo build --release

APP_NAME="Minkey.app"
APP_DIR="$DIR/$APP_NAME"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

echo "📦 Đang đóng gói $APP_NAME..."
rm -rf "$APP_DIR"
mkdir -p "$MACOS_DIR"
mkdir -p "$RESOURCES_DIR"

# Copy binary thực thi
cp "target/release/minkey" "$MACOS_DIR/minkey"
chmod +x "$MACOS_DIR/minkey"

# Tạo Info.plist cho ứng dụng macOS
cat << 'EOF' > "$CONTENTS_DIR/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleDevelopmentRegion</key>
	<string>en</string>
	<key>CFBundleDisplayName</key>
	<string>Minkey</string>
	<key>CFBundleExecutable</key>
	<string>minkey</string>
	<key>CFBundleIconFile</key>
	<string>AppIcon</string>
	<key>CFBundleIdentifier</key>
	<string>org.minkey.Minkey</string>
	<key>CFBundleInfoDictionaryVersion</key>
	<string>6.0</string>
	<key>CFBundleName</key>
	<string>Minkey</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>CFBundleShortVersionString</key>
	<string>0.1.0</string>
	<key>CFBundleVersion</key>
	<string>1</string>
	<key>LSApplicationCategoryType</key>
	<string>public.app-category.utilities</string>
	<key>LSMinimumSystemVersion</key>
	<string>11.0</string>
	<key>LSUIElement</key>
	<true/>
	<key>NSPrincipalClass</key>
	<string>NSApplication</string>
	<key>NSHighResolutionCapable</key>
	<true/>
</dict>
</plist>
EOF

# Ký ad-hoc để macOS nhận diện ứng dụng ổn định khi cấp quyền Accessibility
codesign --force --deep --sign - "$APP_DIR"

echo "✅ Hoàn tất! Ứng dụng đã sẵn sàng tại:"
echo "👉 $APP_DIR"
echo ""
echo "Bạn có thể chạy thử bằng lệnh: open $APP_NAME"
echo "Lần chạy đầu: cấp quyền cho Minkey tại System Settings → Privacy & Security → Accessibility."
