use anyhow::{bail, Result};
use std::sync::OnceLock;

static LANGUAGE: OnceLock<bool> = OnceLock::new();

pub fn initialize() -> Result<()> {
    let mut language = std::env::var("SSH_MGR_LANG").unwrap_or_else(|_| "en".into());
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--lang" {
            language = args
                .next()
                .ok_or_else(|| anyhow::anyhow!("--lang requires en or zh"))?;
        }
    }
    if language != "en" && language != "zh" {
        bail!("unsupported language: {language}; use en or zh");
    }
    let _ = LANGUAGE.set(language == "zh");
    Ok(())
}

pub fn text(value: &str) -> String {
    translate(value, *LANGUAGE.get_or_init(|| false))
}

fn translate(value: &str, chinese: bool) -> String {
    let pairs = [
        ("Reloaded", "已重新加载"),
        ("Reload failed", "重新加载失败"),
        ("SSH failed", "SSH 启动失败"),
        ("No server selected", "未选择服务器"),
        ("Servers shown", "服务器数量"),
        ("Servers", "服务器"),
        (" servers", " 台服务器"),
        (" matches", " 条匹配"),
        ("Search", "搜索"),
        ("Details", "详情"),
        ("Name", "名称"),
        ("Group", "分组"),
        ("Host", "主机"),
        ("User", "用户"),
        ("Auth", "认证"),
        ("Description", "描述"),
        ("Source", "来源"),
        ("State", "状态"),
        ("collapsed", "已折叠"),
        ("expanded", "已展开"),
    ];
    let mut output = value.to_string();
    if chinese {
        // Protect all values after a label and the active search query.
        output = output
            .lines()
            .map(|line| {
                if let Some((label, value)) = line.split_once(": ") {
                    if let Some((_, zh)) = pairs.iter().find(|(en, _)| *en == label) {
                        return format!("{zh}: {value}");
                    }
                }
                let (prefix, query) = line
                    .split_once("Search: ")
                    .map_or((line, None), |(prefix, query)| (prefix, Some(query)));
                let mut translated = prefix.to_string();
                for (en, zh) in pairs {
                    translated = translated.replace(en, zh);
                }
                match query {
                    Some(query) => format!("{translated}搜索: {query}"),
                    None => translated,
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        if value.starts_with(" ↑/↓") {
            output = output
                .replace("move", "移动")
                .replace("group", "分组")
                .replace("background", "后台打开")
                .replace("enter", "进入")
                .replace("search", "搜索")
                .replace("help", "帮助")
                .replace("quit", "退出");
        }
    } else {
        for (zh, en) in [
            ("确认搜索 / 进入 SSH", "Confirm filter / Open SSH"),
            ("折叠或展开分组", "Collapse / expand group"),
            ("后台打开 SSH", "Open SSH in background"),
            ("进入 SSH", "Open SSH"),
            ("清除搜索或返回", "Clear filter / Close help"),
            ("重新扫描配置", "Reload configuration"),
            ("显示快捷键", "Show shortcuts"),
            ("关闭帮助", "Close help"),
            ("移动", "Move"),
            ("搜索", "Search"),
            ("退出", "Quit"),
            ("操作", "Actions"),
            ("快捷键", "Shortcuts"),
            ("确认", "Confirm"),
        ] {
            output = output.replace(zh, en);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn translates_both_interfaces() {
        assert_eq!(translate("Details", true), "详情");
        assert_eq!(translate("Name: GroupHost", true), "名称: GroupHost");
        assert_eq!(translate("Search: Host", true), "搜索: Host");
        assert_eq!(translate("操作", false), "Actions");
        assert_eq!(translate("Search: prod", true), "搜索: prod");
        assert_eq!(translate("o  进入 SSH", false), "o  Open SSH");
    }
}
