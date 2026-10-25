// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `Tools/RetainedSetting.cs`.

use serde::{de::DeserializeOwned, Serialize};

/// Change callback.
type OnChange<T> = Box<dyn FnMut(&T) + Send>;

/// Value mirrored to settings on change.
pub struct RetainedSetting<T> {
    name: String,
    value: T,
    on_change: Option<OnChange<T>>,
}

impl<T: Serialize + DeserializeOwned + PartialEq + Default> RetainedSetting<T> {
    /// Load from settings.
    ///
    /// `name`: setting key. `default`: used if missing. Like C#, the setter only writes back when the
    /// loaded value differs from `T::default()`.
    pub fn new(name: &str, default: T) -> Self {
        let mut s = Self { name: name.to_string(), value: T::default(), on_change: None };
        s.set(crate::settings::get(name, default));
        s
    }

    /// Current value.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Set value; when different store in settings and call the change callback.
    ///
    /// `v`: new value.
    pub fn set(&mut self, v: T) {
        if self.value != v {
            self.value = v;
            crate::settings::set(&self.name, &self.value);
            if let Some(cb) = self.on_change.as_mut() {
                cb(&self.value);
            }
        }
    }

    /// Register the `OnChange` callback.
    ///
    /// `f`: called with the new value.
    pub fn set_on_change(&mut self, f: OnChange<T>) {
        self.on_change = Some(f);
    }
}
