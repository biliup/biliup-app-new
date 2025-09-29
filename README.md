# Biliup App - B站视频上传管理工具

![Version](https://img.shields.io/badge/version-1.0.1-blue)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)
![License-MIT](https://img.shields.io/badge/license-MIT-green)
![License-Apache2.0](https://img.shields.io/badge/license-Apache2.0-green)

一个基于 Tauri + Vue 3 的现代化 B站视频上传桌面应用，提供直观易用的界面和强大的批量管理功能。

## ✨ 核心功能

### 🎬 视频管理
- **多格式支持** - MP4、AVI、MOV、MKV、WMV、FLV、M4V、WEBM
- **拖拽上传** - 直接拖放视频文件到窗口即可添加
- **批量处理** - 支持多文件同时选择和批量配置

### 📝 模板系统  
- **可复用模板** - 为不同类型视频创建上传模板
- **一键重置** - 快速恢复到上次保存状态

### 📤 智能上传
- **队列管理** - 统一上传队列，支持暂停/继续/取消
- **进度监控** - 实时显示上传进度和传输速度  
- **自动重试** - 失败时自动重试，提高成功率
- **自动化流程** - 可选自动上传、自动开始、自动提交

### 🔐 多账号支持
- **同时管理** - 支持多个B站账号同时登录

## 🚀 快速开始

### 基本使用流程

1. **登录账号** - 点击登录按钮，使用B站账号登录
2. **创建模板** - 新建模板并配置视频基本信息
3. **添加视频** - 拖拽视频文件到窗口或点击选择文件
4. **配置属性** - 设置标题、简介、分区、标签等
5. **开始上传** - 加入队列并监控上传进度
6. **发布视频** - 上传完成后提交稿件

## ⌨️ 快捷键

- **Ctrl + S** - 保存当前模板
- **Ctrl + R** - 重置模板到保存前状态
- **Ctrl + F5** - 刷新应用页面
- **拖拽** - 添加视频文件

## 🎯 主要特色

### 文件夹监控  
- **自动检测** - 监控指定文件夹的新增视频
- **智能过滤** - 只处理支持的视频格式
- **批量导入** - 新文件自动加入处理队列

### 状态监控
- **转码状态** - 实时查看B站视频处理进度
- **详细日志** - 查看上传和处理的详细信息
- **错误提示** - 友好的错误信息和解决建议

## 🛠️ 高级功能

### 自动化设置
- **自动上传** - 添加文件后自动加入队列
- **自动开始** - 文件入队后自动开始上传  
- **自动提交** - 上传完成后自动发布稿件
- **自动编辑** - 新发布稿件后自动添加编辑模板
- **批量提交** - 可以同时对多个模块开启自动提交


## � 系统要求

- **Windows**: Windows 10 及以上
- **macOS**: macOS 10.15 及以上  
- **Linux**: 现代 Linux 发行版

## 🤝 贡献指南

欢迎提交 Issue 和 Pull Request！

### 开发准备
1. Fork 本仓库
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add some amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 提交 Pull Request

### 代码规范
- 提交前请运行 `npm run fmt` 格式化所有代码
- 遵循现有的代码结构和命名规范

## 🐛 问题反馈

如果遇到问题，请通过以下方式反馈：

1. **GitHub Issues** - 提交详细的问题描述和复现步骤
2. **功能建议** - 欢迎提出新功能的建议和想法
3. **使用问题** - 详细描述遇到的问题和错误信息

## 📄 许可证

本项目依据以下任一开源协议授权使用：

- ([LICENSE-APACHE](LICENSE-APACHE) 或 http://www.apache.org/licenses/LICENSE-2.0)
- ([LICENSE-MIT](LICENSE-MIT) 或 http://opensource.org/licenses/MIT)

您可自由选择其中之一进行使用。

## 🙏 致谢

- [Tauri](https://tauri.app/) - 跨平台桌面应用框架
- [Vue.js](https://vuejs.org/) - 渐进式 JavaScript 框架
- [Element Plus](https://element-plus.org/) - Vue 3 组件库
- [biliup](https://github.com/biliup/biliup)/[biliup-rs](https://github.com/biliup/biliup-rs) - B站上传核心库

---

**使用提醒**: 请遵守B站社区规则，仅用于上传合法合规内容。
