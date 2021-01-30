// Copyright 2021. remilia-dev
// This source code is licensed under GPLv3 or any later version.
use std::ops::Deref;

use arc_swap::ArcSwapOption;
use criterion::{
    black_box,
    criterion_group,
    Criterion,
};
use vase::{
    sync::{
        Arc,
        AtomicArc,
        AtomicBox,
        Ordering,
    },
    util::mem::make_static_array,
};

const TEST_SIZE: usize = 100;

fn atomic_box_write(num: usize) -> [AtomicBox<usize>; TEST_SIZE] {
    let arr = make_static_array::<_, TEST_SIZE>(&|| AtomicBox::<usize>::empty());
    for index in 0..arr.len() {
        arr[index].set_if_none(Box::new(index + num), Ordering::SeqCst, Ordering::SeqCst);
    }
    arr
}

fn arc_swap_write(num: usize) -> [ArcSwapOption<usize>; TEST_SIZE] {
    let arr = make_static_array::<_, TEST_SIZE>(&|| ArcSwapOption::<usize>::empty());
    for index in 0..arr.len() {
        arr[index].compare_and_swap(&None::<Option<Arc<usize>>>, Some(Arc::new(index + num)));
    }
    arr
}

fn atomic_arc_write(num: usize) -> [AtomicArc<usize>; TEST_SIZE] {
    let arr = make_static_array::<_, TEST_SIZE>(&|| AtomicArc::<usize>::empty());
    for index in 0..arr.len() {
        arr[index].set_if_none(Arc::new(index + num), Ordering::SeqCst, Ordering::SeqCst);
    }
    arr
}

fn atomic_box_read(arr: &[AtomicBox<usize>; TEST_SIZE]) -> usize {
    let mut accum = 0;
    for item in arr {
        accum += match item.load(Ordering::SeqCst) {
            Some(v) => *v,
            None => 0,
        };
    }
    std::mem::forget(arr);
    accum
}

fn arc_swap_read(arr: &[ArcSwapOption<usize>; TEST_SIZE]) -> usize {
    let mut accum = 0;
    for item in arr {
        accum += match item.load().deref() {
            Some(v) => **v,
            None => 0,
        };
    }
    accum
}

fn atomic_arc_read(arr: &[AtomicArc<usize>; TEST_SIZE]) -> usize {
    let mut accum = 0;
    for item in arr {
        accum += match item.load(Ordering::SeqCst) {
            Some(v) => *v,
            None => 0,
        };
    }
    accum
}

fn bench_comparison(c: &mut Criterion) {
    const TEST_NUM: usize = 1;

    let mut group = c.benchmark_group("AB v AS");
    group.bench_function("AtomicBox Write", |b| {
        b.iter(|| atomic_box_write(black_box(TEST_NUM)));
    });
    group.bench_function("AtomicArc Write", |b| {
        b.iter(|| atomic_arc_write(black_box(TEST_NUM)));
    });
    group.bench_function("ArcSwapOption Write", |b| {
        b.iter(|| arc_swap_write(TEST_NUM));
    });

    group.bench_function("AtomicBox Read", |b| {
        let arr = atomic_box_write(TEST_NUM);
        b.iter(|| atomic_box_read(black_box(&arr)));
    });
    group.bench_function("AtomicArc Read", |b| {
        let arr = atomic_arc_write(TEST_NUM);
        b.iter(|| atomic_arc_read(black_box(&arr)));
    });
    group.bench_function("ArcSwapOption Read", |b| {
        let arr = arc_swap_write(TEST_NUM);
        b.iter(|| arc_swap_read(black_box(&arr)));
    });
}

criterion_group!(comparisons, bench_comparison);
