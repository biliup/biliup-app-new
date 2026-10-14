use std::{
    collections::HashMap,
    sync::{
        LazyLock,
        atomic::{AtomicUsize, Ordering},
    },
};

// 新增语种时添加 JSON 字典并在此登记；中文为默认及缺词回退语言。
const SOURCES: &[(&str, &str)] = &[
    ("zh-CN", include_str!("i18n/zh-CN.json")),
    ("en-US", include_str!("i18n/en-US.json")),
];
static CATALOGS: LazyLock<Vec<(&str, HashMap<String, String>)>> = LazyLock::new(|| {
    SOURCES
        .iter()
        .map(|(language, source)| {
            (
                *language,
                serde_json::from_str(source).expect("内置语言字典格式错误"),
            )
        })
        .collect()
});
static LANGUAGE: AtomicUsize = AtomicUsize::new(0);

pub fn normalize_language(language: &str) -> &'static str {
    SOURCES
        .iter()
        .find(|(name, _)| *name == language)
        .map(|(name, _)| *name)
        .unwrap_or(SOURCES[0].0)
}

pub fn set_language(language: &str) {
    let index = SOURCES
        .iter()
        .position(|(name, _)| *name == language)
        .unwrap_or(0);
    LANGUAGE.store(index, Ordering::Relaxed);
}

fn lookup(language: usize, key: &'static str) -> &'static str {
    CATALOGS[language]
        .1
        .get(key)
        .or_else(|| CATALOGS[0].1.get(key))
        .map(String::as_str)
        .unwrap_or(key)
}

pub fn text(key: &'static str) -> &'static str {
    lookup(LANGUAGE.load(Ordering::Relaxed), key)
}

// 单次扫描模板，不处理插值内容中的大括号，API 原始消息不会被二次替换。
pub fn translate(key: &'static str, params: &[(&str, String)]) -> String {
    render(text(key), params)
}

fn render(template: &str, params: &[(&str, String)]) -> String {
    let mut output = String::with_capacity(template.len());
    let mut remaining = template;
    while let Some(start) = remaining.find('{') {
        let Some(end) = remaining[start..].find('}') else {
            break;
        };
        let end = start + end;
        output.push_str(&remaining[..start]);
        let name = &remaining[start + 1..end];
        if let Some((_, value)) = params.iter().find(|(param, _)| *param == name) {
            output.push_str(value);
        } else {
            output.push_str(&remaining[start..=end]);
        }
        remaining = &remaining[end + 1..];
    }
    output.push_str(remaining);
    output
}

#[macro_export]
macro_rules! tr {
    ($key:literal $(, $name:ident = $value:expr)* $(,)?) => {
        $crate::i18n::translate($key, &[$((stringify!($name), format!("{}", $value))),*])
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn catalogs_have_matching_keys_and_named_parameters() {
        fn placeholders(value: &str) -> std::collections::BTreeSet<&str> {
            value
                .split('{')
                .skip(1)
                .map(|part| {
                    let name = part.split_once('}').expect("占位符未闭合").0;
                    assert!(!name.is_empty());
                    assert!(name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
                    name
                })
                .collect()
        }

        let default = &super::CATALOGS[0].1;
        for (language, catalog) in super::CATALOGS.iter() {
            assert_eq!(catalog.len(), default.len(), "{language}");
            for (key, value) in default {
                let translated = catalog.get(key).expect("字典缺少文本键");
                assert_eq!(
                    placeholders(value),
                    placeholders(translated),
                    "{language}: {key}"
                );
            }
        }
    }

    #[test]
    fn raw_parameters_are_inserted_once_and_missing_values_are_preserved() {
        let raw = r#"{"message":"原始中文 {file} $&"}"#;
        let output = super::render(
            "{error} / {file} / {error} / {missing}",
            &[("error", raw.into()), ("file", "中文文件.mp4".into())],
        );
        assert_eq!(
            output,
            format!("{raw} / 中文文件.mp4 / {raw} / {{missing}}")
        );
        assert_eq!(super::render("未闭合 {error", &[]), "未闭合 {error");
    }

    #[test]
    fn unsupported_languages_and_unknown_keys_have_stable_fallbacks() {
        assert_eq!(super::normalize_language("en-US"), "en-US");
        assert_eq!(super::normalize_language("unsupported"), "zh-CN");
        assert_eq!(super::lookup(0, "missing.key"), "missing.key");
        assert_eq!(super::lookup(1, "missing.key"), "missing.key");
    }

    #[test]
    fn language_switch_preserves_external_error_details() {
        let raw = r#"{"code":-1,"message":"B站原始错误，中文内容保留"}"#;
        super::set_language("en-US");
        let error = crate::error::AppError::Biliup(raw.to_string());
        assert_eq!(error.to_string(), format!("Bilibili API error: {raw}"));
        assert_eq!(
            serde_json::to_value(&error).unwrap(),
            serde_json::Value::String(format!("Bilibili API error: {raw}"))
        );
        assert_eq!(
            crate::tr!("upload.task_canceled", task_id = raw),
            format!("Task canceled: {raw}")
        );
        super::set_language("zh-CN");
        assert_eq!(error.to_string(), format!("Bilibili API 错误: {raw}"));
        super::set_language("unsupported");
        assert_eq!(super::text("error.internal"), "内部错误");
    }
}
