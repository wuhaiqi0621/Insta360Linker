#!/bin/sh
set -eu

PROJECT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
APP_NAME="Luna Studio"
APP_DIR="$PROJECT_DIR/dist/$APP_NAME.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"
FFMPEG_BINARY="$PROJECT_DIR/assets/ffmpeg/ffmpeg"

if [ ! -x "$FFMPEG_BINARY" ]; then
    case "$(uname -m)" in
        arm64) FFMPEG_ARCH="arm64" ;;
        x86_64) FFMPEG_ARCH="amd64" ;;
        *)
            echo "Unsupported macOS architecture: $(uname -m)" >&2
            exit 1
            ;;
    esac

    FFMPEG_URL="https://ffmpeg.martin-riedl.de/redirect/latest/macos/$FFMPEG_ARCH/release/ffmpeg.zip"
    FFMPEG_ARCHIVE="${TMPDIR:-/tmp}/luna-studio-ffmpeg.zip"
    mkdir -p "$PROJECT_DIR/assets/ffmpeg"
    curl -fL --retry 5 --retry-delay 2 "$FFMPEG_URL" -o "$FFMPEG_ARCHIVE"
    unzip -o "$FFMPEG_ARCHIVE" -d "$PROJECT_DIR/assets/ffmpeg"
    chmod +x "$FFMPEG_BINARY"
fi

cd "$PROJECT_DIR"
cargo build --release --bin html_app

mkdir -p "$MACOS_DIR" "$RESOURCES_DIR/ffmpeg"
cp "$PROJECT_DIR/target/release/html_app" "$MACOS_DIR/$APP_NAME"
cp "$PROJECT_DIR/macos/Info.plist" "$CONTENTS_DIR/Info.plist"

cp "$FFMPEG_BINARY" "$RESOURCES_DIR/ffmpeg/ffmpeg"

chmod +x "$MACOS_DIR/$APP_NAME"
codesign --force --deep --sign - "$APP_DIR"

echo "$APP_DIR"
