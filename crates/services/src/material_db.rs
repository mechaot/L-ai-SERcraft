//! Port of `PSHelper/MaterialDB.cs` (+ `MaterialDB.xsd` / typed DataSet of `MaterialDB.Designer.cs`).
//!
//! The typed `DataSet` becomes [`MaterialsTable`] (rows + "has pending changes" flag + the two
//! unique constraints and column checks). Files use the same DataSet XML so the standard
//! `StandardMaterials.psh` and the user `UserMaterials.psh` stay interchangeable.
//!
//! Deviations: `ServerFile` lives in the data dir like C#, but when it is missing the bundled
//! `resources/misc/StandardMaterials.psh` is used (nothing downloads it, PLAN section 6). C#
//! `OrderBy(s => s)` uses culture compare; [`culture_compare`] approximates the invariant culture.

use std::cmp::Ordering;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

const NAMESPACE: &str = "http://tempuri.org/MaterialDB.xsd";

/// One `Materials` row (C# `MaterialsRow`).
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialRow {
    /// `id` (Guid text, lowercase).
    pub id: String,
    /// `Visible`.
    pub visible: bool,
    pub model: String,
    pub material: String,
    pub thickness: String,
    pub action: String,
    pub power: i32,
    pub speed: i32,
    pub cycles: i32,
    /// `Remarks`; None = DBNull (element absent in the file).
    pub remarks: Option<String>,
}

impl MaterialRow {
    /// Typed getter `Remarks` (`nullValue="_empty"`): empty text for DBNull.
    pub fn remarks(&self) -> &str {
        self.remarks.as_deref().unwrap_or("")
    }
}

/// Error of a table operation (C# throws `Exception` / `ConstraintException`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialError(pub String);

impl std::fmt::Display for MaterialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for MaterialError {}

/// `OnColumnChanging` checks. Messages are the C# texts.
///
/// `power`/`speed`/`cycles`: proposed values (pass the current value for untouched columns).
fn validate(power: i32, speed: i32, cycles: i32) -> Result<(), MaterialError> {
    let e = |m: &str| Err(MaterialError(m.to_string()));
    if power < 1 {
        return e("Please enter a valid power level (Min 1%)");
    }
    if power > 100 {
        return e("Please enter a valid power level (Max 100%)");
    }
    if speed < 1 {
        return e("Please enter a valid speed (Min 1 mm/min)");
    }
    if speed > 100000 {
        return e("Please enter a valid speed (Max 100000 mm/min)");
    }
    if cycles < 1 {
        return e("Please enter a valid cycles number (Min 1 cycles)");
    }
    // NOTE(upstream): check is `>= 100` with the text "Max 99 cycles"
    if cycles >= 100 {
        return e("Please enter a valid cycles number (Max 99 cycles)");
    }
    Ok(())
}

/// Key of the `RecordUnique` constraint (DataSet compares strings case-insensitively by default).
fn record_key(r: &MaterialRow) -> (String, String, String, String) {
    (r.model.to_lowercase(), r.material.to_lowercase(), r.thickness.to_lowercase(), r.action.to_lowercase())
}

/// Approximation of .NET invariant-culture `string.Compare` for `OrderBy(s => s)`.
///
/// Primary key: punctuation (DUCET order) < symbols < digits < letters, letters compared without
/// case; ties fall back to lowercase-before-uppercase. Matches the Mono reference lists.
///
/// `a`,`b`: strings. Returns the ordering.
pub fn culture_compare(a: &str, b: &str) -> Ordering {
    // DUCET order of common variable / punctuation characters
    const PUNCT: &str = "_-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$";
    fn primary(c: char) -> (u8, u32) {
        if c.is_whitespace() {
            (0, 0)
        } else if let Some(i) = PUNCT.find(c) {
            (1, i as u32)
        } else if c.is_ascii_digit() {
            (3, c as u32)
        } else if c.is_alphabetic() {
            // fold case and strip the common latin accents by simple lowercase
            (4, c.to_lowercase().next().unwrap_or(c) as u32)
        } else {
            (2, c as u32)
        }
    }
    let pa: Vec<(u8, u32)> = a.chars().map(primary).collect();
    let pb: Vec<(u8, u32)> = b.chars().map(primary).collect();
    pa.cmp(&pb).then_with(|| {
        // tertiary: lowercase sorts before uppercase
        for (x, y) in a.chars().zip(b.chars()) {
            if x != y {
                return match (x.is_lowercase(), y.is_lowercase()) {
                    (true, false) => Ordering::Less,
                    (false, true) => Ordering::Greater,
                    _ => x.cmp(&y),
                };
            }
        }
        Ordering::Equal
    })
}

/// `Select(..).Distinct().OrderBy(s => s)`.
///
/// `it`: values. Returns distinct values sorted with [`culture_compare`].
fn distinct_sorted<'a>(it: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut v: Vec<String> = it.filter(|s| seen.insert(*s)).map(str::to_string).collect();
    v.sort_by(|a, b| culture_compare(a, b));
    v
}

/// The `Materials` DataTable.
#[derive(Debug, Clone, Default)]
pub struct MaterialsTable {
    rows: Vec<MaterialRow>,
    /// DataTable.GetChanges() != null
    changed: bool,
}

impl MaterialsTable {
    /// All (not deleted) rows in table order.
    pub fn rows(&self) -> &[MaterialRow] {
        &self.rows
    }

    /// True when rows were added, edited or deleted since the last [`accept_changes`](Self::accept_changes).
    pub fn has_changes(&self) -> bool {
        self.changed
    }

    /// `AcceptChanges()`.
    pub fn accept_changes(&mut self) {
        self.changed = false;
    }

    /// `FindByid`.
    ///
    /// `id`: guid text. Returns the row index or None.
    pub fn find_by_id(&self, id: &str) -> Option<usize> {
        self.rows.iter().position(|r| r.id.eq_ignore_ascii_case(id))
    }

    /// `OnTableNewRow` + column defaults: a fresh row for the editor (not yet added).
    ///
    /// Returns a row with a new guid; model/material copied from the last row (C#
    /// `GetLastNotDeleted`), other columns at their defaults.
    pub fn new_row(&self) -> MaterialRow {
        let (model, material) = match self.rows.last() {
            Some(l) => (l.model.clone(), l.material.clone()),
            None => ("Generic Laser".into(), "Generic Material".into()),
        };
        MaterialRow {
            id: crate::custom_button::new_guid(),
            visible: true,
            model,
            material,
            thickness: String::new(),
            action: String::new(),
            power: 100,
            speed: 1000,
            cycles: 1,
            remarks: None,
        }
    }

    /// `AddMaterialsRow` / `Rows.Add`: checks column limits, `Constraint1` (id) and `RecordUnique`.
    ///
    /// `row`: row to append. Returns Ok or the C# exception text.
    pub fn add_row(&mut self, row: MaterialRow) -> Result<(), MaterialError> {
        validate(row.power, row.speed, row.cycles)?;
        self.check_unique(&row, None)?;
        self.rows.push(row);
        self.changed = true;
        Ok(())
    }

    /// Unique constraint check.
    ///
    /// `row`: candidate. `skip`: index of the row being replaced. Returns Ok or a ConstraintException text.
    fn check_unique(&self, row: &MaterialRow, skip: Option<usize>) -> Result<(), MaterialError> {
        let key = record_key(row);
        for (i, r) in self.rows.iter().enumerate() {
            if Some(i) == skip {
                continue;
            }
            if r.id.eq_ignore_ascii_case(&row.id) {
                return Err(MaterialError(format!("Column 'id' is constrained to be unique.  Value '{}' is already present.", row.id)));
            }
            if record_key(r) == key {
                return Err(MaterialError("Column 'Model, Material, Thickness, Action' is constrained to be unique.".into()));
            }
        }
        Ok(())
    }

    /// `ImportRow`: copy a row of another table (keeps values and id). Same constraints as add,
    /// without the column limit events.
    ///
    /// `row`: source row. Returns Ok or the constraint error.
    pub fn import_row(&mut self, row: &MaterialRow) -> Result<(), MaterialError> {
        self.check_unique(row, None)?;
        self.rows.push(row.clone());
        self.changed = true;
        Ok(())
    }

    /// Replace the editable columns of a row (grid edit). `id` is read-only.
    ///
    /// `index`: row. `new`: new values (its `id` is ignored). Returns Ok or the exception text.
    pub fn update_row(&mut self, index: usize, mut new: MaterialRow) -> Result<(), MaterialError> {
        let Some(old) = self.rows.get(index) else { return Err(MaterialError("There is no row at position".into())) };
        new.id = old.id.clone();
        validate(new.power, new.speed, new.cycles)?;
        self.check_unique(&new, Some(index))?;
        self.rows[index] = new;
        self.changed = true;
        Ok(())
    }

    /// `Rows[i].Delete()`.
    ///
    /// `index`: row. Returns false when out of range.
    pub fn delete_row(&mut self, index: usize) -> bool {
        if index >= self.rows.len() {
            return false;
        }
        self.rows.remove(index);
        self.changed = true;
        true
    }

    fn enabled(&self) -> impl Iterator<Item = &MaterialRow> {
        self.rows.iter().filter(|r| r.visible)
    }

    /// `Models()`: distinct models of visible rows, sorted.
    pub fn models(&self) -> Vec<String> {
        distinct_sorted(self.enabled().map(|r| r.model.as_str()))
    }

    /// `Materials(model)`.
    ///
    /// `model`: model name. Returns distinct sorted materials.
    pub fn materials(&self, model: &str) -> Vec<String> {
        distinct_sorted(self.enabled().filter(|r| r.model == model).map(|r| r.material.as_str()))
    }

    /// `Thickness(model, material, action)`.
    ///
    /// Returns distinct sorted thickness values.
    pub fn thickness(&self, model: &str, material: &str, action: &str) -> Vec<String> {
        distinct_sorted(self.enabled().filter(|r| r.model == model && r.material == material && r.action == action).map(|r| r.thickness.as_str()))
    }

    /// `Actions(model, material)`.
    ///
    /// Returns distinct sorted actions.
    pub fn actions(&self, model: &str, material: &str) -> Vec<String> {
        distinct_sorted(self.enabled().filter(|r| r.model == model && r.material == material).map(|r| r.action.as_str()))
    }

    /// `GetResult(model, material, action, thickness)` (C# `.First()` throws when empty).
    ///
    /// Returns the first visible matching row or None.
    pub fn get_result(&self, model: &str, material: &str, action: &str, thickness: &str) -> Option<&MaterialRow> {
        self.enabled().find(|r| r.model == model && r.material == material && r.thickness == thickness && r.action == action)
    }
}

/// Read a DataSet XML file into rows (`DataTable.ReadXml`).
///
/// `path`: file. Returns the rows, or Err on I/O / XML problems (C# throws).
fn read_xml(path: &Path) -> Result<Vec<MaterialRow>, MaterialError> {
    let text = std::fs::read_to_string(path).map_err(|e| MaterialError(e.to_string()))?;
    parse_xml(&text)
}

/// Parse DataSet XML text.
///
/// `text`: document. Returns the rows or an error.
pub fn parse_xml(text: &str) -> Result<Vec<MaterialRow>, MaterialError> {
    let text = text.trim_start_matches('\u{feff}');
    let doc = roxmltree::Document::parse(text).map_err(|e| MaterialError(e.to_string()))?;
    let mut rows = Vec::new();
    for n in doc.root_element().children().filter(|n| n.is_element() && n.tag_name().name() == "Materials") {
        let get = |name: &str| n.children().find(|c| c.is_element() && c.tag_name().name() == name).map(|c| c.text().unwrap_or("").to_string());
        let int = |name: &str, def: i32| get(name).and_then(|s| s.trim().parse::<i32>().ok()).unwrap_or(def);
        let Some(id) = get("id") else { return Err(MaterialError("Column 'id' does not allow nulls.".into())) };
        let row = MaterialRow {
            id: id.trim().to_lowercase(),
            visible: get("Visible").map(|s| matches!(s.trim(), "true" | "1")).unwrap_or(true),
            model: get("Model").unwrap_or_else(|| "Generic Laser".into()),
            material: get("Material").unwrap_or_else(|| "Generic Material".into()),
            thickness: get("Thickness").unwrap_or_default(),
            action: get("Action").unwrap_or_default(),
            power: int("Power", 100),
            speed: int("Speed", 1000),
            cycles: int("Cycles", 1),
            remarks: get("Remarks"),
        };
        validate(row.power, row.speed, row.cycles)?;
        rows.push(row);
    }
    Ok(rows)
}

/// Escape text for an element (`XmlWriter` rules: `& < >` and `\r`).
///
/// `s`: raw text. Returns the escaped text.
fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '\r' => o.push_str("&#xD;"),
            c => o.push(c),
        }
    }
    o
}

/// DataSet `WriteXml` text (no trailing newline, empty strings as `<X />`, DBNull omitted).
///
/// `rows`: rows to write. Returns the XML document text.
pub fn write_xml(rows: &[MaterialRow]) -> String {
    if rows.is_empty() {
        return format!("<?xml version=\"1.0\" standalone=\"yes\"?>\n<MaterialDB xmlns=\"{NAMESPACE}\" />");
    }
    let mut o = format!("<?xml version=\"1.0\" standalone=\"yes\"?>\n<MaterialDB xmlns=\"{NAMESPACE}\">\n");
    let el = |o: &mut String, name: &str, v: &str| {
        if v.is_empty() {
            o.push_str(&format!("    <{name} />\n"));
        } else {
            o.push_str(&format!("    <{name}>{}</{name}>\n", esc(v)));
        }
    };
    for r in rows {
        o.push_str("  <Materials>\n");
        el(&mut o, "id", &r.id);
        el(&mut o, "Visible", if r.visible { "true" } else { "false" });
        el(&mut o, "Model", &r.model);
        el(&mut o, "Material", &r.material);
        el(&mut o, "Thickness", &r.thickness);
        el(&mut o, "Action", &r.action);
        el(&mut o, "Power", &r.power.to_string());
        el(&mut o, "Speed", &r.speed.to_string());
        el(&mut o, "Cycles", &r.cycles.to_string());
        if let Some(rm) = &r.remarks {
            el(&mut o, "Remarks", rm);
        }
        o.push_str("  </Materials>\n");
    }
    o.push_str("</MaterialDB>");
    o
}

/// C# `MaterialDB` DataSet.
#[derive(Debug, Clone, Default)]
pub struct MaterialDb {
    /// `Materials` table.
    pub materials: MaterialsTable,
    /// `FromServer`: server rows the user file does not know.
    from_server: Vec<MaterialRow>,
    user_file: PathBuf,
}

impl MaterialDb {
    /// `UserFile`: `<data>/UserMaterials.psh`.
    pub fn user_file() -> PathBuf {
        lg_base::paths::data_path().join("UserMaterials.psh")
    }

    /// `ServerFile`: `<data>/StandardMaterials.psh` (where an updater would store it).
    pub fn server_file() -> PathBuf {
        lg_base::paths::data_path().join("StandardMaterials.psh")
    }

    /// Server file actually read by [`load`](Self::load): [`server_file`](Self::server_file) when
    /// present, else the bundled `resources/misc/StandardMaterials.psh`.
    pub fn effective_server_file() -> PathBuf {
        let s = Self::server_file();
        if s.exists() { s } else { lg_base::paths::resources_dir().join("misc").join("StandardMaterials.psh") }
    }

    /// C# `MaterialDB.Load()` with the default files.
    pub fn load() -> Self {
        Self::load_from(&Self::user_file(), &Self::effective_server_file())
    }

    /// `Load()` with explicit files.
    ///
    /// `user_file`: user DB (also where [`save_changes`](Self::save_changes) writes). `server_file`:
    /// standard DB. Returns the DB; user rows win, server rows unknown to the user file are
    /// imported (and saved) like C#.
    pub fn load_from(user_file: &Path, server_file: &Path) -> Self {
        let mut rv = MaterialDb { user_file: user_file.to_path_buf(), ..Default::default() };
        // C# swallows every exception of the whole block
        let _ = rv.load_inner(user_file, server_file);
        rv.materials.accept_changes();
        if rv.get_new_count() > 0 {
            rv.import_server();
            rv.save_changes();
        }
        rv
    }

    fn load_inner(&mut self, user_file: &Path, server_file: &Path) -> Result<(), MaterialError> {
        let user = if user_file.exists() { read_xml(user_file)? } else { Vec::new() };
        let server = if server_file.exists() { read_xml(server_file)? } else { Vec::new() };
        for r in &user {
            self.materials.import_row(r)?;
        }
        if self.materials.rows.is_empty() {
            for r in &server {
                self.materials.import_row(r)?;
            }
        } else {
            let mut fs = MaterialsTable::default();
            for r in &server {
                if self.materials.find_by_id(&r.id).is_none() {
                    fs.import_row(r)?;
                }
            }
            self.from_server = fs.rows;
        }
        Ok(())
    }

    /// C# `SaveChanges`: write the user file when there are pending changes.
    pub fn save_changes(&mut self) {
        if self.materials.has_changes() {
            self.materials.accept_changes();
            let _ = std::fs::write(&self.user_file, write_xml(&self.materials.rows));
        }
    }

    /// C# `GetNewCount`: server rows not yet in the table.
    pub fn get_new_count(&self) -> usize {
        self.from_server.iter().filter(|r| self.materials.find_by_id(&r.id).is_none()).count()
    }

    /// C# `ImportServer`: add server rows missing from the table; stops at the first
    /// constraint violation like the C# try/catch.
    pub fn import_server(&mut self) {
        for r in self.from_server.clone() {
            if self.materials.find_by_id(&r.id).is_none() && self.materials.import_row(&r).is_err() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("lg_services_mdb_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn std_file() -> PathBuf {
        root().join("resources/misc/StandardMaterials.psh")
    }

    #[test]
    fn standard_file_roundtrips_byte_exact() {
        let text = std::fs::read_to_string(std_file()).unwrap();
        let rows = parse_xml(&text).unwrap();
        assert_eq!(rows.len(), 906);
        assert_eq!(write_xml(&rows), text.replace("\r\n", "\n"));
    }

    #[test]
    fn queries_match_mono() {
        let d = scratch("q");
        let db = MaterialDb::load_from(&d.join("u.psh"), &std_file());
        assert_eq!(db.materials.rows().len(), 906);
        let gold = std::fs::read_to_string(root().join("tests/golden/services/materials_queries.txt")).unwrap();
        let line = |p: &str| gold.lines().find(|l| l.starts_with(p)).unwrap().split_once('\t').unwrap().1.to_string();
        let models = db.materials.models();
        assert_eq!(models.join("|"), line("models"));
        let m0 = &models[0];
        let mats = db.materials.materials(m0);
        assert_eq!(mats.join("|"), line("materials("));
        let acts = db.materials.actions(m0, &mats[0]);
        assert_eq!(acts.join("|"), line("actions("));
        let th = db.materials.thickness(m0, &mats[0], &acts[0]);
        assert_eq!(th.join("|"), line("thickness"));
        let r = db.materials.get_result(m0, &mats[0], &acts[0], &th[0]).unwrap();
        assert_eq!(format!("{}\t{}\t{}\t{}\t{}", r.id, r.power, r.speed, r.cycles, r.remarks()), line("result"));
    }

    #[test]
    fn edit_save_matches_mono_file_and_reload() {
        let d = scratch("save");
        let user = d.join("UserMaterials.psh");
        let mut db = MaterialDb::load_from(&user, &std_file());
        // not saved on plain load
        assert!(!user.exists());
        // same edit sequence as the Mono generator
        let mut r0 = db.materials.rows()[0].clone();
        r0.power = 55;
        db.materials.update_row(0, r0).unwrap();
        let mut a = db.materials.new_row();
        a.id = "11111111-2222-3333-4444-555555555555".into();
        a.model = "Test <Laser> & Co".into();
        a.material = "Mat \"x\"".into();
        a.thickness = "2mm".into();
        a.action = "Cut".into();
        a.power = 80;
        a.speed = 300;
        a.cycles = 2;
        a.remarks = Some("line1\nline2".into());
        db.materials.add_row(a).unwrap();
        let mut b = db.materials.new_row();
        b.id = "11111111-2222-3333-4444-555555555556".into();
        b.visible = false;
        b.model = "Test <Laser> & Co".into();
        b.material = "Mat2".into();
        b.action = "Engrave".into();
        b.power = 10;
        b.speed = 3000;
        b.remarks = Some(String::new());
        db.materials.add_row(b).unwrap();
        assert!(db.materials.delete_row(1));
        db.save_changes();
        assert!(!db.materials.has_changes());
        let ours = std::fs::read_to_string(&user).unwrap();
        let gold = std::fs::read_to_string(root().join("tests/golden/services/materials_saved.psh")).unwrap();
        assert_eq!(ours, gold);
        // reload: deleted server row comes back as "new"
        let db2 = MaterialDb::load_from(&user, &std_file());
        let rg = std::fs::read_to_string(root().join("tests/golden/services/materials_reload.txt")).unwrap();
        let rows: usize = rg.lines().next().unwrap().split('\t').nth(1).unwrap().parse().unwrap();
        assert_eq!(db2.materials.rows().len(), rows);
        assert_eq!(db2.get_new_count(), 0);
    }

    #[test]
    fn column_limits() {
        let mut t = MaterialsTable::default();
        let mut r = t.new_row();
        assert_eq!((r.model.as_str(), r.power, r.speed, r.cycles), ("Generic Laser", 100, 1000, 1));
        r.power = 0;
        assert_eq!(t.add_row(r.clone()).unwrap_err().0, "Please enter a valid power level (Min 1%)");
        r.power = 101;
        assert_eq!(t.add_row(r.clone()).unwrap_err().0, "Please enter a valid power level (Max 100%)");
        r.power = 50;
        r.speed = 100001;
        assert_eq!(t.add_row(r.clone()).unwrap_err().0, "Please enter a valid speed (Max 100000 mm/min)");
        r.speed = 5;
        r.cycles = 100;
        assert_eq!(t.add_row(r.clone()).unwrap_err().0, "Please enter a valid cycles number (Max 99 cycles)");
        r.cycles = 99;
        t.add_row(r).unwrap();
        assert!(t.has_changes());
    }

    #[test]
    fn unique_constraints() {
        let mut t = MaterialsTable::default();
        let mut r = t.new_row();
        r.action = "Cut".into();
        t.add_row(r.clone()).unwrap();
        // same id
        assert!(t.add_row(r.clone()).is_err());
        // other id, same record key (case-insensitive)
        let mut r2 = t.new_row();
        r2.action = "CUT".into();
        assert!(t.add_row(r2).is_err());
        // new row copies model/material of the last row
        let r3 = t.new_row();
        assert_eq!(r3.material, "Generic Material");
    }

    #[test]
    fn invisible_rows_not_listed() {
        let mut t = MaterialsTable::default();
        let mut r = t.new_row();
        r.visible = false;
        t.add_row(r).unwrap();
        assert!(t.models().is_empty());
    }

    #[test]
    fn culture_order_samples() {
        assert_eq!(culture_compare("Non_Woven_Fabrics", "Non-Woven Fabrics (dark color)"), Ordering::Less);
        assert_eq!(culture_compare("abc", "ABC"), Ordering::Less);
        assert_eq!(culture_compare("ABS", "Acrylic"), Ordering::Less);
        assert_eq!(culture_compare("1", "a"), Ordering::Less);
    }

    #[test]
    fn empty_db_xml() {
        assert_eq!(write_xml(&[]), "<?xml version=\"1.0\" standalone=\"yes\"?>\n<MaterialDB xmlns=\"http://tempuri.org/MaterialDB.xsd\" />");
        assert!(parse_xml(&write_xml(&[])).unwrap().is_empty());
    }

    #[test]
    fn broken_user_file_gives_empty_db() {
        let d = scratch("broken");
        std::fs::write(d.join("u.psh"), "<not xml").unwrap();
        let db = MaterialDb::load_from(&d.join("u.psh"), &std_file());
        assert!(db.materials.rows().is_empty());
    }
}
