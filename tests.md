### Unoptimised (cargo run debug)
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

### Serial refactor (unoptimised)

#### OpenCV Align + Rust Merge (Zaal) + OpenCV tonemap
318.47s / 6m 3s - 8 threads \

### Serial refactor (unoptimised)

#### OpenCV Align + Rust Merge (Zaal) + OpenCV tonemap
