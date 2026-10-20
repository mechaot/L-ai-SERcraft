//! Port of `AutoUpdate/GitHub.cs`: version helpers only.
//!
//! Network update check and installer download are DISABLED by design (PLAN section 6: upstream
//! releases are Windows installers; no calls to the upstream author's servers). The pure parts are
//! ported: `OnlineVersion` JSON model, `System.Version` ordering, the release selection of
//! `CheckVersionFromSite`, `InitFlags`. `CleanupOldVersion` (deletes `*.todelete` below the cwd)
//! is N/A on Linux.

use crate::json::{parse_list_elements, parse_string_value, split_list, split_object};
use std::cmp::Ordering;
use std::path::PathBuf;

/// `System.Version`. Missing components are -1 like .NET (`Build`, `Revision`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version {
    pub major: i32,
    pub minor: i32,
    pub build: i32,
    pub revision: i32,
}

impl Version {
    /// `new Version(string)`: 2 to 4 non-negative integer parts.
    ///
    /// `s`: text like `7.14.1` or `7.14.1.2`. Returns the version or None (C# throws).
    pub fn parse(s: &str) -> Option<Version> {
        let parts: Vec<&str> = s.trim().split('.').collect();
        if !(2..=4).contains(&parts.len()) {
            return None;
        }
        let mut n = [-1i32; 4];
        for (i, p) in parts.iter().enumerate() {
            let v: i32 = p.trim().parse().ok()?;
            if v < 0 {
                return None;
            }
            n[i] = v;
        }
        Some(Version { major: n[0], minor: n[1], build: n[2], revision: n[3] })
    }

    /// Version from major/minor/build (like `new Version(a,b,c)`).
    ///
    /// `major`,`minor`,`build`: parts. Returns the version (revision -1).
    pub fn new3(major: i32, minor: i32, build: i32) -> Version {
        Version { major, minor, build, revision: -1 }
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, o: &Self) -> Ordering {
        (self.major, self.minor, self.build, self.revision).cmp(&(o.major, o.minor, o.build, o.revision))
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)?;
        if self.build >= 0 {
            write!(f, ".{}", self.build)?;
            if self.revision >= 0 {
                write!(f, ".{}", self.revision)?;
            }
        }
        Ok(())
    }
}

/// Release asset (C# `OnlineVersion.VersionAsset`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VersionAsset {
    pub browser_download_url: Option<String>,
}

/// Release entry of the GitHub API (C# `OnlineVersion`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OnlineVersion {
    pub html_url: Option<String>,
    pub tag_name: Option<String>,
    pub name: Option<String>,
    pub prerelease: bool,
    pub assets: Option<Vec<VersionAsset>>,
}

impl OnlineVersion {
    /// C# `IsValid`: version parses and a download url exists.
    pub fn is_valid(&self) -> bool {
        self.version().is_some() && self.download_url().is_some()
    }

    /// C# `IsPreRelease`.
    pub fn is_pre_release(&self) -> bool {
        self.prerelease
    }

    /// C# `VersionName` (the tag).
    pub fn version_name(&self) -> Option<&str> {
        self.tag_name.as_deref()
    }

    /// C# `Version`: tag without leading `v`s.
    ///
    /// Returns the parsed version or None.
    pub fn version(&self) -> Option<Version> {
        Version::parse(self.tag_name.as_deref()?.trim_start_matches('v'))
    }

    /// C# `DownloadUrl`: first asset url.
    pub fn download_url(&self) -> Option<&str> {
        self.assets.as_ref()?.first()?.browser_download_url.as_deref()
    }

    /// C# `HtmlUrl`.
    pub fn html_url(&self) -> Option<&str> {
        self.html_url.as_deref()
    }
}

impl std::fmt::Display for OnlineVersion {
    /// C# `ToString`: `"{version}{-pre}{ (not available)}"`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let v = self.version().map(|v| v.to_string()).unwrap_or_default();
        write!(f, "{}{}{}", v, if self.is_pre_release() { "-pre" } else { "" }, if self.is_valid() { "" } else { " (not available)" })
    }
}

/// Typed parse of one release object (C# reflection `ParseObject` for `OnlineVersion`).
///
/// `c`: compact object text. Returns the release; unknown members are ignored, names match
/// case-insensitively like the C# member dictionaries.
fn parse_online_version(c: &[char]) -> Option<OnlineVersion> {
    let mut ov = OnlineVersion::default();
    for (k, v) in split_object(c) {
        match k.to_lowercase().as_str() {
            "html_url" => ov.html_url = parse_string_value(&v),
            "tag_name" => ov.tag_name = parse_string_value(&v),
            "name" => ov.name = parse_string_value(&v),
            // Convert.ChangeType(json, bool) throws on garbage
            "prerelease" => ov.prerelease = v.iter().collect::<String>().eq_ignore_ascii_case("true"),
            "assets" => {
                ov.assets = split_list(&v).map(|els| {
                    els.iter()
                        .map(|e| {
                            let mut a = VersionAsset::default();
                            for (k2, v2) in split_object(e) {
                                if k2.eq_ignore_ascii_case("browser_download_url") {
                                    a.browser_download_url = parse_string_value(&v2);
                                }
                            }
                            a
                        })
                        .collect()
                });
            }
            _ => {}
        }
    }
    Some(ov)
}

/// `FromJson<List<OnlineVersion>>`.
///
/// `json`: GitHub releases API text. Returns the list, or None when it is no JSON array.
pub fn parse_online_versions(json: &str) -> Option<Vec<OnlineVersion>> {
    let els = parse_list_elements(json)?;
    Some(els.iter().filter_map(|e| parse_online_version(e)).collect())
}

/// Release selection of `CheckVersionFromSite` (no network).
///
/// `versions`: releases, newest first. `current`: running version. `build`: accept build
/// releases (setting "Auto Update Build" or manual check). `pre`: accept pre-releases (setting
/// "Auto Update Pre"). Returns the best newer release or None.
pub fn select_update<'a>(versions: &'a [OnlineVersion], current: Version, build: bool, pre: bool) -> Option<&'a OnlineVersion> {
    let mut found: Option<&OnlineVersion> = None;
    for ov in versions {
        // skip invalid
        if !ov.is_valid() {
            continue;
        }
        let Some(v) = ov.version() else { continue };
        // skip pre-release
        if !pre && ov.is_pre_release() {
            continue;
        }
        // NOTE(upstream): a 2-part tag has Build == -1 so it counts as "build version" too
        if !build && v.build != 0 {
            continue;
        }
        // already found a better match
        if let Some(f) = found
            && f.version().is_some_and(|fv| v < fv)
        {
            continue;
        }
        if v > current {
            found = Some(ov);
        }
    }
    found
}

/// C# `GitHub.CheckVersion(manual)`: DISABLED (PLAN section 6). Never touches the network.
///
/// `manual`: unused. Returns false (nothing started).
pub fn check_version(_manual: bool) -> bool {
    false
}

/// C# `GitHub.installer`: path of the downloaded updater (unused, update disabled).
///
/// Returns `<temp>/LaserGRBL Updater.exe`.
pub fn installer() -> PathBuf {
    lg_base::paths::temp_path().join("LaserGRBL Updater.exe")
}

/// C# `InitFlags` (part of `InitUpdate`): create "Auto Update Build" once.
///
/// `rng_percent`: 0..99 random draw (C# `rng.Next(0, 100)`); pass any value, the flag is only
/// set when "Auto Update" is on and the draw is below 2.
pub fn init_flags(rng_percent: i32) {
    use lg_base::settings;
    if !settings::exists("Auto Update Build") {
        let percentage = 2;
        let auto: bool = settings::get("Auto Update", true);
        settings::set("Auto Update Build", auto && rng_percent < percentage);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[
 {"html_url":"https://x/1","tag_name":"v7.15.0","name":"n","prerelease":false,"assets":[{"browser_download_url":"https://x/a.exe"}]},
 {"html_url":"https://x/2","tag_name":"v7.15.1","name":"n","prerelease":false,"assets":[{"browser_download_url":"https://x/b.exe"}]},
 {"html_url":"https://x/3","tag_name":"v7.16.0","name":"n","prerelease":true,"assets":[{"browser_download_url":"https://x/c.exe"}]},
 {"tag_name":"v9.0.0","assets":[]}
]"#;

    #[test]
    fn version_parse_order() {
        let a = Version::parse("7.14.1").unwrap();
        let b = Version::parse("7.14.1.0").unwrap();
        assert!(a < b);
        assert!(Version::parse("7.15").unwrap() > b);
        assert!(Version::parse("7").is_none());
        assert_eq!(a.to_string(), "7.14.1");
    }

    #[test]
    fn parse_and_select() {
        let v = parse_online_versions(SAMPLE).unwrap();
        assert_eq!(v.len(), 4);
        assert!(v[0].is_valid() && !v[3].is_valid());
        assert_eq!(v[0].to_string(), "7.15.0");
        assert_eq!(v[3].to_string(), "9.0.0 (not available)");
        let cur = Version::new3(7, 14, 1);
        // build releases (Build != 0) are skipped without the flag
        assert_eq!(select_update(&v, cur, false, false).unwrap().tag_name.as_deref(), Some("v7.15.0"));
        assert_eq!(select_update(&v, cur, true, false).unwrap().tag_name.as_deref(), Some("v7.15.1"));
        assert_eq!(select_update(&v, cur, true, true).unwrap().tag_name.as_deref(), Some("v7.16.0"));
    }

    #[test]
    fn not_an_array() {
        assert!(parse_online_versions("{}").is_none());
    }
}
