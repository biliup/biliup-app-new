# Biliup App

![Version](https://img.shields.io/badge/version-1.4.2-blue)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)
![License-MIT](https://img.shields.io/badge/license-MIT-green)
![License-Apache2.0](https://img.shields.io/badge/license-Apache2.0-green)

基于 Tauri 2、Vue 3 和 Rust 的 B 站视频上传桌面应用，使用 [biliup](https://github.com/biliup/biliup) 作为上传核心库，支持多账号、批量上传、投稿模板、稿件管理和合集管理。

[下载最新版本](https://github.com/biliup/biliup-app-new/releases/latest) · [更新日志](changelog.md) · [问题反馈](https://github.com/biliup/biliup-app-new/issues)

## 功能

- **视频上传**：拖拽或批量选择视频文件，管理上传队列，支持暂停、重试和切换上传线路。
- **投稿模板**：保存常用投稿配置，创建空白模板，或根据已有稿件创建编辑、复制模板。
- **文件夹监控**：监控指定文件夹，按配置自动添加视频并投稿。
- **稿件管理**：浏览账号已有稿件，快速创建编辑或复制模板，支持删除稿件及安全验证。
- **合集管理**：查看合集及小节，批量添加单集、移出稿件、修改单集标题和拖动排序。
- **处理进度**：查看稿件各分 P 的上传、转码及审核状态，以及各清晰度的转码进度。
- **投稿配置**：支持联合投稿、简介 @ 好友、独立封面，以及将模板中的视频按独立稿件依次提交。
- **多账号管理**：同时登录多个 B 站账号，分别管理设置和模板。
- **安全验证**：上传触发风控时弹出验证窗口，验证通过后继续上传。
- **多语言界面**：支持简体中文、繁体中文、英语和日语，切换后自动保存。翻译由 GPT 生成。
- **投稿统计与日志**：查看投稿统计，导出日志以便排查问题。

## 快速开始

### 下载与安装

前往 [Releases](https://github.com/biliup/biliup-app-new/releases)，在对应版本的 Assets 中选择适合操作系统和处理器架构的安装包。

| 系统 | 安装包 | 架构 |
| --- | --- | --- |
| Windows | `.exe` / `.msi` | x64、ARM64（实验性支持） |
| macOS | `.app` / `.dmg` | Apple Silicon、Intel |
| Linux | `.deb` / `.AppImage` | x64、ARM64 |

具体安装包以该版本实际发布的 Assets 为准。

### 第一次使用

1. 打开应用，通过用户列表添加账号，使用二维码或短信登录 B 站。
2. 选择账号，新建投稿模板；也可以从已有稿件创建编辑或复制模板。
3. 拖入视频文件或选择本地视频，填写标题、分区、标签、简介等信息。
4. 确认投稿配置后上传，并在上传队列中查看进度。
5. 在顶部的“稿件管理”或“合集管理”中管理已投稿的内容。

## 使用说明

### 模板与批量投稿

模板用于保存常用投稿配置，可通过 `Ctrl + S` 保存，通过 `Ctrl + R` 恢复到上次保存的状态。

- **编辑已有稿件**：新建编辑模板时可直接选择已有稿件，也可填写 BV/AV 号。编辑稿件时，分区、水印、杜比和无损音质等不可修改的设置会锁定。
- **复制已有稿件**：基于现有稿件的配置创建新模板，再添加本地视频进行投稿。
- **多稿件投稿**：将同一模板中的多个视频按独立稿件依次提交，目前仅支持修改各稿件标题。
- **文件夹监控**：为指定文件夹配置监控及目标模板，需要自动投稿时开启相应选项。

### 稿件与合集管理

在“稿件管理”中选择账号，浏览稿件并执行编辑、复制或删除操作。删除稿件时，按 B 站要求完成短信及验证码安全验证。

在“合集管理”中查看合集及小节，为小节批量添加单集、修改单集标题或调整顺序。拖动排序后需保存；“移出合集”会保留稿件本身。

稿件处理进度可用于查看上传后的转码、审核状态。上传完成后，稿件仍可能需要等待 B 站处理。

### 快捷键

| 快捷键 | 功能 |
| --- | --- |
| `Ctrl + S` | 保存当前模板 |
| `Ctrl + R` | 将当前模板恢复到上次保存的状态 |
| `Ctrl + F5` | 刷新应用页面 |

## 已知问题

- macOS 下通过代理进行安全验证需要 macOS 14 或更高版本。

## 开发

### 环境准备

- Node.js LTS 与 npm。
- Rust stable 工具链。
- 对应平台的 [Tauri 2 开发依赖](https://v2.tauri.app/start/prerequisites/)。

### 本地运行

```shell
git clone https://github.com/biliup/biliup-app-new.git
cd biliup-app-new
npm ci
npm run tauri dev
```

仅调试前端时可运行 `npm run dev`，开发地址为 `http://localhost:1520`；调用桌面端接口的功能需要在 Tauri 环境中运行。

### 构建与检查

```shell
# 构建前端（包含 TypeScript 检查）
npm run build

# 构建桌面应用及安装包
npm run tauri build

# 格式化前端和 Rust 代码
npm run fmt

# 检查 Rust 代码
cargo check --manifest-path src-tauri/Cargo.toml
```

### 目录结构

| 路径 | 说明 |
| --- | --- |
| `src/components` | 上传队列、模板、稿件与合集管理等界面组件 |
| `src/views` | 主工作区 |
| `src/stores` | 账号、上传任务和配置状态 |
| `src/composables` | 共享的 Vue 逻辑 |
| `src/i18n` | 前端语言资源 |
| `src-tauri/src/commands` | Tauri IPC 命令 |
| `src-tauri/src/services` | Rust 业务逻辑 |
| `src-tauri/src/models` | 数据模型 |
| `src-tauri/src/i18n` | 后端语言资源 |

## 贡献与反馈

欢迎通过 [Issues](https://github.com/biliup/biliup-app-new/issues) 提交问题和功能建议，或通过 Pull Request 贡献代码。

反馈问题时，请提供应用版本、操作系统、复现步骤和错误信息；可使用应用顶部的日志导出功能补充日志，分享前请检查并移除个人敏感信息。

提交代码前请运行 `npm run fmt`，遵循现有的代码结构和命名规范。提交信息使用英文 Conventional Commits 格式，例如 `fix: prevent startup crash`。

## 许可证

本项目采用 [MIT](LICENSE-MIT) 或 [Apache-2.0](LICENSE-APACHE) 双许可证，可任选其一使用。

请遵守 B 站社区规则，尊重内容版权。

## 致谢

- [biliup](https://github.com/biliup/biliup)
- [Tauri](https://tauri.app/)：跨平台桌面应用框架。
- [Vue.js](https://vuejs.org/)：前端框架。
- [Element Plus](https://element-plus.org/)：Vue 3 组件库。
