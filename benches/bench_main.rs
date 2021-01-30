// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use criterion::criterion_main;

criterion_main! {
    once_array_v_rwlock::comparisons,
}

mod once_array_v_rwlock;
