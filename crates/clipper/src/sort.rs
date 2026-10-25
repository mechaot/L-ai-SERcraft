// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2010-2017 Angus Johnson (Clipper)
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)
// Clipper parts: Boost Software License 1.0, see NOTICE.md

//! .NET `List<T>.Sort(IComparer)` (introsort from referencesource `ArraySortHelper`).
//! Not stable, so tie order must match .NET exactly: port the algorithm.

const INTROSORT_SIZE_THRESHOLD: usize = 16;

/// Bit length of `n` (`IntrospectiveSortUtilities.FloorLog2`).
fn floor_log2(mut n: usize) -> usize {
    let mut result = 0;
    while n >= 1 {
        result += 1;
        n /= 2;
    }
    result
}

/// Sort like `List<T>.Sort(comparer)`.
///
/// `keys` is the slice to sort. `capacity` is the List's backing array length (drives depth
/// limit). `cmp` returns <0, 0, >0 like `IComparer.Compare`.
pub(crate) fn list_sort<T: Copy>(keys: &mut [T], capacity: usize, cmp: &dyn Fn(&T, &T) -> i32) {
    let len = keys.len();
    if len < 2 {
        return;
    }
    intro_sort(keys, 0, len - 1, 2 * floor_log2(capacity.max(len)), cmp);
}

fn swap_if_greater<T: Copy>(keys: &mut [T], cmp: &dyn Fn(&T, &T) -> i32, a: usize, b: usize) {
    if a != b && cmp(&keys[a], &keys[b]) > 0 {
        keys.swap(a, b);
    }
}

fn intro_sort<T: Copy>(
    keys: &mut [T],
    lo: usize,
    mut hi: usize,
    mut depth_limit: usize,
    cmp: &dyn Fn(&T, &T) -> i32,
) {
    while hi > lo {
        let partition_size = hi - lo + 1;
        if partition_size <= INTROSORT_SIZE_THRESHOLD {
            if partition_size == 1 {
                return;
            }
            if partition_size == 2 {
                swap_if_greater(keys, cmp, lo, hi);
                return;
            }
            if partition_size == 3 {
                swap_if_greater(keys, cmp, lo, hi - 1);
                swap_if_greater(keys, cmp, lo, hi);
                swap_if_greater(keys, cmp, hi - 1, hi);
                return;
            }
            insertion_sort(keys, lo, hi, cmp);
            return;
        }
        if depth_limit == 0 {
            heap_sort(keys, lo, hi, cmp);
            return;
        }
        depth_limit -= 1;
        let p = pick_pivot_and_partition(keys, lo, hi, cmp);
        intro_sort(keys, p + 1, hi, depth_limit, cmp);
        // p >= lo + 1 here, no underflow
        hi = p - 1;
    }
}

fn pick_pivot_and_partition<T: Copy>(
    keys: &mut [T],
    lo: usize,
    hi: usize,
    cmp: &dyn Fn(&T, &T) -> i32,
) -> usize {
    let middle = lo + (hi - lo) / 2;
    swap_if_greater(keys, cmp, lo, middle);
    swap_if_greater(keys, cmp, lo, hi);
    swap_if_greater(keys, cmp, middle, hi);

    let pivot = keys[middle];
    keys.swap(middle, hi - 1);
    let mut left = lo;
    let mut right = hi - 1;
    while left < right {
        loop {
            left += 1;
            if cmp(&keys[left], &pivot) >= 0 {
                break;
            }
        }
        loop {
            right -= 1;
            if cmp(&pivot, &keys[right]) >= 0 {
                break;
            }
        }
        if left >= right {
            break;
        }
        keys.swap(left, right);
    }
    keys.swap(left, hi - 1);
    left
}

fn insertion_sort<T: Copy>(keys: &mut [T], lo: usize, hi: usize, cmp: &dyn Fn(&T, &T) -> i32) {
    for i in lo..hi {
        let mut j = i as isize;
        let t = keys[i + 1];
        while j >= lo as isize && cmp(&t, &keys[j as usize]) < 0 {
            keys[(j + 1) as usize] = keys[j as usize];
            j -= 1;
        }
        keys[(j + 1) as usize] = t;
    }
}

fn heap_sort<T: Copy>(keys: &mut [T], lo: usize, hi: usize, cmp: &dyn Fn(&T, &T) -> i32) {
    let n = hi - lo + 1;
    let mut i = n / 2;
    while i >= 1 {
        down_heap(keys, i, n, lo, cmp);
        i -= 1;
    }
    let mut i = n;
    while i > 1 {
        keys.swap(lo, lo + i - 1);
        down_heap(keys, 1, i - 1, lo, cmp);
        i -= 1;
    }
}

fn down_heap<T: Copy>(
    keys: &mut [T],
    mut i: usize,
    n: usize,
    lo: usize,
    cmp: &dyn Fn(&T, &T) -> i32,
) {
    let d = keys[lo + i - 1];
    while i <= n / 2 {
        let mut child = 2 * i;
        if child < n && cmp(&keys[lo + child - 1], &keys[lo + child]) < 0 {
            child += 1;
        }
        if cmp(&d, &keys[lo + child - 1]) >= 0 {
            break;
        }
        keys[lo + i - 1] = keys[lo + child - 1];
        i = child;
    }
    keys[lo + i - 1] = d;
}
