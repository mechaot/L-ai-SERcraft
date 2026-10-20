//! Port of `Tools/Json.cs` (`JSONParser`): tiny forgiving JSON reader.
//!
//! Only the shapes LaserGRBL uses are ported: dynamic parse (`FromJson<object>`) and the typed
//! helpers needed by `GitHub.OnlineVersion` (see [`crate::github`]). The C# reflection based
//! `ParseObject` for arbitrary classes has no Rust equivalent (N/A).
//! Quirks kept: strings in dynamic mode lose every backslash instead of being unescaped,
//! numbers with `.` are doubles, others ints (parse failure = 0), bad input gives `None`.

use lg_dotnet::{is_digit, is_white_space, parse_double, parse_int};

/// Dynamic JSON value (C# `object` from `ParseAnonymousValue`).
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    /// `null` or anything invalid.
    Null,
    Bool(bool),
    /// `int` (no dot in text).
    Int(i32),
    /// `double` (dot in text).
    Double(f64),
    Str(String),
    /// `List<object>`.
    List(Vec<JsonValue>),
    /// `Dictionary<string,object>`; insertion order kept.
    Dict(Vec<(String, JsonValue)>),
}

impl JsonValue {
    /// Dictionary lookup (exact key).
    ///
    /// `key`: member name. Returns the member or None (also when self is no dict).
    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Dict(d) => d.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// String content.
    ///
    /// Returns the text for `Str`, else None.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::Str(s) => Some(s),
            _ => None,
        }
    }

    /// List content.
    ///
    /// Returns the slice for `List`, else None.
    pub fn as_list(&self) -> Option<&[JsonValue]> {
        match self {
            JsonValue::List(l) => Some(l),
            _ => None,
        }
    }
}

/// Remove whitespace outside strings (first step of `FromJson`).
///
/// `json`: raw text. Returns the compact char vector (string contents kept verbatim).
fn compact(json: &str) -> Vec<char> {
    let src: Vec<char> = json.chars().collect();
    let mut out = Vec::with_capacity(src.len());
    let mut i = 0;
    while i < src.len() {
        let c = src[i];
        if c == '"' {
            i = append_until_string_end(&mut out, i, &src);
            i += 1;
            continue;
        }
        if !is_white_space(c) {
            out.push(c);
        }
        i += 1;
    }
    out
}

/// C# `AppendUntilStringEnd` (escape chars always kept).
///
/// `out`: target buffer. `start`: index of the opening quote. `json`: source.
/// Returns the index of the closing quote (or the last index when unterminated).
fn append_until_string_end(out: &mut Vec<char>, start: usize, json: &[char]) -> usize {
    out.push(json[start]);
    let mut i = start + 1;
    while i < json.len() {
        if json[i] == '\\' {
            out.push(json[i]);
            // NOTE(upstream): C# throws on a trailing backslash; we stop there instead.
            if i + 1 < json.len() {
                out.push(json[i + 1]);
            }
            i += 2;
            continue;
        }
        if json[i] == '"' {
            out.push(json[i]);
            return i;
        }
        out.push(json[i]);
        i += 1;
    }
    json.len().saturating_sub(1)
}

/// C# `Split`: split `{k:v,..}` / `[a,b]` into the flat list of values (keys and values alternate).
///
/// `json`: compact text including the outer brackets. Returns the element texts.
pub fn split(json: &[char]) -> Vec<Vec<char>> {
    let mut list = Vec::new();
    if json.len() <= 2 {
        return list;
    }
    let mut depth = 0i32;
    let mut cur: Vec<char> = Vec::new();
    let mut i = 1;
    while i < json.len() - 1 {
        match json[i] {
            '[' | '{' => depth += 1,
            ']' | '}' => depth -= 1,
            '"' => {
                i = append_until_string_end(&mut cur, i, json) + 1;
                continue;
            }
            ',' | ':' if depth == 0 => {
                list.push(std::mem::take(&mut cur));
                i += 1;
                continue;
            }
            _ => {}
        }
        cur.push(json[i]);
        i += 1;
    }
    list.push(cur);
    list
}

/// Key text of a split key element (`Substring(1, len-2)`).
///
/// `k`: quoted key. Returns the inner text, or None when too short.
fn key_text(k: &[char]) -> Option<String> {
    if k.len() < 2 {
        return None;
    }
    Some(k[1..k.len() - 1].iter().collect())
}

/// C# `ParseAnonymousValue`.
///
/// `json`: compact value text. Returns the value, or Err on duplicate keys / bad key (C# throws).
fn parse_anonymous(json: &[char]) -> Result<JsonValue, String> {
    if json.is_empty() {
        return Ok(JsonValue::Null);
    }
    let (first, last) = (json[0], json[json.len() - 1]);
    if first == '{' && last == '}' {
        let elems = split(json);
        if elems.len() % 2 != 0 {
            return Ok(JsonValue::Null);
        }
        let mut dict: Vec<(String, JsonValue)> = Vec::with_capacity(elems.len() / 2);
        for pair in elems.chunks(2) {
            let key = key_text(&pair[0]).ok_or("bad json key")?;
            let val = parse_anonymous(&pair[1])?;
            // Dictionary.Add throws on duplicates
            if dict.iter().any(|(k, _)| *k == key) {
                return Err(format!("duplicate key '{key}'"));
            }
            dict.push((key, val));
        }
        return Ok(JsonValue::Dict(dict));
    }
    if first == '[' && last == ']' {
        let items = split(json);
        let mut out = Vec::with_capacity(items.len());
        for it in &items {
            out.push(parse_anonymous(it)?);
        }
        return Ok(JsonValue::List(out));
    }
    if first == '"' && last == '"' {
        // NOTE(upstream): strips every backslash instead of unescaping
        let s: String = json[1..json.len().max(2) - 1].iter().filter(|c| **c != '\\').collect();
        return Ok(JsonValue::Str(s));
    }
    if is_digit(first) || first == '-' {
        let text: String = json.iter().collect();
        return Ok(if text.contains('.') {
            JsonValue::Double(parse_double(&text).unwrap_or(0.0))
        } else {
            JsonValue::Int(parse_int(&text).unwrap_or(0))
        });
    }
    let text: String = json.iter().collect();
    Ok(match text.as_str() {
        "true" => JsonValue::Bool(true),
        "false" => JsonValue::Bool(false),
        _ => JsonValue::Null,
    })
}

/// `FromJson<object>`: dynamic parse.
///
/// `json`: text. Returns the value, `JsonValue::Null` for garbage, Err for duplicate keys.
pub fn from_json_object(json: &str) -> Result<JsonValue, String> {
    parse_anonymous(&compact(json))
}

/// C# `ParseValue(typeof(string), json)`: unescape a quoted string.
///
/// `json`: compact value text (with quotes) or `null`. Returns the string or None for `null`.
pub fn parse_string_value(json: &[char]) -> Option<String> {
    if json.iter().collect::<String>() == "null" {
        return None;
    }
    if json.len() <= 2 {
        return Some(String::new());
    }
    let mut out = String::new();
    let end = json.len() - 1;
    let mut i = 1;
    while i < end {
        if json[i] == '\\' && i + 1 < end {
            if let Some(j) = "\"\\nrtbf/".find(json[i + 1]) {
                out.push("\"\\\n\r\t\u{8}\u{c}/".chars().nth(j).unwrap_or(' '));
                i += 2;
                continue;
            }
            if json[i + 1] == 'u' && i + 5 < end {
                let hex: String = json[i + 2..i + 6].iter().collect();
                // UInt32.TryParse(AllowHexSpecifier): hex digits only (from_str_radix would accept a sign)
                if let Some(c) = Some(&hex).filter(|h| h.chars().all(|c| c.is_ascii_hexdigit())).and_then(|h| u32::from_str_radix(h, 16).ok()) {
                    // lone surrogates cannot be a Rust char
                    out.push(char::from_u32(c).unwrap_or('\u{fffd}'));
                    i += 6;
                    continue;
                }
            }
        }
        out.push(json[i]);
        i += 1;
    }
    Some(out)
}

/// Typed `List<T>` split: elements of a compact JSON array.
///
/// `json`: whole text. Returns None when it is not an array (C# returns null).
pub fn parse_list_elements(json: &str) -> Option<Vec<Vec<char>>> {
    let c = compact(json);
    split_list(&c)
}

/// Like [`parse_list_elements`] for already compact text.
///
/// `c`: compact value. Returns the element texts or None.
pub fn split_list(c: &[char]) -> Option<Vec<Vec<char>>> {
    if c.is_empty() || c[0] != '[' || c[c.len() - 1] != ']' {
        return None;
    }
    Some(split(c))
}

/// Typed object members: key/value text pairs of a compact `{..}` (C# `ParseObject` split).
///
/// `c`: compact value. Returns the pairs (empty when odd split or not an object).
pub fn split_object(c: &[char]) -> Vec<(String, Vec<char>)> {
    if c.is_empty() || c[0] != '{' || c[c.len() - 1] != '}' {
        return Vec::new();
    }
    let elems = split(c);
    if elems.len() % 2 != 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for pair in elems.chunks(2) {
        // keys of 2 chars or less are skipped like C#
        if pair[0].len() <= 2 {
            continue;
        }
        if let Some(k) = key_text(&pair[0]) {
            out.push((k, pair[1].clone()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_values() {
        let v = from_json_object(r#" {"a": 1, "b": [1.5, "x\ny", true, null], "c": {"d": -3}} "#).unwrap();
        assert_eq!(v.get("a"), Some(&JsonValue::Int(1)));
        let b = v.get("b").unwrap().as_list().unwrap();
        assert_eq!(b[0], JsonValue::Double(1.5));
        // backslash stripped, not unescaped
        assert_eq!(b[1], JsonValue::Str("xny".into()));
        assert_eq!(b[2], JsonValue::Bool(true));
        assert_eq!(b[3], JsonValue::Null);
        assert_eq!(v.get("c").unwrap().get("d"), Some(&JsonValue::Int(-3)));
    }

    #[test]
    fn garbage_and_duplicates() {
        assert_eq!(from_json_object("nope").unwrap(), JsonValue::Null);
        assert!(from_json_object(r#"{"a":1,"a":2}"#).is_err());
        assert_eq!(from_json_object("").unwrap(), JsonValue::Null);
        assert_eq!(from_json_object("[]").unwrap(), JsonValue::List(vec![]));
        assert_eq!(from_json_object("{}").unwrap(), JsonValue::Dict(vec![]));
    }

    #[test]
    fn whitespace_inside_strings_kept() {
        let v = from_json_object(r#"{"k k": "a b"}"#).unwrap();
        assert_eq!(v.get("k k").unwrap().as_str(), Some("a b"));
    }

    #[test]
    fn colon_comma_in_strings() {
        let v = from_json_object(r#"{"u":"http://x,y"}"#).unwrap();
        assert_eq!(v.get("u").unwrap().as_str(), Some("http//x,y".replace("//", "://").as_str()));
    }

    #[test]
    fn typed_string_unescape() {
        let c: Vec<char> = r#""a\n\u0041\"""#.chars().collect();
        assert_eq!(parse_string_value(&c).unwrap(), "a\nA\"");
        assert_eq!(parse_string_value(&"null".chars().collect::<Vec<_>>()), None);
        // "+041" is no hex number for TryParse: the escape stays literal
        let c: Vec<char> = r#""x\u+041y""#.chars().collect();
        assert_eq!(parse_string_value(&c).unwrap(), "x\\u+041y");
    }
}
