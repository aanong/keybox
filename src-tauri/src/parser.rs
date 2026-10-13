//! Smart parser: turns free-form pasted text into account entries.
//!
//! Strategy: locate field keywords (English and Chinese), and treat everything
//! between a keyword and the next keyword (or end of line) as its value. This
//! keeps passwords containing symbols, spaces or CJK characters intact.
//! Text without keywords is classified heuristically (URLs, hosts, host:port,
//! connection strings, and positional username/password).

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::models::EntryData;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Field {
    Title,
    Host,
    Port,
    User,
    Password,
    Notes,
}

const EN_KEYWORDS: &[(&str, Field)] = &[
    ("hostname", Field::Host),
    ("address", Field::Host),
    ("server", Field::Host),
    ("domain", Field::Host),
    ("host", Field::Host),
    ("addr", Field::Host),
    ("url", Field::Host),
    ("ip", Field::Host),
    ("port", Field::Port),
    ("username", Field::User),
    ("account", Field::User),
    ("login", Field::User),
    ("email", Field::User),
    ("uid", Field::User),
    ("user", Field::User),
    ("password", Field::Password),
    ("passwd", Field::Password),
    ("secret", Field::Password),
    ("pass", Field::Password),
    ("pwd", Field::Password),
    ("title", Field::Title),
    ("notes", Field::Notes),
    ("note", Field::Notes),
    ("remark", Field::Notes),
];

const ZH_KEYWORDS: &[(&str, Field)] = &[
    ("服务器地址", Field::Host),
    ("登录地址", Field::Host),
    ("访问地址", Field::Host),
    ("网址", Field::Host),
    ("地址", Field::Host),
    ("主机", Field::Host),
    ("域名", Field::Host),
    ("链接", Field::Host),
    ("端口号", Field::Port),
    ("端口", Field::Port),
    ("登录账号", Field::User),
    ("用户名", Field::User),
    ("账号", Field::User),
    ("帐号", Field::User),
    ("账户", Field::User),
    ("帐户", Field::User),
    ("用户", Field::User),
    ("邮箱", Field::User),
    ("登录密码", Field::Password),
    ("密码", Field::Password),
    ("口令", Field::Password),
    ("名称", Field::Title),
    ("标题", Field::Title),
    ("备注", Field::Notes),
    ("说明", Field::Notes),
];

static KEYWORD_RE: LazyLock<Regex> = LazyLock::new(|| {
    let alt = |list: &[(&str, Field)]| {
        let mut words: Vec<&str> = list.iter().map(|(w, _)| *w).collect();
        words.sort_by_key(|w| std::cmp::Reverse(w.chars().count()));
        words.iter().map(|w| regex::escape(w)).collect::<Vec<_>>().join("|")
    };
    // English keywords must start at a boundary; Chinese ones may follow any char.
    // Both must be followed by a separator (colon / equals / whitespace).
    Regex::new(&format!(
        r"(?im)(?:(?:^|[\s,;，；|、(（\[【])(?P<en>{})|(?P<zh>{}))[ \t\u3000]*(?:[:：=][ \t\u3000]*|[ \t\u3000]+)",
        alt(EN_KEYWORDS),
        alt(ZH_KEYWORDS)
    ))
    .unwrap()
});

static SCHEME_URL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?i)([a-z][a-z0-9+.\-]*)://(\S+)$").unwrap());
static HOST_PORT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([A-Za-z0-9](?:[A-Za-z0-9.\-]*[A-Za-z0-9])?|\[[0-9A-Fa-f:]+\]):(\d{1,5})$").unwrap());
static IPV4_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:\d{1,3}\.){3}\d{1,3}$").unwrap());
static DOMAIN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:[A-Za-z0-9](?:[A-Za-z0-9\-]*[A-Za-z0-9])?\.)+[A-Za-z]{2,}(?:/\S*)?$").unwrap());
static DB_HINT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(cdb|rds|mysql|maria|postgres|pgsql|redis|mongo|sqlserver|oracle|clickhouse|elastic|tencentcdb|polardb|\bdb\b|\.sql\.|database)").unwrap()
});

const DB_PORTS: &[u16] = &[3306, 3307, 5432, 6379, 27017, 1433, 1521, 9200, 8123, 9000, 5672, 11211, 2181, 9092];
const DB_SCHEMES: &[&str] = &[
    "mysql", "mariadb", "postgres", "postgresql", "redis", "rediss", "mongodb", "mongodb+srv", "sqlserver", "jdbc",
    "clickhouse", "amqp",
];
const SERVER_SCHEMES: &[&str] = &["ssh", "sftp", "ftp", "rdp", "vnc", "telnet"];

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ParsedEntry {
    #[serde(flatten)]
    pub data: EntryData,
    /// Which fields were recognized (for UI hints).
    pub recognized: Vec<&'static str>,
}

#[derive(Default)]
struct Draft {
    title: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    scheme: Option<String>,
    user: Option<String>,
    password: Option<String>,
    notes: Vec<String>,
    leftovers: Vec<String>,
}

impl Draft {
    fn is_empty(&self) -> bool {
        self.host.is_none() && self.user.is_none() && self.password.is_none() && self.title.is_none()
    }
}

/// Parse one or more entries. Entries are separated by blank lines; if every
/// non-blank line is a self-contained entry, each line becomes its own entry.
pub fn parse(text: &str) -> Vec<ParsedEntry> {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let blocks: Vec<String> = split_blocks(&text);
    blocks
        .iter()
        .filter_map(|b| parse_block(b))
        .collect()
}

fn split_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut cur: Vec<&str> = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            if !cur.is_empty() {
                blocks.push(cur.join("\n"));
                cur.clear();
            }
        } else {
            cur.push(line);
        }
    }
    if !cur.is_empty() {
        blocks.push(cur.join("\n"));
    }
    blocks
}

fn field_of(m: &regex::Captures) -> Field {
    if let Some(en) = m.name("en") {
        let w = en.as_str().to_lowercase();
        EN_KEYWORDS.iter().find(|(k, _)| *k == w).map(|(_, f)| *f).unwrap()
    } else {
        let w = m.name("zh").unwrap().as_str();
        ZH_KEYWORDS.iter().find(|(k, _)| *k == w).map(|(_, f)| *f).unwrap()
    }
}

struct Mark {
    start: usize,
    end: usize,
    field: Field,
}

/// Finds keyword labels in a block.
/// - Chinese labels absorb preceding Han characters ("数据库账号" is one label).
/// - A repeated field on the same line is treated as part of the previous
///   value (e.g. a password that itself contains "密码").
fn keyword_marks(block: &str) -> Vec<Mark> {
    let mut marks: Vec<Mark> = Vec::new();
    let mut seen_line: Option<usize> = None;
    let mut seen_fields: Vec<Field> = Vec::new();
    for c in KEYWORD_RE.captures_iter(block) {
        let field = field_of(&c);
        let whole = c.get(0).unwrap();
        let kw = c.name("en").or(c.name("zh")).unwrap();
        let mut start = if c.name("en").is_some() { kw.start() } else { whole.start() };
        if c.name("zh").is_some() {
            let floor = marks.last().map(|m| m.end).unwrap_or(0);
            for (i, ch) in block[floor..start].char_indices().rev() {
                if is_han(ch) {
                    start = floor + i;
                } else {
                    break;
                }
            }
        }
        let line = block[..start].matches('\n').count();
        if seen_line != Some(line) {
            seen_line = Some(line);
            seen_fields.clear();
        }
        if seen_fields.contains(&field) {
            continue;
        }
        seen_fields.push(field);
        marks.push(Mark { start, end: whole.end(), field });
    }
    marks
}

fn is_han(ch: char) -> bool {
    matches!(ch as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0xF900..=0xFAFF)
}

fn parse_block(block: &str) -> Option<ParsedEntry> {
    let mut d = Draft::default();
    let mut unlabeled: Vec<String> = Vec::new();

    let marks = keyword_marks(block);
    let first_start = marks.first().map(|m| m.start).unwrap_or(block.len());
    unlabeled.push(block[..first_start].to_string());

    for (i, m) in marks.iter().enumerate() {
        let field = m.field;
        let start = m.end;
        let end = marks.get(i + 1).map(|n| n.start).unwrap_or(block.len());
        let raw = &block[start..end];
        // A value never spans lines; the rest of the line goes back to unlabeled text.
        let (value, rest) = match raw.find('\n') {
            Some(p) => (&raw[..p], &raw[p + 1..]),
            None => (raw, ""),
        };
        unlabeled.push(rest.to_string());
        assign(&mut d, field, value);
    }

    for chunk in &unlabeled {
        for line in chunk.lines() {
            classify_unlabeled_line(&mut d, line);
        }
    }

    // Positional fallback for leftovers: first → username, second → password.
    let mut leftovers = std::mem::take(&mut d.leftovers).into_iter().peekable();
    if d.user.is_none() && d.password.is_none() && leftovers.len() >= 2 {
        d.user = leftovers.next();
        d.password = leftovers.next();
    } else if d.user.is_some() && d.password.is_none() && leftovers.peek().is_some() {
        d.password = leftovers.next();
    } else if d.user.is_none() && d.password.is_some() && leftovers.peek().is_some() {
        d.user = leftovers.next();
    }
    let rest: Vec<String> = leftovers.collect();
    if !rest.is_empty() {
        if d.title.is_none() {
            d.title = Some(rest.join(" "));
        } else {
            d.notes.push(rest.join(" "));
        }
    }

    if d.is_empty() {
        return None;
    }
    Some(finish(d))
}

fn clean(value: &str, field: Field) -> String {
    let v = value.trim_matches(|c: char| c.is_whitespace());
    if field == Field::Password {
        return v.to_string();
    }
    v.trim_end_matches([',', '，', ';', '；', '|', '、'])
        .trim_matches(['"', '\'', '“', '”', '「', '」'])
        .trim()
        .to_string()
}

fn assign(d: &mut Draft, field: Field, value: &str) {
    let v = clean(value, field);
    if v.is_empty() {
        return;
    }
    match field {
        Field::Title => d.title = d.title.take().or(Some(v)),
        Field::Host => {
            if d.host.is_some() {
                d.notes.push(v);
            } else {
                apply_host_token(d, &v);
            }
        }
        Field::Port => match v.parse::<u16>() {
            Ok(p) if p > 0 => d.port = Some(p),
            _ => d.notes.push(format!("端口: {v}")),
        },
        Field::User => {
            if d.user.is_none() {
                d.user = Some(v);
            } else {
                d.notes.push(v);
            }
        }
        Field::Password => {
            if d.password.is_none() {
                d.password = Some(v);
            } else {
                d.notes.push(format!("密码2: {v}"));
            }
        }
        Field::Notes => d.notes.push(v),
    }
}

/// Interprets a host-like token: URL, connection string, host:port, plain host.
fn apply_host_token(d: &mut Draft, token: &str) {
    if let Some(c) = SCHEME_URL_RE.captures(token) {
        let scheme = c[1].to_lowercase();
        let rest = &c[2];
        if scheme == "http" || scheme == "https" {
            // Strip credentials from web URLs but keep the URL itself.
            let (cred, after) = split_userinfo(rest);
            if let Some((u, p)) = cred {
                d.user = d.user.take().or(u);
                d.password = d.password.take().or(p);
            }
            d.host = Some(format!("{scheme}://{after}"));
            d.scheme = Some(scheme);
            return;
        }
        let (cred, after) = split_userinfo(rest);
        if let Some((u, p)) = cred {
            d.user = d.user.take().or(u);
            d.password = d.password.take().or(p);
        }
        let hostport = after.split(['/', '?', '#']).next().unwrap_or("");
        let (h, p) = split_host_port(hostport);
        d.host = Some(h.to_string());
        if let Some(p) = p {
            d.port = d.port.or(Some(p));
        }
        if let Some(path) = after.find('/').map(|i| &after[i + 1..]).filter(|s| !s.is_empty()) {
            d.notes.push(format!("库/路径: {path}"));
        }
        d.scheme = Some(scheme);
        return;
    }
    let (h, p) = split_host_port(token);
    d.host = Some(h.to_string());
    if let Some(p) = p {
        d.port = d.port.or(Some(p));
    }
}

type Credentials = (Option<String>, Option<String>);

/// Splits "user:pass@host..." at the LAST '@', so '@' inside passwords is kept.
fn split_userinfo(s: &str) -> (Option<Credentials>, String) {
    match s.rfind('@') {
        Some(at) => {
            let info = &s[..at];
            let (u, p) = match info.find(':') {
                Some(i) => (&info[..i], Some(&info[i + 1..])),
                None => (info, None),
            };
            let u = (!u.is_empty()).then(|| u.to_string());
            let p = p.filter(|p| !p.is_empty()).map(str::to_string);
            (Some((u, p)), s[at + 1..].to_string())
        }
        None => (None, s.to_string()),
    }
}

fn split_host_port(s: &str) -> (&str, Option<u16>) {
    if let Some(c) = HOST_PORT_RE.captures(s)
        && let Ok(p) = c[2].parse::<u16>()
    {
        return (c.get(1).unwrap().as_str(), Some(p));
    }
    (s, None)
}

fn looks_like_host(token: &str) -> bool {
    SCHEME_URL_RE.is_match(token)
        || HOST_PORT_RE.is_match(token)
        || IPV4_RE.is_match(token)
        || DOMAIN_RE.is_match(token)
        || token.eq_ignore_ascii_case("localhost")
}

fn classify_unlabeled_line(d: &mut Draft, line: &str) {
    let line = line.trim();
    if line.is_empty() {
        return;
    }
    let tokens: Vec<&str> = line
        .split(|c: char| c.is_whitespace() || c == ',' || c == '，' || c == ';' || c == '；' || c == '|')
        .filter(|t| !t.is_empty())
        .collect();

    for t in tokens {
        if d.host.is_none() && looks_like_host(t) {
            apply_host_token(d, t);
        } else if d.port.is_none() && d.host.is_some() && t.starts_with(':') && t[1..].parse::<u16>().is_ok() {
            d.port = t[1..].parse().ok();
        } else {
            d.leftovers.push(t.to_string());
        }
    }
}

fn infer_category(d: &Draft) -> &'static str {
    if let Some(s) = d.scheme.as_deref() {
        if DB_SCHEMES.contains(&s) {
            return "database";
        }
        if SERVER_SCHEMES.contains(&s) {
            return "server";
        }
        if s == "http" || s == "https" {
            return "website";
        }
    }
    let host = d.host.as_deref().unwrap_or("");
    if d.port.is_some_and(|p| DB_PORTS.contains(&p)) || (!host.is_empty() && DB_HINT_RE.is_match(host)) {
        return "database";
    }
    if d.port.is_some_and(|p| p == 22 || p == 3389) || IPV4_RE.is_match(host) {
        return "server";
    }
    if !host.is_empty() {
        return "website";
    }
    "other"
}

fn infer_title(d: &Draft) -> String {
    let host = d.host.as_deref().unwrap_or("");
    if !host.is_empty() {
        let bare = host.split("://").last().unwrap_or(host);
        let bare = bare.split(['/', ':', '?']).next().unwrap_or(bare);
        if IPV4_RE.is_match(bare) {
            return bare.to_string();
        }
        let labels: Vec<&str> = bare.split('.').filter(|s| !s.is_empty()).collect();
        let known = [
            ("tencentcdb", "腾讯云数据库"),
            ("myqcloud", "腾讯云"),
            ("tencentcloud", "腾讯云"),
            ("aliyuncs", "阿里云"),
            ("aliyun", "阿里云"),
            ("amazonaws", "AWS"),
            ("azure", "Azure"),
            ("huaweicloud", "华为云"),
            ("myhuaweicloud", "华为云"),
            ("volces", "火山引擎"),
            ("baidubce", "百度云"),
        ];
        for (k, name) in known {
            if labels.iter().any(|l| l.eq_ignore_ascii_case(k)) {
                return name.to_string();
            }
        }
        if labels.len() >= 2 {
            return labels[labels.len() - 2..].join(".");
        }
        return bare.to_string();
    }
    d.user.clone().unwrap_or_default()
}

fn finish(d: Draft) -> ParsedEntry {
    let mut recognized = Vec::new();
    if d.host.is_some() {
        recognized.push("host");
    }
    if d.port.is_some() {
        recognized.push("port");
    }
    if d.user.is_some() {
        recognized.push("username");
    }
    if d.password.is_some() {
        recognized.push("password");
    }
    let category = infer_category(&d).to_string();
    let title = d.title.clone().unwrap_or_else(|| infer_title(&d));
    ParsedEntry {
        data: EntryData {
            title,
            category,
            host: d.host.unwrap_or_default(),
            port: d.port,
            username: d.user.unwrap_or_default(),
            password: d.password.unwrap_or_default(),
            notes: d.notes.join("\n"),
            tags: Vec::new(),
            favorite: false,
        },
        recognized,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(text: &str) -> EntryData {
        let v = parse(text);
        assert_eq!(v.len(), 1, "expected one entry from {text:?}, got {v:#?}");
        v[0].data.clone()
    }

    #[test]
    fn user_example() {
        let e = one("gz-cdb-example.sql.tencentcdb.com        port:25729       user root       password Demo.456!@#$%^&*()");
        assert_eq!(e.host, "gz-cdb-example.sql.tencentcdb.com");
        assert_eq!(e.port, Some(25729));
        assert_eq!(e.username, "root");
        assert_eq!(e.password, "Demo.456!@#$%^&*()");
        assert_eq!(e.category, "database");
        assert_eq!(e.title, "腾讯云数据库");
    }

    #[test]
    fn user_example_with_escaped_star() {
        let e = one(r"gz-cdb-example.sql.tencentcdb.com port:25729 user root password Demo.456!@#$%^&\*()");
        assert_eq!(e.password, r"Demo.456!@#$%^&\*()");
    }

    #[test]
    fn chinese_labels_and_values() {
        let e = one("网址：https://mail.qq.com\n账号：张三@qq.com\n密码：我的密码 带空格!@#");
        assert_eq!(e.host, "https://mail.qq.com");
        assert_eq!(e.username, "张三@qq.com");
        assert_eq!(e.password, "我的密码 带空格!@#");
        assert_eq!(e.category, "website");
    }

    #[test]
    fn chinese_label_without_leading_space() {
        let e = one("测试数据库地址:10.0.0.8 端口:3306 数据库账号：管理员 数据库密码：密码123");
        assert_eq!(e.host, "10.0.0.8");
        assert_eq!(e.port, Some(3306));
        assert_eq!(e.username, "管理员");
        assert_eq!(e.password, "密码123");
        assert_eq!(e.category, "database");
    }

    #[test]
    fn host_port_combined() {
        let e = one("192.168.1.10:22 user=ubuntu pwd=P@ss word");
        assert_eq!(e.host, "192.168.1.10");
        assert_eq!(e.port, Some(22));
        assert_eq!(e.username, "ubuntu");
        assert_eq!(e.password, "P@ss word");
        assert_eq!(e.category, "server");
    }

    #[test]
    fn connection_string() {
        let e = one("mysql://admin:p@ss:w0rd@db.example.com:3307/app");
        assert_eq!(e.host, "db.example.com");
        assert_eq!(e.port, Some(3307));
        assert_eq!(e.username, "admin");
        assert_eq!(e.password, "p@ss:w0rd");
        assert_eq!(e.category, "database");
        assert!(e.notes.contains("app"));
    }

    #[test]
    fn positional_without_keywords() {
        let e = one("10.1.2.3 root 123456");
        assert_eq!(e.host, "10.1.2.3");
        assert_eq!(e.username, "root");
        assert_eq!(e.password, "123456");
    }

    #[test]
    fn title_line_and_multiline() {
        let e = one("公司 GitLab\nURL: https://git.example.com/\nUsername: dev\nPassword: abc");
        assert_eq!(e.title, "公司 GitLab");
        assert_eq!(e.host, "https://git.example.com/");
        assert_eq!(e.username, "dev");
        assert_eq!(e.password, "abc");
    }

    #[test]
    fn multiple_blocks() {
        let v = parse("host a.com user u1 password p1\n\nhost b.com user u2 password p2");
        assert_eq!(v.len(), 2);
        assert_eq!(v[1].data.host, "b.com");
        assert_eq!(v[1].data.password, "p2");
    }

    #[test]
    fn keyword_inside_word_not_matched() {
        let e = one("host superuser.example.com user admin password passport123");
        assert_eq!(e.host, "superuser.example.com");
        assert_eq!(e.username, "admin");
        assert_eq!(e.password, "passport123");
    }

    #[test]
    fn empty_input() {
        assert!(parse("   \n\n ").is_empty());
    }
}
