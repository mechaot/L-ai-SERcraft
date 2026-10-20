//! Port of `CSV/CsvDictionary.cs`.

use super::formatted_message::{FormattedMessage, StringList};
use std::collections::HashMap;

/// Key -> fixed number of columns, loaded from CSV text.
#[derive(Debug, Clone, Default)]
pub struct CsvDictionary {
    map: HashMap<String, Vec<String>>,
}

impl CsvDictionary {
    /// Parse CSV text (first occurrence of a key wins).
    ///
    /// `text`: CSV content (BOM stripped). `len`: data columns after the key.
    /// Returns Err when a row has too few columns or an unescaped `#` field (C# would throw).
    pub fn from_text(text: &str, len: usize) -> Result<Self, String> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut map: HashMap<String, Vec<String>> = HashMap::new();
        // ReadLine: split on \n, \r\n and lone \r; last empty piece after final newline is not a line
        let mut lines: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).collect();
        if lines.last() == Some(&"") {
            lines.pop();
        }
        for rline in lines {
            let sl = StringList::from_message(rline, ",");
            let fm = FormattedMessage::from_string_list(sl);
            let mut idx = 0usize;
            let mut next = |fm: &FormattedMessage| -> Result<String, String> {
                let v = fm.read_string(idx)?.ok_or("null field")?;
                idx += 1;
                Ok(v.trim().to_string())
            };
            let key = next(&fm)?;
            let mut data = Vec::with_capacity(len);
            for _ in 0..len {
                data.push(next(&fm)?);
            }
            map.entry(key).or_insert(data);
        }
        Ok(Self { map })
    }

    /// Lookup a column.
    ///
    /// `key`: row key. `item`: column index. Returns None when key or column missing.
    pub fn get_item(&self, key: &str, item: usize) -> Option<&str> {
        self.map.get(key).and_then(|v| v.get(item)).map(String::as_str)
    }

    /// Number of rows.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// True when empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}
