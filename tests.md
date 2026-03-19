### Unoptimised (cargo run)
test set.json \
4 threads
#### OpenCV Align + Blender Merge + OpenCV tonemap
46.09s
#### OpenCV Align + OpenCV Merge (Debevec) + OpenCV tonemap
59.73s
#### OpenCV Align + Rust Merge (Zaal) + OpenCV tonemap
310.91s

### Optimised (cargo run --release)
#### OpenCV Align + Blender Merge + OpenCV tonemap
32.09
#### OpenCV Align + OpenCV Merge (Debevec) + OpenCV tonemap
34.45
#### OpenCV Align + Rust Merge (Zaal) + OpenCV tonemap
37.39s

### Serial refactor (unoptimised) (cargo run)

#### OpenCV Align + Rust Merge (Zaal) + OpenCV tonemap
318.47s / 6m 3s - 8 threads \

### Serial refactor (optimised) (cargo run --release)

#### OpenCV Align + Rust Merge (Zaal) + OpenCV tonemap
53.98
### Serial parallel (unoptimised) (cargo run)

#### OpenCV Align + Rust Merge (Zaal) + OpenCV tonemap
105.05s / 2m 11s - 8 threads

### Serial parallel (optimized) (cargo run --release)

#### OpenCV Align + Rust Merge (Zaal) + OpenCV tonemap
25.10s - 8 threads