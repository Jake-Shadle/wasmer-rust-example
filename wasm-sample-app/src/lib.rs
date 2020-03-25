use std::slice;
use std::str;

// Define a function that is imported into the module.
// By default, the "env" namespace is used.
extern "C" {
    fn print_str(ptr: *const u8, len: usize);
    fn print_str2(ptr: *const u8, len: usize);
    fn increment_shared();
    fn register_panic(
        msg_ptr: *const u8,
        msg_len: u32,
        file_ptr: *const u8,
        file_len: u32,
        line: u32,
        column: u32,
    );
}

// Define a string that is accessible within the wasm
// linear memory.
static HELLO: &'static str = "Hello, World!";

// Export a function named "hello_wasm". This can be called
// from the embedder!
#[no_mangle]
pub extern "C" fn hello_wasm() {
    // Call the function we just imported and pass in
    // the offset of our string and its length as parameters.
    unsafe {
        print_str(HELLO.as_ptr(), HELLO.len());
        print_str2(HELLO.as_ptr(), HELLO.len());
        increment_shared();
        increment_shared();
        print_str2(HELLO.as_ptr(), HELLO.len());
    }
}

#[no_mangle]
pub extern "C" fn hello_string_from_rust(ptr: i32, len: i32) {
    let slice = unsafe { slice::from_raw_parts(ptr as _, len as _) };
    let string_from_host = str::from_utf8(&slice).unwrap();
    let out_str = format!("Hello {}", string_from_host);
    unsafe {
        print_str(out_str.as_ptr(), out_str.len());
    }
}

#[no_mangle]
pub extern "C" fn fails() {
    register_panic_hook();

    panic!("oh no");
}

fn hook(info: &std::panic::PanicInfo<'_>) {
    let error_msg = info
        .payload()
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| info.payload().downcast_ref::<&'static str>().copied())
        .unwrap_or("");
    let location = info.location();

    unsafe {
        let _ = match location {
            Some(loc) => {
                let file = loc.file();
                let line = loc.line();
                let column = loc.column();

                register_panic(
                    error_msg.as_ptr(),
                    error_msg.len() as u32,
                    file.as_ptr(),
                    file.len() as u32,
                    line,
                    column,
                )
            }
            None => register_panic(
                error_msg.as_ptr(),
                error_msg.len() as u32,
                std::ptr::null(),
                0,
                0,
                0,
            ),
        };
    }
}

fn register_panic_hook() {
    use std::sync::Once;
    static SET_HOOK: Once = Once::new();
    SET_HOOK.call_once(|| {
        std::panic::set_hook(Box::new(hook));
    });
}
