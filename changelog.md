# 更新日志

## [1.4.0](https://github.com/biliup/biliup-app-new/releases/tag/app-v1.4.0) - 2026-10-15

**Full Changelog**: [app-v1.3.0...app-v1.4.0](https://github.com/biliup/biliup-app-new/compare/app-v1.3.0...app-v1.4.0)

### 修复

- 更新上传线路（[#51](https://github.com/biliup/biliup-app-new/issues/51)）。

- 合集相关内容改为调用上层biliup的api。

### 新增

- 新增中文、英文、日文界面切换，支持在顶部切换并自动保存（[#5](https://github.com/biliup/biliup-app-new/issues/5)）。（所有翻译均为gpt翻译）

- 新增上传风控安全验证：触发风控后弹出验证窗口，验证通过后继续上传（[#37](https://github.com/biliup/biliup-app-new/issues/37)）。

- 新增等待安全验证状态，同账号任务等待验证，其他账号可继续上传。

- 新增更多上传线路选项。

- 新增投稿后的合集配置进度提示。

### 已知问题

- macOS 下使用代理进行安全验证需要 macOS 14 或更高版本。

## [1.3.0](https://github.com/biliup/biliup-app-new/releases/tag/app-v1.3.0) - 2026-10-10

**Full Changelog**: [app-v1.2.4...app-v1.3.0](https://github.com/biliup/biliup-app-new/compare/app-v1.2.4...app-v1.3.0)

### 修复

- 修复新登录账号上传队列不自动刷新的问题。

- 修复暂停与恢复任务的调度问题，暂停后释放并行名额，恢复时重新排队。

- 修复上传速度显示，改为根据近期传输量计算。

- 修复文件夹监控检测重叠、停止后旧检测仍继续处理的问题。

- 修复文件夹监控添加失败后的重试逻辑，达到视频数量上限或遇到重复文件时可正常结束监控。

- 修复提交排队期间修改模板影响待提交内容的问题。

- 修复日志转发失败时递归调用的问题，并移除日志中的代理配置及手机号输出。

- 移除不支持的 WS、QN 上传线路，已有配置自动迁移为自动选择。

### 新增

- 新增后端上传停滞检测与自动重试，达到重试上限后保留失败任务供手动处理。

- 完善退出登录提示，明确清除范围为该账号的本地登录信息、设置及模板。

## [1.2.4](https://github.com/biliup/biliup-app-new/releases/tag/app-v1.2.4) - 2026-10-10

### 修复

- 修复 DevTools 控制台报错（[PR #59](https://github.com/biliup/biliup-app-new/pull/59)）。

- 修复描述粘贴后空行丢失问题（[#55](https://github.com/biliup/biliup-app-new/issues/55)）。

- 修复文件夹监控场景下视频被重复加入的问题。

- 修复新增稿件未配置合集时仍触发合集同步的问题。

- 修复上传队列批量操作反馈不准确的问题，现已区分成功、状态未变化、失败，并显示失败文件名。

### 新增

- 新增多稿件独立封面支持（[#56](https://github.com/biliup/biliup-app-new/issues/56)）。

- 新增模板级提交后自动打开编辑选项。

### 优化

- 重构上传任务批量接口（创建、开始、暂停、取消、重试）与前后端返回结构。

- 优化 MainView 提交流程与多稿件按钮状态文案/样式表现。

### 已知问题

- 上传时如果持续不更新进度，可先删除任务，手动切换线路后重传。

- 如最近遇到 App 打不开，多数是登录过期导致；该问题已在 1.0.4 中修复，如仍复现建议重新登录后再试。

## [1.2.3](https://github.com/biliup/biliup-app-new/releases/tag/app-v1.2.3) - 2026-06-03

**Full Changelog**: [app-v1.2.2...app-v1.2.3](https://github.com/biliup/biliup-app-new/compare/app-v1.2.2...app-v1.2.3)

### 新增

- （[#53](https://github.com/biliup/biliup-app-new/issues/53)）视频列表按标题名排序

### 修复

- 复制模板时不带视频列表

## [1.2.2](https://github.com/biliup/biliup-app-new/releases/tag/app-v1.2.2) - 2026-05-19

**Full Changelog**: [app-v1.2.1...app-v1.2.2](https://github.com/biliup/biliup-app-new/compare/app-v1.2.1...app-v1.2.2)

- 修复多稿件模式的一些bug

## [1.2.1](https://github.com/biliup/biliup-app-new/releases/tag/app-v1.2.1) - 2026-05-12

**Full Changelog**: [app-v1.2.0...app-v1.2.1](https://github.com/biliup/biliup-app-new/compare/app-v1.2.0...app-v1.2.1)

### 修复

- 修复日期选择器限制（[#36](https://github.com/biliup/biliup-app-new/issues/36)）。

- 修复登录后类型列表初始化时机问题（[#49](https://github.com/biliup/biliup-app-new/issues/49)）。

- 修复文件夹监控碰到被过滤的文件会误触提交的问题。

- 修复新增稿件未配置合集时仍触发合集同步的问题。

### 新增

- 新增新版分区列表的支持（[#50](https://github.com/biliup/biliup-app-new/issues/50)）。

- 新增文件夹监控触发多稿件提交流程。

- 新增提交统计页（支持查看与清空统计）。

- 新增视频列表统一前缀/后缀批量改名能力。

- 修复多稿件提交流程：支持取消队列、状态收敛与调度稳定性。

### 已知问题

- 上传时如果持续不更新进度，可先删除任务，手动切换线路后重传。

- 如最近遇到 App 打不开，多数是登录过期导致；该问题已在 1.0.4 中修复，如仍复现建议重新登录后再试。

## [1.2.0](https://github.com/biliup/biliup-app-new/releases/tag/app-v1.2.0) - 2026-04-17

**Full Changelog**: [app-v1.1.0...app-v1.2.0](https://github.com/biliup/biliup-app-new/compare/app-v1.1.0...app-v1.2.0)

### 修复

- 暂无。

### 新增

- 新增联合投稿

- 新增简介@功能

- 新增全景视频支持

### 已知问题

- 上传时如果持续不更新进度，删除任务，手动切换一下线路再上传

- 如最近遇到app打不开的问题，多数是因为登录过期，已在1.0.4版本中修复

## [1.1.0](https://github.com/biliup/biliup-app-new/releases/tag/app-v1.1.0) - 2026-04-13

**Full Changelog**: [app-v1.0.6...app-v1.1.0](https://github.com/biliup/biliup-app-new/compare/app-v1.0.6...app-v1.1.0)

### 修复

- 暂无。

### 新增

- 新增短信登录，请根据提示收集登录所需信息

### 优化

- 二维码可以主动刷新

### 已知问题

- 上传时如果持续不更新进度，删除任务，手动切换一下线路再上传

- 如最近遇到app打不开的问题，多数是因为登录过期，已在1.0.4版本中修复

## [1.0.6](https://github.com/biliup/biliup-app-new/releases/tag/app-v1.0.6) - 2026-04-13

**Full Changelog**: [app-v1.0.5...app-v1.0.6](https://github.com/biliup/biliup-app-new/compare/app-v1.0.5...app-v1.0.6)

### 修复

- 暂无。

### 新增

- 新增“以多稿件模式分别提交视频”：可将模板内每个视频按独立稿件逐条提交。

- 新增提交结果汇总弹窗。

### 优化

- 每次提交稿件到b站中间新增5秒间隔，防止提交过快。

- 仅在模板数量大于 1 时显示排序。

### 已知问题

- 上传时如果持续不更新进度，删除任务，手动切换一下线路再上传

- 如最近遇到app打不开的问题，多数是因为登录过期，已在1.0.4版本中修复

## [1.0.5](https://github.com/biliup/biliup-app-new/releases/tag/app-v1.0.5) - 2026-04-10

**Full Changelog**: [app-v1.0.4...app-v1.0.5](https://github.com/biliup/biliup-app-new/compare/app-v1.0.4...app-v1.0.5)

### 修复

- [#36](https://github.com/biliup/biliup-app-new/issues/36), [#46](https://github.com/biliup/biliup-app-new/issues/46)

### 新增

- 现在支持模板列表拖拽排序。

- 现在支持用户列表拖拽排序。

- 优化定时发布使用的“日期选择器”。

- 新增合集列表手动刷新按钮。

- 当选择了合集且当前为“新增稿件”时，新增“此稿件不产生更新推送”选项

### 已知问题

- 上传时如果持续不更新进度，删除任务，手动切换一下线路再上传

- 如最近遇到app打不开的问题，多数是因为登录过期，已在1.0.4版本中修复

## [1.0.4](https://github.com/biliup/biliup-app-new/releases/tag/app-v1.0.4) - 2026-03-12

**Full Changelog**: [app-v1.0.3...app-v1.0.4](https://github.com/biliup/biliup-app-new/compare/app-v1.0.3...app-v1.0.4)

### 修复

- [#30](https://github.com/biliup/biliup-app-new/issues/30), [#43](https://github.com/biliup/biliup-app-new/issues/43), [#44](https://github.com/biliup/biliup-app-new/issues/44)

### 新增

- 现在可以删除模板中的封面了（用以存储不带封面的模板）

### 已知问题

- 上传时如果持续不更新进度，删除任务，手动切换一下线路再上传

- 如最近遇到app打不开的问题，多数是因为登录过期，已在本次更新中修复

## [1.0.3](https://github.com/biliup/biliup-app-new/releases/tag/app-v1.0.3) - 2026-02-23

### 更新内容

- 由 [@z0d0dOr](https://github.com/z0d0dOr) 贡献 1 项修复和 1 项新增功能（[PR #42](https://github.com/biliup/biliup-app-new/pull/42)）。


**Full Changelog**: [app-v1.0.2...app-v1.0.3](https://github.com/biliup/biliup-app-new/compare/app-v1.0.2...app-v1.0.3)

## [1.0.2](https://github.com/biliup/biliup-app-new/releases/tag/app-v1.0.2) - 2025-10-11

**Full Changelog**: [app-v1.0.1...app-v1.0.2](https://github.com/biliup/biliup-app-new/compare/app-v1.0.1...app-v1.0.2)


## [1.0.1](https://github.com/biliup/biliup-app-new/releases/tag/app-v1.0.1) - 2025-09-29

**Full Changelog**: [app-v1.0.0...app-v1.0.1](https://github.com/biliup/biliup-app-new/compare/app-v1.0.0...app-v1.0.1)


## [1.0.0](https://github.com/biliup/biliup-app-new/releases/tag/app-v1.0.0) - 2025-09-18

**Full Changelog**: [app-v0.2.7...app-v1.0.0](https://github.com/biliup/biliup-app-new/compare/app-v0.2.7...app-v1.0.0)


## [0.2.7](https://github.com/biliup/biliup-app-new/releases/tag/app-v0.2.7) - 2025-09-05

**Full Changelog**: [app-v0.2.6...app-v0.2.7](https://github.com/biliup/biliup-app-new/compare/app-v0.2.6...app-v0.2.7)

- **_注意：对于所有追加的稿件，需要手动刷新一下并且保存模板，否则无法使用’合集‘功能_**

## [0.2.6](https://github.com/biliup/biliup-app-new/releases/tag/app-v0.2.6) - 2025-08-22

此版本未提供更新说明。

## [0.2.5](https://github.com/biliup/biliup-app-new/releases/tag/app-v0.2.5) - 2025-08-22

**Full Changelog**: [app-v0.2.4...app-v0.2.5](https://github.com/biliup/biliup-app-new/compare/app-v0.2.4...app-v0.2.5)


## [0.2.4](https://github.com/biliup/biliup-app-new/releases/tag/app-v0.2.4) - 2025-08-22

**Full Changelog**: [app-v0.2.3...app-v0.2.4](https://github.com/biliup/biliup-app-new/compare/app-v0.2.3...app-v0.2.4)


## [0.2.3](https://github.com/biliup/biliup-app-new/releases/tag/app-v0.2.3) - 2025-08-21

**Full Changelog**: [app-v0.2.2...app-v0.2.3](https://github.com/biliup/biliup-app-new/compare/app-v0.2.2...app-v0.2.3)


## [0.2.2](https://github.com/biliup/biliup-app-new/releases/tag/app-v0.2.2) - 2025-08-21

**Full Changelog**: [app-v0.2.1...app-v0.2.2](https://github.com/HsuJv/biliup-app-new/compare/app-v0.2.1...app-v0.2.2)


## [0.2.1](https://github.com/biliup/biliup-app-new/releases/tag/app-v0.2.1) - 2025-08-19

此版本未提供更新说明。

## [0.2.0](https://github.com/biliup/biliup-app-new/releases/tag/app-v0.2.0) - 2025-08-18

**Full Changelog**: [app-v0.1.7...app-v0.2.0](https://github.com/HsuJv/biliup-app-new/compare/app-v0.1.7...app-v0.2.0)


## [0.1.7](https://github.com/biliup/biliup-app-new/releases/tag/app-v0.1.7) - 2025-08-16

**Full Changelog**: [app-v0.1.6...app-v0.1.7](https://github.com/HsuJv/biliup-app-new/compare/app-v0.1.6...app-v0.1.7)


## [0.1.6](https://github.com/biliup/biliup-app-new/releases/tag/app-v0.1.6) - 2025-08-16

**Full Changelog**: [app-v0.1.5...app-v0.1.6](https://github.com/HsuJv/biliup-app-new/compare/app-v0.1.5...app-v0.1.6)


## [0.1.5](https://github.com/biliup/biliup-app-new/releases/tag/app-v0.1.5) - 2025-08-15

**Full Changelog**: [app-v0.1.2...app-v0.1.5](https://github.com/HsuJv/biliup-app-new/compare/app-v0.1.2...app-v0.1.5)


## [0.1.2](https://github.com/biliup/biliup-app-new/releases/tag/app-v0.1.2) - 2025-08-15

**Full Changelog**: [app-v0.1.0...app-v0.1.2](https://github.com/HsuJv/biliup-app-new/compare/app-v0.1.0...app-v0.1.2)

