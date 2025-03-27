// Unused function
// pub fn split_vec<T>(v: Vec<T>, chunk_size: usize) -> Vec<Vec<T>> {
//     use std::collections::VecDeque;
//
//     let mut v: VecDeque<T> = v.into(); // avoids reallocating when possible
//
//     let mut acc = Vec::new();
//     while v.len() > chunk_size {
//         acc.push(v.drain(0..chunk_size).collect());
//         v.shrink_to_fit();
//     }
//     acc.push(v.into());
//     acc
// }
