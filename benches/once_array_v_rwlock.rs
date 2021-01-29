// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use criterion::{
    criterion_group,
    Criterion,
};
use parking_lot::RwLock;
use vase::sync::OnceArray;

const TEST_SIZE: usize = 100;

fn once_array_write(item: String) -> OnceArray<String> {
    let oa = OnceArray::new();
    for _ in 0..TEST_SIZE {
        oa.push(item.clone());
    }
    oa
}

fn rwlock_write(item: String) -> RwLock<Vec<String>> {
    let rw = RwLock::new(Vec::new());
    for _ in 0..TEST_SIZE {
        rw.write().push(item.clone());
    }
    rw
}

fn once_array_read(arr: &OnceArray<String>) -> u32 {
    let mut accum = 0u32;
    for i in 0..TEST_SIZE as u32 {
        accum += arr[i].len() as u32;
    }
    accum
}

fn rwlock_read(arr: &RwLock<Vec<String>>) -> usize {
    let mut accum = 0usize;
    for i in 0..TEST_SIZE {
        accum += arr.read().get(i).unwrap().len();
    }
    accum
}

fn bench_comparison(c: &mut Criterion) {
    const TEST_VAL: &str = "TEST";

    let mut group = c.benchmark_group("OA v RW");
    group.bench_function("OnceArray Write", |b| {
        b.iter(|| once_array_write(String::from(TEST_VAL)));
    });
    group.bench_function("RwLock Write", |b| {
        b.iter(|| rwlock_write(String::from(TEST_VAL)));
    });
    group.bench_function("OnceArray Read", |b| {
        let arr = once_array_write(String::from(TEST_VAL));
        b.iter(|| once_array_read(&arr));
    });
    group.bench_function("RwLock Read", |b| {
        let arr = rwlock_write(String::from(TEST_VAL));
        b.iter(|| rwlock_read(&arr));
    });
}

criterion_group!(comparisons, bench_comparison);
