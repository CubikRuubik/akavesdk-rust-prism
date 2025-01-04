use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use js_sys::Array;

#[wasm_bindgen]
extern "C" {
    // External JS function to request accounts from MetaMask
    #[wasm_bindgen(js_name = "ethereum.request")]
    fn request(args: JsValue) -> JsValue;

    // TODO: extend metamask interface
}

#[wasm_bindgen]
pub async fn sign_message_with_metamask(message: String) -> Result<String, JsValue> {
    // Get the window object
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window object available"))?;
    
    // Check if MetaMask is available and connected
    if let Some(ethereum) = window.get("ethereum") {
        let ethereum = ethereum.dyn_into::<js_sys::Object>().unwrap();

        // Log that MetaMask exists and is available
        web_sys::console::log_1(&JsValue::from_str("MetaMask is available"));
        // Log the entire ethereum object to the console
        web_sys::console::log_1(&ethereum);

        // TODO: check connected
        // TODO: request signature

        // Sign the message with the obtained address
        let signed_message = sign_message("mock".into(), message);

        Ok(signed_message.as_string().unwrap_or_default())
    } else {
        Err(JsValue::from_str("MetaMask is not available"))
    }
}

// Mocked sign message function, assumes the real implementation would sign the message.
// TODO: hook in proper signing
fn sign_message(address: String, message: String) -> JsValue {
    // This is a mock; the actual implementation would trigger the signing in MetaMask
    JsValue::from_str(&format!("Signed message by {}: {}", address, message))
}
