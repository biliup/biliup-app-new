# Rust 多语言字典

后端文本集中在本目录的语言 JSON 文件中，字典通过 `include_str!` 编译到程序内。

目前支持简体中文（`zh-CN`）、繁体中文（`zh-TW`）、英语（`en-US`）和日语（`ja-JP`）。

业务代码使用稳定的语义键：

`crate::i18n::text("auth.login_success")`

带插值的文本使用具名参数：

`crate::tr!("upload.task_failed", error = error)`

字典中的对应内容为 `"upload.task_failed": "上传任务失败: {error}"`。
参数按原样插入，不会再次翻译或替换，因此 B站 API 的消息及用户内容可以保留。

调试格式在调用处处理，例如 `video = format!("{:?}", video)`；
字典只使用 `{参数名}`，各语言可以调整参数的位置或重复使用参数。

新增语种：

1. 复制 `zh-CN.json` 为新的语言文件，保留所有键及具名参数并翻译值。
2. 在上级 `i18n.rs` 的 `SOURCES` 中登记语言代码和 `include_str!` 路径。
3. 完整的应用语言支持还需补充前端字典、可选语言和校验列表。
4. 运行 `cargo test --offline --lib`，校验字典键、参数及配置兼容性。

不支持的语言回退到中文；所选字典缺少某个键时查中文，中文也缺少时返回键名。
