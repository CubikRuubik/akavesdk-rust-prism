mod utils;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet(name: &str) {
    let mut final_str: String = "Hello, ".to_owned();
    final_str.push_str(name);
    alert(&final_str);
}
