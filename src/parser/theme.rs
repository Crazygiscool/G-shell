use std::collections::HashMap;
use std::env;
use std::process::{Command, Stdio};


// ── Color / Style ──

#[derive(Clone, Debug)]
pub struct Style {
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
}

impl Style {
    pub fn from_env(prefix: &str) -> Self {
        let val = env::var(format!("GS_STYLE_{}", prefix)).unwrap_or_default();
        Self::parse(&val)
    }

    pub fn parse(s: &str) -> Self {
        let mut style = Style { fg: None, bg: None, bold: false, dim: false, italic: false };
        for word in s.split_whitespace() {
            match word.to_lowercase().as_str() {
                "bold" => style.bold = true,
                "dim" => style.dim = true,
                "italic" => style.italic = true,
                "underline" => {}
                _ => {
                    if word.starts_with("on_") {
                        style.bg = Some(word[3..].to_string());
                    } else if style.fg.is_none() {
                        style.fg = Some(word.to_string());
                    }
                }
            }
        }
        style
    }

    pub fn to_ansi(&self) -> String {
        let mut codes: Vec<String> = Vec::new();
        if self.bold { codes.push("1".into()); }
        if self.dim { codes.push("2".into()); }
        if self.italic { codes.push("3".into()); }
        if let Some(ref fg) = self.fg {
            codes.push(color_to_ansi(fg, false));
        }
        if let Some(ref bg) = self.bg {
            codes.push(color_to_ansi(bg, true));
        }
        if codes.is_empty() {
            String::new()
        } else {
            format!("\x1b[{}m", codes.join(";"))
        }
    }
}

fn color_to_ansi(name: &str, bg: bool) -> String {
    let base: u8 = if bg { 48 } else { 38 };
    match name {
        "black" => format!("{};5;0", base),
        "red" => format!("{};5;1", base),
        "green" => format!("{};5;2", base),
        "yellow" => format!("{};5;3", base),
        "blue" => format!("{};5;4", base),
        "magenta" | "purple" => format!("{};5;5", base),
        "cyan" => format!("{};5;6", base),
        "white" => format!("{};5;7", base),
        "bright_black" | "gray" | "grey" => format!("{};5;8", base),
        "bright_red" => format!("{};5;9", base),
        "bright_green" => format!("{};5;10", base),
        "bright_yellow" => format!("{};5;11", base),
        "bright_blue" => format!("{};5;12", base),
        "bright_magenta" | "bright_purple" => format!("{};5;13", base),
        "bright_cyan" => format!("{};5;14", base),
        "bright_white" => format!("{};5;15", base),
        _ => {
            // Check for hex color #rrggbb
            if name.starts_with('#') && name.len() == 7 {
                if let (Ok(r), Ok(g), Ok(b)) = (
                    u8::from_str_radix(&name[1..3], 16),
                    u8::from_str_radix(&name[3..5], 16),
                    u8::from_str_radix(&name[5..7], 16),
                ) {
                    return format!("{};2;{};{};{}", base, r, g, b);
                }
            }
            // Try as ANSI 0-255
            if let Ok(n) = name.parse::<u8>() {
                return format!("{};5;{}", base, n);
            }
            String::new()
        }
    }
}

// ── Segment rendering ──

pub fn render_user() -> String {
    env::var("USER").unwrap_or_else(|_| "?".to_string())
}

pub fn render_host() -> String {
    env::var("HOSTNAME").ok().or_else(|| {
        Command::new("hostname").arg("-s").output().ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok().map(|x| x.trim().to_string()))
    }).unwrap_or_else(|| "?".to_string())
}

pub fn render_path() -> String {
    let cwd = env::current_dir().ok();
    let cwd = cwd.as_deref().and_then(|p| p.to_str()).unwrap_or("?");
    if let Ok(home) = env::var("HOME") {
        if cwd.starts_with(&home) {
            return "~".to_string() + &cwd[home.len()..];
        }
    }
    cwd.to_string()
}

pub fn render_git() -> String {
    // Check for GS_GIT_DISABLED
    if env::var("GS_GIT_DISABLED").is_ok() {
        return String::new();
    }

    let branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string());

    match branch {
        Some(b) if !b.is_empty() => {
            let status = Command::new("git")
                .args(["status", "--porcelain"])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output()
                .ok()
                .map(|o| o.stdout.iter().any(|&c| c != b'\n'));
            let dirty = status.unwrap_or(false);
            if dirty {
                format!(" {} \u{2717}", b)  // branch ✗
            } else {
                format!(" {} \u{2713}", b)  // branch ✓
            }
        }
        _ => String::new(),
    }
}

pub fn render_time() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let sec = now.as_secs() % 86400;
    let h = sec / 3600;
    let m = sec / 60 % 60;
    let s = sec % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

pub fn render_exit(last_exit_code: i32) -> String {
    if last_exit_code == 0 {
        String::new()
    } else {
        format!("({})", last_exit_code)
    }
}

pub fn render_root() -> String {
    let user = env::var("USER").unwrap_or_default();
    if user == "root" { "#".to_string() } else { String::new() }
}

pub fn render_prompt_char() -> String {
    let user = env::var("USER").unwrap_or_default();
    if user == "root" { "#".to_string() } else { "$".to_string() }
}

// ── Prompt renderer ──

pub fn render_prompt(format: &str, last_exit_code: i32) -> String {
    let mut result = String::new();
    let mut chars = format.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            // Legacy PS1 escape sequences
            match chars.next() {
                Some('w') => result.push_str(&render_path()),
                Some('W') => {
                    let p = render_path();
                    let base = p.rsplit('/').next().unwrap_or(&p);
                    result.push_str(base);
                }
                Some('u') => result.push_str(&render_user()),
                Some('h') => result.push_str(&render_host()),
                Some('$') => result.push_str(&render_prompt_char()),
                Some('t') => result.push_str(&render_time()),
                Some('e') => result.push('\x1b'),
                Some('n') => result.push('\n'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else if c == '{' {
            // Segment placeholder: {user}, {path}, etc.
            let mut seg_name = String::new();
            while let Some(&ch) = chars.peek() {
                if ch == '}' {
                    chars.next();
                    break;
                }
                seg_name.push(ch);
                chars.next();
            }
            let seg_text = match seg_name.as_str() {
                "user" => render_user(),
                "host" => render_host(),
                "path" => render_path(),
                "git" => render_git(),
                "exit" => render_exit(last_exit_code),
                "time" => render_time(),
                "root" => render_root(),
                "prompt" => render_prompt_char(),
                "newline" => "\n".to_string(),
                other => format!("{{{}}}", other),
            };
            // Apply style from GS_STYLE_<NAME> env var
            let style_upper = seg_name.to_uppercase();
            let style = Style::from_env(&style_upper);
            let ansi = style.to_ansi();
            if !ansi.is_empty() {
                result.push_str(&ansi);
                result.push_str(&seg_text);
                result.push_str("\x1b[0m");
            } else {
                result.push_str(&seg_text);
            }
        } else {
            result.push(c);
        }
    }

    result
}

// ── oh-my-posh .omp.json loader ──

/// Parse an oh-my-posh JSON theme and return a GS_PROMPT_FORMAT string
pub fn load_omp_theme(path: &str) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let json = parse_json(&content)?;
    let obj = json.as_object()?;

    let mut format_parts: Vec<String> = Vec::new();

    if let Some(blocks) = obj.get("blocks")?.as_array() {
        for block in blocks {
            let block_obj = block.as_object()?;
            if let Some(segments) = block_obj.get("segments")?.as_array() {
                let mut block_parts: Vec<String> = Vec::new();
                for seg in segments {
                    let seg_obj = seg.as_object()?;
                    let seg_type = seg_obj.get("type")?.as_str()?;
                    let fg = seg_obj.get("foreground").and_then(|v| v.as_str());
                    let bg = seg_obj.get("background").and_then(|v| v.as_str());

                    let mapped = map_omp_segment(seg_type, seg)?;
                    let mut style_str = String::new();

                    if let Some(c) = fg {
                        if c != "transparent" && c != "default" {
                            style_str.push_str(c);
                        }
                    }
                    if let Some(c) = bg {
                        if c != "transparent" && c != "default" {
                            style_str.push_str(&format!(" on_{}", c));
                        }
                    }

                    if style_str.is_empty() {
                        block_parts.push(mapped);
                    } else {
                        let seg_name = seg_type.to_uppercase();
                        // Set style via temporary env override
                        unsafe { env::set_var(format!("GS_STYLE_{}", seg_name), &style_str); }
                        block_parts.push(mapped);
                    }
                }
                format_parts.push(block_parts.join(""));
            }
        }
    }

    if format_parts.is_empty() { None } else { Some(format_parts.join("")) }
}

fn map_omp_segment(seg_type: &str, obj: &JsonValue) -> Option<String> {
    match seg_type {
        "session" => Some("{user}@{host}".to_string()),
        "path" => Some("{path}".to_string()),
        "git" => Some(" {git}".to_string()),
        "exit" => Some("{exit}".to_string()),
        "time" => Some("{time}".to_string()),
        "text" => {
            let template = obj.get("template").and_then(|v| v.as_str())?;
            // Convert oh-my-posh templates: replace Go template vars with our segments
            let converted = template
                .replace(r"\ue0b6", "\u{e0b6}")
                .replace(r"\ue0b0", "\u{e0b0}")
                .replace(r"\ue0b1", "\u{e0b1}")
                .replace(r"\ue0b2", "\u{e0b2}")
                .replace(r"\ue0b3", "\u{e0b3}")
                .replace(r"\ue0b4", "\u{e0b4}")
                .replace("{{ .Folder }}", "{path}")
                .replace("{{ .Path }}", "{path}")
                .replace("{{ .UserName }}", "{user}")
                .replace("{{ .HostName }}", "{host}")
                .replace("{{ .Branch }}", "{git}")
                .replace("{{ .Code }}", "{exit}")
                .replace("{{ .Time }}", "{time}");
            Some(converted)
        }
        _ => None,
    }
}

// ── Minimal JSON parser (no dependencies) ──

#[derive(Debug, Clone)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    fn as_object(&self) -> Option<&HashMap<String, JsonValue>> {
        match self { JsonValue::Object(m) => Some(m), _ => None }
    }
    fn as_array(&self) -> Option<&Vec<JsonValue>> {
        match self { JsonValue::Array(a) => Some(a), _ => None }
    }
    fn as_str(&self) -> Option<&str> {
        match self { JsonValue::String(s) => Some(s.as_str()), _ => None }
    }
    fn get(&self, key: &str) -> Option<&JsonValue> {
        self.as_object()?.get(key)
    }
}

fn parse_json(input: &str) -> Option<JsonValue> {
    let mut chars = input.trim().chars().peekable();
    skip_whitespace(&mut chars);
    parse_value(&mut chars)
}

fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while let Some(&c) = chars.peek() {
        if c.is_whitespace() { chars.next(); } else { break; }
    }
}

fn parse_value(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<JsonValue> {
    skip_whitespace(chars);
    match chars.peek()? {
        '"' => parse_string(chars).map(JsonValue::String),
        '{' => parse_object(chars).map(JsonValue::Object),
        '[' => parse_array(chars).map(JsonValue::Array),
        't' | 'f' => parse_bool(chars).map(JsonValue::Bool),
        'n' => { chars.next(); chars.next(); chars.next(); chars.next(); Some(JsonValue::Null) }
        '0'..='9' | '-' => parse_number(chars).map(JsonValue::Number),
        _ => None,
    }
}

fn parse_string(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<String> {
    chars.next()?; // consume opening "
    let mut s = String::new();
    loop {
        match chars.next()? {
            '"' => return Some(s),
            '\\' => {
                match chars.next()? {
                    '"' => s.push('"'),
                    '\\' => s.push('\\'),
                    '/' => s.push('/'),
                    'n' => s.push('\n'),
                    'r' => s.push('\r'),
                    't' => s.push('\t'),
                    'u' => {
                        let hex: String = chars.by_ref().take(4).collect();
                        if let Ok(cp) = u32::from_str_radix(&hex, 16) {
                            if let Ok(c) = char::try_from(cp) {
                                s.push(c);
                            }
                        }
                    }
                    c => s.push(c),
                }
            }
            c => s.push(c),
        }
    }
}

fn parse_number(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<f64> {
    let mut s = String::new();
    if let Some(&'-') = chars.peek() { s.push(chars.next()?); }
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' || c == '+' || c == '-' {
            s.push(chars.next()?);
        } else { break; }
    }
    s.parse().ok()
}

fn parse_bool(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<bool> {
    if chars.peek() == Some(&'t') {
        chars.next(); chars.next(); chars.next(); chars.next(); // true
        Some(true)
    } else {
        chars.next(); chars.next(); chars.next(); chars.next(); chars.next(); // false
        Some(false)
    }
}

fn parse_array(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<Vec<JsonValue>> {
    chars.next()?; // consume [
    let mut arr = Vec::new();
    loop {
        skip_whitespace(chars);
        match chars.peek() {
            Some(']') => { chars.next(); return Some(arr); }
            None => return None,
            _ => {}
        }
        arr.push(parse_value(chars)?);
        skip_whitespace(chars);
        if chars.peek() == Some(&',') { chars.next(); }
    }
}

fn parse_object(chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<HashMap<String, JsonValue>> {
    chars.next()?; // consume {
    let mut obj = HashMap::new();
    loop {
        skip_whitespace(chars);
        match chars.peek() {
            Some('}') => { chars.next(); return Some(obj); }
            None => return None,
            _ => {}
        }
        let key = parse_string(chars)?;
        skip_whitespace(chars);
        chars.next()?; // consume :
        skip_whitespace(chars);
        let value = parse_value(chars)?;
        obj.insert(key, value);
        skip_whitespace(chars);
        if chars.peek() == Some(&',') { chars.next(); }
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_style_parse_foreground() {
        let s = Style::parse("red");
        assert_eq!(s.fg, Some("red".into()));
        assert!(s.bg.is_none());
    }

    #[test]
    fn test_style_parse_bold() {
        let s = Style::parse("green bold");
        assert_eq!(s.fg, Some("green".into()));
        assert!(s.bold);
    }

    #[test]
    fn test_style_parse_background() {
        let s = Style::parse("white on_blue");
        assert_eq!(s.fg, Some("white".into()));
        assert_eq!(s.bg, Some("blue".into()));
    }

    #[test]
    fn test_style_parse_italic_dim() {
        let s = Style::parse("cyan italic dim");
        assert_eq!(s.fg, Some("cyan".into()));
        assert!(s.italic);
        assert!(s.dim);
    }

    #[test]
    fn test_color_to_ansi_named() {
        assert!(color_to_ansi("red", false).contains("38;5;1"));
        assert!(color_to_ansi("blue", true).contains("48;5;4"));
    }

    #[test]
    fn test_color_to_ansi_hex() {
        let result = color_to_ansi("#ff0000", false);
        assert!(result.contains("38;2;255;0;0"));
    }

    #[test]
    fn test_render_user() {
        let prev = env::var("USER").ok();
        unsafe { env::set_var("USER", "testuser"); }
        assert_eq!(render_user(), "testuser");
        if let Some(v) = prev { unsafe { env::set_var("USER", v); } }
        else { unsafe { env::remove_var("USER"); } }
    }

    #[test]
    fn test_render_path_in_home() {
        let prev_home = env::var("HOME").ok();
        let cwd = env::current_dir().unwrap();
        // Set HOME to current dir's parent (which definitely exists)
        let parent = cwd.parent().unwrap().to_str().unwrap().to_string();
        let sub = cwd.file_name().unwrap().to_str().unwrap();
        unsafe { env::set_var("HOME", &parent); }
        let p = render_path();
        assert_eq!(p, format!("~/{}", sub));
        if let Some(v) = prev_home { unsafe { env::set_var("HOME", v); } }
        else { unsafe { env::remove_var("HOME"); } }
    }

    #[test]
    fn test_render_exit_zero() {
        assert_eq!(render_exit(0), "");
    }

    #[test]
    fn test_render_exit_nonzero() {
        assert_eq!(render_exit(42), "(42)");
    }

    #[test]
    fn test_render_prompt_not_root() {
        let prev = env::var("USER").ok();
        unsafe { env::set_var("USER", "nobody"); }
        assert_eq!(render_prompt_char(), "$");
        if let Some(v) = prev { unsafe { env::set_var("USER", v); } }
        else { unsafe { env::remove_var("USER"); } }
    }

    #[test]
    fn test_render_prompt_root() {
        let prev = env::var("USER").ok();
        unsafe { env::set_var("USER", "root"); }
        assert_eq!(render_prompt_char(), "#");
        if let Some(v) = prev { unsafe { env::set_var("USER", v); } }
        else { unsafe { env::remove_var("USER"); } }
    }

    #[test]
    fn test_render_prompt_legacy_escapes() {
        let prev_user = env::var("USER").ok();
        let prev_home = env::var("HOME").ok();
        let prev_pwd = env::current_dir().ok();
        unsafe {
            env::set_var("USER", "test");
            env::set_var("HOME", "/home/test");
            std::env::set_current_dir("/home/test").ok();
        }
        let r = render_prompt("\\u@\\h:\\w\\$ ", 0);
        assert!(r.contains("test"));
        assert!(r.contains('$'));
        if let Some(d) = prev_pwd { std::env::set_current_dir(d).ok(); }
        if let Some(v) = prev_home { unsafe { env::set_var("HOME", v); } }
        else { unsafe { env::remove_var("HOME"); } }
        if let Some(v) = prev_user { unsafe { env::set_var("USER", v); } }
        else { unsafe { env::remove_var("USER"); } }
    }

    #[test]
    fn test_render_prompt_segments() {
        let prev_user = env::var("USER").ok();
        unsafe { env::set_var("USER", "testuser"); }
        let r = render_prompt("{user} x {exit} $ ", 0);
        assert_eq!(r, "testuser x  $ ");
        let r2 = render_prompt("{user} x {exit} $ ", 42);
        assert_eq!(r2, "testuser x (42) $ ");
        if let Some(v) = prev_user { unsafe { env::set_var("USER", v); } }
        else { unsafe { env::remove_var("USER"); } }
    }

    #[test]
    fn test_render_prompt_mixed() {
        let r = render_prompt("\\u {host} \\w", 0);
        assert!(r.contains(' '));
    }

    #[test]
    fn test_render_prompt_newline() {
        let r = render_prompt("line1{newline}line2", 0);
        assert_eq!(r, "line1\nline2");
    }

    #[test]
    fn test_json_parse_string() {
        let json = parse_json(r#"{"key": "value"}"#).unwrap();
        let obj = json.as_object().unwrap();
        assert_eq!(obj.get("key").unwrap().as_str().unwrap(), "value");
    }

    #[test]
    fn test_json_parse_nested() {
        let json = parse_json(r#"{"a": {"b": [1, 2, 3]}}"#);
        assert!(json.is_some());
    }

    #[test]
    fn test_json_parse_empty_object() {
        let json = parse_json("{}");
        assert!(json.is_some());
        assert!(json.unwrap().as_object().unwrap().is_empty());
    }

    #[test]
    fn test_json_parse_array() {
        let json = parse_json(r#"["a", "b", "c"]"#);
        assert!(json.is_some());
        assert_eq!(json.unwrap().as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_json_parse_null() {
        let json = parse_json("null");
        assert!(matches!(json, Some(JsonValue::Null)));
    }

    #[test]
    fn test_json_parse_number() {
        let json = parse_json("42");
        assert!(matches!(json, Some(JsonValue::Number(v)) if v == 42.0));
    }

    #[test]
    fn test_json_parse_unicode() {
        let json = parse_json(r#""\u0048\u0065\u006c\u006c\u006f""#).unwrap();
        assert_eq!(json.as_str().unwrap(), "Hello");
    }

    #[test]
    fn test_style_to_ansi() {
        let s = Style { fg: Some("red".into()), bg: None, bold: true, dim: false, italic: false };
        let a = s.to_ansi();
        assert!(a.starts_with("\x1b["));
        assert!(a.contains("1")); // bold
        assert!(a.contains("38;5;1")); // red
    }

    #[test]
    fn test_style_to_ansi_empty() {
        let s = Style { fg: None, bg: None, bold: false, dim: false, italic: false };
        assert_eq!(s.to_ansi(), "");
    }
}
