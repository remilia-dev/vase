// NOTE: This file just represents an early benchmark of my StringCache that I used in its development.
fn main() {
    let cache = util::StringCache::new();

    let files: Vec<_> =
        walkdir::WalkDir::new("../linux-master/")
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .collect();

    let count = std::sync::atomic::AtomicI64::new(0);
    files.par_iter().for_each(|entry: &DirEntry| {
        let file_path = match entry.file_name().to_str() {
            Some(filename) => {
                if !(filename.ends_with(".c") || filename.ends_with(".h")) {
                    return;
                };
                entry.path()
            },
            None => return,
        };

        let file = match std::fs::File::open(file_path) {
            Ok(x) => x,
            Err(_) => return,
        };

        let mmap = match unsafe { memmap::MmapOptions::new().map(&file) } {
            Ok(x) => x,
            Err(_) => return,
        };

        let mut start: usize = 0;
        for index in 0..mmap.len() {
            let c = mmap[index];
            if (48 <= c && c <= 57) || (65 <= c && c <= 90) || (97 <= c && c <= 122) {
                continue;
            } else if start >= index {
                start = index + 1;
                continue
            }

            let segment = match std::str::from_utf8(&mmap[start..index]) {
                Ok(x) => x,
                Err(e) => panic!(e),
            };
            start = index + 1;

            let cached = cache.get_or_cache(segment);
            assert_eq!(cached.string().as_bytes(), segment.as_bytes(), "{} {}", cached.string(), segment);
            count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    });

    println!("Count: {}", count.load(std::sync::atomic::Ordering::SeqCst));
}