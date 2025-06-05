use std::sync::Once;

/// Set up panic hook for better error logging in WASM context
#[allow(unused)]
pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Initialize the panic handler exactly once
///
/// This function uses `std::sync::Once` to ensure the panic hook
/// is set only once during the application lifecycle
pub fn initialize_panic_handler() {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        // Set the panic hook with better error messages
        #[cfg(feature = "console_error_panic_hook")]
        {
            console_error_panic_hook::set_once();

            // Log that panic handler has been initialized
            web_sys::console::log_1(
                &"Panic handler initialized with console_error_panic_hook".into(),
            );
        }

        #[cfg(not(feature = "console_error_panic_hook"))]
        {
            // Even without the feature, we can provide some basic panic handling
            std::panic::set_hook(Box::new(|panic_info| {
                if let Some(location) = panic_info.location() {
                    web_sys::console::error_2(
                        &format!("Panic occurred at {}:{}", location.file(), location.line())
                            .into(),
                        &format!("{}", panic_info).into(),
                    );
                } else {
                    web_sys::console::error_1(&format!("Panic occurred: {}", panic_info).into());
                }
            }));

            // Log that basic panic handler has been initialized
            web_sys::console::log_1(&"Basic panic handler initialized".into());
        }
    });
}
