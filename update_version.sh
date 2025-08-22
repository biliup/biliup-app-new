#!/bin/bash
#!/bin/bash

# 用法提示
if [[ -z "$1" ]]; then
  echo "❗ 用法: $0 <新版本号>"
  exit 1
fi

NEW_VERSION="$1"
echo "🚀 准备将 biliup-app 升级到版本：$NEW_VERSION"

# 确保在项目根目录
if [[ -d "src-tauri" && -f "package.json" ]]; then
  echo "📍 当前在项目根目录"
elif [[ -f "Cargo.toml" && -d "../src" ]]; then
  echo "📍 当前在 src-tauri 目录，回到项目根目录"
  cd ..
elif [[ -f "../package.json" && -d "../src-tauri" ]]; then
  echo "📍 回到项目根目录"
  cd ..
else
  echo "❌ 错误：无法找到项目根目录，请在 biliup-app 项目根目录下执行此脚本"
  exit 1
fi

# 编译检查函数
run_checks() {
  echo "🔍 开始编译检查..."
  
  echo "⚡ 执行代码格式化检查..."
  if ! npm run fmt 2>&1 ; then
    echo "❌ 代码格式化失败，请修复后再试"
    exit 1
  fi
  
  echo "🔨 执行前端构建检查..."
  if ! npm run build 2>&1 ; then
    echo "❌ 前端构建失败，请修复后再试"
    exit 1
  fi
  
  echo "🦀 执行后端构建检查..."
  if ! (cd src-tauri && cargo build --release 2>&1); then
    echo "❌ 后端构建失败，请修复后再试"
    exit 1
  fi
  
  echo "✅ 所有编译检查通过！"
}

# 执行编译检查
run_checks

# 自动从 package.json 中提取旧版本号
OLD_VERSION=$(grep '"version":' package.json | head -n 1 | sed -E 's/.*"version": "([^"]+)".*/\1/')
echo "🔍 检测到旧版本号：$OLD_VERSION"

# 更新 package.json 和 package-lock.json 中的 "name": "biliup-app" 块
update_json_version() {
  file="$1"
  tmpfile="${file}.tmp"

  awk -v old="$OLD_VERSION" -v new="$NEW_VERSION" '
    BEGIN { inBlock = 0 }
    /"name": "biliup-app"/ { inBlock = 1 }
    inBlock && /"version": "[0-9]+\.[0-9]+\.[0-9]+"/ {
      sub(/"version": "[0-9]+\.[0-9]+\.[0-9]+"/, "\"version\": \"" new "\"")
      inBlock = 0
    }
    { print }
  ' "$file" > "$tmpfile" && mv "$tmpfile" "$file" && echo "✅ 已更新 $file"
}

# 更新 Cargo.toml 中的 [package] 块
update_toml_version() {
  file="$1"
  tmpfile="${file}.tmp"

  awk -v new="$NEW_VERSION" '
    BEGIN { in_package = 0 }
    /^\[package\]/ { in_package = 1; print; next }
    in_package == 1 && /^version = "[0-9]+\.[0-9]+\.[0-9]+"/ {
      sub(/version = "[0-9]+\.[0-9]+\.[0-9]+"/, "version = \"" new "\"")
      in_package = 0
    }
    { print }
  ' "$file" > "$tmpfile" && mv "$tmpfile" "$file" && echo "✅ 已更新 $file"
}

# 更新 Cargo.lock 中的 [[package]] 块
update_lock_version() {
  file="$1"
  tmpfile="${file}.tmp"

  awk -v new="$NEW_VERSION" '
    BEGIN { in_block = 0 }
    /^\[\[package\]\]/ { in_block = 0 }
    /^name = "biliup-app"/ { in_block = 1; print; next }
    in_block == 1 && /^version = "[0-9]+\.[0-9]+\.[0-9]+"/ {
      sub(/version = "[0-9]+\.[0-9]+\.[0-9]+"/, "version = \"" new "\"")
      in_block = 0
    }
    { print }
  ' "$file" > "$tmpfile" && mv "$tmpfile" "$file" && echo "✅ 已更新 $file"
}

# 更新 tauri.conf.json 中的 "productName": "biliup-app" 块
update_tauri_conf() {
  file="$1"
  tmpfile="${file}.tmp"

  awk -v new="$NEW_VERSION" '
    BEGIN { inBlock = 0 }
    /"productName": "biliup-app"/ { inBlock = 1 }
    inBlock && /"version": "[0-9]+\.[0-9]+\.[0-9]+"/ {
      sub(/"version": "[0-9]+\.[0-9]+\.[0-9]+"/, "\"version\": \"" new "\"")
      inBlock = 0
    }
    { print }
  ' "$file" > "$tmpfile" && mv "$tmpfile" "$file" && echo "✅ 已更新 $file"
}

# 更新 README.md 中的版本号图标
update_readme_version() {
  file="$1"
  tmpfile="${file}.tmp"
  
  sed -E "s/version-[0-9]+\.[0-9]+\.[0-9]+-blue/version-$NEW_VERSION-blue/g" "$file" > "$tmpfile" && mv "$tmpfile" "$file" && echo "✅ 已更新 $file"
}

# 执行更新
update_json_version "package.json"
update_json_version "package-lock.json"
update_toml_version "src-tauri/Cargo.toml"
update_lock_version "src-tauri/Cargo.lock"
update_tauri_conf "src-tauri/tauri.conf.json"
update_readme_version "README.md"

# Git 提交
echo "📦 正在提交 Git 更改..."
git add package.json package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json README.md
git commit -m "🔖 v$NEW_VERSION"
git tag app-v$NEW_VERSION

echo "🎉 biliup-app 版本号已成功更新为 $NEW_VERSION 并提交到 Git！"


