// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2016-2025 Diego Settimi and LaserGRBL contributors (original C# code)
// Copyright (C) 2026 Andreas Pösch (Rust port and changes)

//! Port of `OptimizePaths` / `ParallelOptimizePaths` of `GrblFile.cs`: nearest-neighbour ordering
//! of potrace path lists.

use crate::{GResult, GcodeError};
use lg_potrace::{Curve, DPoint};

/// Squared distance between two points.
///
/// `a`,`b`: points.
fn square_distance(a: DPoint, b: DPoint) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    (dx * dx) + (dy * dy)
}

/// Squared distance from origin.
///
/// `a`: point.
fn square_distance_zero(a: DPoint) -> f64 {
    (a.x * a.x) + (a.y * a.y)
}

/// C# `OptimizePaths`: start at the path nearest to (0,0), then always go to the unvisited path
/// whose first point is nearest to the end of the current path (first wins on ties).
///
/// `list`: paths. `_changecost`: unused in C# (direction change cost, disabled). Returns the new
/// order; Err where C# `First()/Last()` would throw on an empty path.
pub fn optimize_paths(list: Vec<Vec<Curve>>, _changecost: f64) -> GResult<Vec<Vec<Curve>>> {
    if list.len() <= 1 {
        return Ok(list);
    }
    let empty = || GcodeError::Other("InvalidOperationException: Sequence contains no elements".into());
    let n = list.len();
    let mut first_a = Vec::with_capacity(n);
    let mut last_b = Vec::with_capacity(n);
    let mut nearest_to_zero = 0usize;
    let mut best_distance_to_zero = f64::MAX;
    for (c1, path) in list.iter().enumerate() {
        let fa = path.first().ok_or_else(empty)?.a;
        let lb = path.last().ok_or_else(empty)?.b;
        first_a.push(fa);
        last_b.push(lb);
        // path starting nearest to zero
        let dz = square_distance_zero(fa);
        if dz < best_distance_to_zero {
            nearest_to_zero = c1;
            best_distance_to_zero = dz;
        }
    }

    let mut unvisited: Vec<usize> = (0..n).collect();
    let mut order = vec![nearest_to_zero];
    unvisited.retain(|&i| i != nearest_to_zero);
    let mut last_index = nearest_to_zero;

    while !unvisited.is_empty() {
        let mut best_index = 0usize;
        let mut best_distance = f64::MAX;
        for &next in &unvisited {
            // costs[last, next]; the diagonal (MaxValue) never competes: last is not unvisited
            let dist = square_distance(last_b[last_index], first_a[next]);
            if dist < best_distance {
                best_index = next;
                best_distance = dist;
            }
        }
        order.push(best_index);
        match unvisited.iter().position(|&i| i == best_index) {
            Some(p) => {
                unvisited.remove(p);
            }
            // NOTE(upstream): NaN coordinates make C# loop forever; keep the rest in order instead
            None => {
                order.append(&mut unvisited);
                break;
            }
        }
        last_index = best_index;
    }

    let mut slots: Vec<Option<Vec<Curve>>> = list.into_iter().map(Some).collect();
    Ok(order.into_iter().filter_map(|i| slots[i].take()).collect())
}

/// C# `ParallelOptimizePaths`: blocks of 2048 paths optimized independently (in parallel) and
/// concatenated in block order. Result identical to a sequential run of the same blocks.
///
/// `list`: paths (None = C# null, returned as is). `changecost`: unused. Returns the ordering.
pub fn parallel_optimize_paths(list: Option<Vec<Vec<Curve>>>, changecost: f64) -> GResult<Option<Vec<Vec<Curve>>>> {
    let Some(list) = list else { return Ok(None) };
    if list.len() <= 1 {
        return Ok(Some(list));
    }
    let maxblocksize = 2048usize;
    let blocknum = (list.len() as f64 / maxblocksize as f64).ceil() as usize;
    if blocknum <= 1 {
        return optimize_paths(list, changecost).map(Some);
    }
    let count = list.len();
    let jobs: Vec<Vec<Vec<Curve>>> = (0..blocknum)
        .map(|i| {
            let from = (i * count) / blocknum;
            let to = ((i + 1) * count) / blocknum;
            list[from..to].to_vec()
        })
        .collect();
    let results: Vec<GResult<Vec<Vec<Curve>>>> = std::thread::scope(|s| {
        let handles: Vec<_> = jobs.into_iter().map(|job| s.spawn(move || optimize_paths(job, changecost))).collect();
        handles.into_iter().map(|h| h.join().unwrap_or_else(|_| Err(GcodeError::Other("optimizer thread panicked".into())))).collect()
    });
    let mut rv = Vec::with_capacity(count);
    for r in results {
        rv.extend(r?);
    }
    Ok(Some(rv))
}
