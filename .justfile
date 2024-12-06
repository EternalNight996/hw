# 项目配置
root_dir := justfile_directory()
os := os_family()
arch := arch()
scripts_dir := root_dir / "scripts" / os
project := "hw"
base_dir := root_dir / "target"
release_dir := base_dir / "release"
out_dir := release_dir / "out"
artifact_dir := release_dir / "artifact"
features := "built"

# 运行开发版本（带参数）
set positional-arguments

# 默认任务
default: clean init build zip-package

# 帮助信息
help:
    @just --list

# 清理构建目录
clean:
    @echo "=== Cleaning build directories ==="
    @if [ -d "{{out_dir}}" ]; then \
        just remove "{{out_dir}}" ; \
    fi
    @if [ -d "{{artifact_dir}}" ]; then \
        just remove "{{artifact_dir}}" ; \
    fi
# 显示信息
show:
    @echo ""
    @echo "=== Build Information ==="
    @echo "Project: {{project}}"
    @echo "Version: $(just get-version)"
    @echo "Date: $(just get-date)"
    @echo ""
    @echo "=== System Information ==="
    @echo "Platform: {{os}}"
    @echo "Architecture: {{arch}}"
    @echo "Root Directory: {{root_dir}}"
    @echo ""

# 初始化目录
init: clean
    @echo "=== Initializing directories ==="
    just show
    mkdir -p "{{base_dir}}"
    mkdir -p "{{release_dir}}"
    mkdir -p "{{out_dir}}"
    mkdir -p "{{artifact_dir}}"

# 构建主程序
build:
    @echo "=== Building release version ==="
    cargo build --release --features {{features}}
    @echo "=== Copying and compressing executable ==="
    just _copy-exe
    just _compress-exe
    just _copy-resources
    # 打开输出目录
    just open "{{out_dir}}"


# ZIP打包项目
zip-package:
    @echo "=== Creating release package ==="
    @if [ "{{os}}" = "windows" ]; then \
        powershell -NoProfile -Command "Compress-Archive -Force -Path '{{out_dir}}/*' -DestinationPath '{{artifact_dir}}/$(just get-archive-name)'" ; \
    else \
        zip -r "{{artifact_dir}}/$(just get-archive-name)" "{{out_dir}}"/* ; \
    fi
    @echo "=== Build completed successfully! ==="
    @echo "Release package: $(just get-archive-name)"
    just open "{{artifact_dir}}"

# 运行测试
test:
    @echo "=== Running tests ==="
    cargo test


# 运行开发版本（带参数）
@dev *args='':
    @echo "=== Running development version ==="
    @echo "Running: cargo run -- $@"
    cargo run -- $@

# 运行发布版本
release: init
    @echo "=== Running release version ==="
    cargo run --release

# 检查代码
check:
    @echo "=== Checking code ==="
    cargo check
    cargo clippy
    cargo fmt -- --check

# 格式化代码
format:
    @echo "=== Formatting code ==="
    cargo fmt

# 获取日期
get-date:
    @date +%Y%m%d
# 跨平台打开目录
open path:
    @if [ "{{os}}" = "windows" ]; then \
        powershell -NoProfile -Command "Start-Process '{{path}}'" ; \
    elif [ "{{os}}" = "macos" ]; then \
        open "{{path}}" ; \
    else \
        xdg-open "{{path}}" ; \
    fi
# 获取文件名
get-archive-name:
    @echo "{{project}}-v$(just get-version)-$(just get-date)-{{os}}-{{arch}}.zip"

# 版本和日期
get-version:
  @cargo pkgid | cut -d# -f2 | tr -d '\r\n'

# 复制可执行文件
_copy-exe:
    @if [ "{{os}}" = "windows" ]; then \
        just copy "{{release_dir}}/{{project}}.exe" "{{out_dir}}/" ; \
    else \
        just copy "{{release_dir}}/{{project}}" "{{out_dir}}/" ; \
    fi

# UPX压缩
_compress-exe:
    @if [ -f "{{scripts_dir}}/upx.exe" ]; then \
        "{{scripts_dir}}/upx.exe" --best --lzma "{{out_dir}}/{{project}}.exe"; \
    fi

# 复制资源文件
_copy-resources:
    @if [ -d "plugins" ]; then \
        just copy "plugins" "{{out_dir}}/" ; \
    fi
    @if [ -d "assets" ]; then \
        just copy "assets" "{{out_dir}}/" ; \
    fi
    @if [ -f "app.db" ]; then \
        just copy "app.db" "{{out_dir}}/" ; \
    fi
    @if [ -f "LICENSE" ]; then \
        just copy "LICENSE" "{{out_dir}}/" ; \
    fi
    @if [ -f "readme.md" ]; then \
        just copy "readme.md" "{{out_dir}}/" ; \
    fi
    @if [ "{{os}}" = "windows" ] && [ -f "{{scripts_dir}}/e-app-fileinfo.exe" ]; then \
        "{{scripts_dir}}/e-app-fileinfo.exe" --api fileinfo --task copy-lib --args "{{out_dir}}/{{project}}.exe" "{{out_dir}}" ; \
    fi

# 跨平台复制
copy src dst:
    @if [ "{{os}}" = "windows" ]; then \
        powershell -NoProfile -Command "\
            if (Test-Path '{{src}}' -PathType Container) { \
                Copy-Item -Recurse -Force '{{src}}' '{{dst}}' \
            } else { \
                Copy-Item -Force '{{src}}' '{{dst}}' \
            }" ; \
    else \
        if [ -d "{{src}}" ]; then \
            cp -r "{{src}}" "{{dst}}" ; \
        else \
            cp "{{src}}" "{{dst}}" ; \
        fi \
    fi

# 跨平台删除
remove src:
    @if [ "{{os}}" = "windows" ]; then \
        powershell -NoProfile -Command "\
            if (Test-Path '{{src}}' -PathType Container) { \
                Remove-Item -Recurse -Force '{{src}}' \
            } else { \
                Remove-Item -Force '{{src}}' \
            }" ; \
    else \
        if [ -d "{{src}}" ]; then \
            rm -rf "{{src}}" ; \
        else \
            rm -f "{{src}}" ; \
        fi \
    fi