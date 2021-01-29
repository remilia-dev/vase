// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use criterion::criterion_main;

criterion_main! {
    atomic_box_v_arc_swap::comparisons,
    once_array_v_rwlock::comparisons,
}

mod atomic_box_v_arc_swap;
mod once_array_v_rwlock;
