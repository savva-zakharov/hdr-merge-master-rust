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
32.09s - 4 threads \
30.44s - 8 threads
#### OpenCV Align + OpenCV Merge (Debevec) + OpenCV tonemap
34.45s - 4 threads \
31.26s - 8 threads 
#### OpenCV Align + Rust Merge (Zaal) + OpenCV tonemap
37.39s - 4 threads \
35.08s - 8 threads 

