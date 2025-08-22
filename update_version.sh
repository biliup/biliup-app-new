#!/bin/bash
#!/bin/bash

# 用法提示
if [[ -z "$1" ]]; then
  echo "❗ 用法: $0 <新版本号>"
  exit 1
fi

NEW_VERSION="$1"
echo "🚀 准备将 biliup-app 升级到版本：$NEW_VERSION"

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


