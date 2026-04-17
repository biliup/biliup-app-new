# Biliup App - B站视频上传管理工具

![Version](https://img.shields.io/badge/version-1.2.0-blue)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)
![License-MIT](https://img.shields.io/badge/license-MIT-green)
![License-Apache2.0](https://img.shields.io/badge/license-Apache2.0-green)

一个基于 Tauri + Vue 3 的现代化 B站视频上传桌面应用，提供直观易用的界面和强大的批量管理功能。

## 已知问题
- 上传时如果持续不更新进度，删除任务，手动切换一下线路再上传

## 下载
[Release](https://github.com/biliup/biliup-app-new/releases)

##  核心功能

### 视频管理
- **拖拽上传** - 直接拖放视频文件到窗口即可添加
- **批量处理** - 支持多文件同时选择和批量配置
- **文件夹监控** - 监控指定的文件夹变化自动上传

### 模板系统  
- **可复用模板** - 为不同类型视频创建上传模板
- **一键重置** - 快速恢复到上次保存状态
- **从现有视频复制** - 根据bv号复制现有配置

### 编辑稿件
- **支持联合投稿的活动** - 选择活动页面可以直接看到是否支持联合投稿
- **联合投稿** - 可以通过uid或者昵称搜索好友进行联合投稿
- **简介@好友** - 可以通过输入@符号弹出用户列表
- **状态显示** - 可以直观地看到当前稿件状态
- **多稿件同步投稿** - 可以快速进行同一个模板下多个稿件的按序投稿（目前仅支持修改稿件标题）

### 多账号支持
- **同时管理** - 支持多个B站账号同时登录

## 快捷键

- **Ctrl + S** - 保存当前模板
- **Ctrl + R** - 重置模板到保存前状态
- **Ctrl + F5** - 刷新应用页面


## 系统要求

- **Windows**: Windows 10 及以上
- **macOS**: macOS 10.15 及以上  
- **Linux**: 现代 Linux 发行版

## 贡献指南

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

## 问题反馈

如果遇到问题，请通过以下方式反馈：

1. **GitHub Issues** - 提交详细的问题描述和复现步骤
2. **功能建议** - 欢迎提出新功能的建议和想法
3. **使用问题** - 详细描述遇到的问题和错误信息

## 许可证

本项目依据以下任一开源协议授权使用：

- ([LICENSE-APACHE](LICENSE-APACHE) 或 http://www.apache.org/licenses/LICENSE-2.0)
- ([LICENSE-MIT](LICENSE-MIT) 或 http://opensource.org/licenses/MIT)

您可自由选择其中之一进行使用。

## 致谢

- [Tauri](https://tauri.app/) - 跨平台桌面应用框架
- [Vue.js](https://vuejs.org/) - 渐进式 JavaScript 框架
- [Element Plus](https://element-plus.org/) - Vue 3 组件库
- [biliup](https://github.com/biliup/biliup)/[biliup-rs](https://github.com/biliup/biliup-rs) - B站上传核心库

---

**使用提醒**: 请遵守B站社区规则，仅用于上传合法合规内容。
