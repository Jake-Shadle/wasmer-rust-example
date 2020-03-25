extern crate wasmer_runtime;

use std::{
    fmt,
    sync::{Arc, Mutex},
};
use wasmer_runtime::{error, func, imports, instantiate, Array, Ctx, WasmPtr};

// Make sure that the compiled wasm-sample-app is accessible at this path.
static WASM: &'static [u8] =
    include_bytes!("../wasm-sample-app/target/wasm32-unknown-unknown/release/wasm_sample_app.wasm");

#[derive(Debug)]
pub struct PanicLocation {
    pub file: std::path::PathBuf,
    pub line: u32,
    pub column: u32,
}

/// Represents a panic that was caught inside the wasm module
#[derive(Debug)]
pub struct PanicInfo {
    /// The error message for the panic
    pub message: String,
    /// The location of the panic, if available
    pub location: Option<PanicLocation>,
}

impl fmt::Display for PanicInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)?;

        if let Some(loc) = &self.location {
            write!(f, ", {}:{}:{}", loc.file.display(), loc.line, loc.column)?;
        }

        Ok(())
    }
}

fn main() -> error::Result<()> {
    // create shared data that we'll use in 2 host functions
    let shared_data = Arc::new(Mutex::new(0usize));

    // copy the [`Arc`] and move it into the closure
    let data = Arc::clone(&shared_data);
    let print_str2 = move |ctx: &mut Ctx, ptr: WasmPtr<u8, Array>, len: u32| {
        let memory = ctx.memory(0);

        // Use helper method on `WasmPtr` to read a utf8 string
        let string = ptr.get_utf8_string(memory, len).unwrap();

        // Get the value from the shared data
        let guard = data.lock().unwrap();
        // Print it!
        println!("{}: {}", guard, string);
    };

    // Copy the [`Arc`] and move it into the closure
    let data = Arc::clone(&shared_data);
    let increment_shared = move || {
        // get the shared data and increment it
        let mut guard = data.lock().unwrap();
        *guard += 1;
    };

    let panic_info = Arc::new(Mutex::new(None));
    let pi = panic_info.clone();
    let register_panic = move |ctx: &mut wasmer_runtime::Ctx,
                               err_msg_ptr: WasmPtr<u8, Array>,
                               err_msg_len: u32,
                               loc_file_ptr: WasmPtr<u8, Array>,
                               loc_file_len: u32,
                               loc_line: u32,
                               loc_column: u32| {
        let memory = ctx.memory(0);
        let err_msg = err_msg_ptr.get_utf8_string(memory, err_msg_len).unwrap();

        let file = loc_file_ptr.get_utf8_string(memory, loc_file_len).unwrap();

        let panic_info = PanicInfo {
            message: err_msg.to_owned(),
            location: Some(PanicLocation {
                file: file.into(),
                line: loc_line,
                column: loc_column,
            }),
        };

        (*pi.lock().unwrap()) = Some(panic_info);
    };

    // Let's define the import object used to import our function
    // into our webassembly sample application.
    //
    // We've defined a macro that makes it super easy.
    //
    // The signature tells the runtime what the signature (the parameter
    // and return types) of the function we're defining here is.
    // The allowed types are `i32`, `u32`, `i64`, `u64`,
    // `f32`, and `f64`.
    //
    // Make sure to check this carefully!
    let import_object = imports! {
        // Define the "env" namespace that was implicitly used
        // by our sample application.
        "env" => {
            // name        // the func! macro autodetects the signature
            "print_str" => func!(print_str),
            // we can use closures here too
            "print_str2" => func!(print_str2),
            "increment_shared" => func!(increment_shared),
            "register_panic" => func!(register_panic),
        },
    };

    // Compile our webassembly into an `Instance`.
    let instance = instantiate(WASM, &import_object)?;

    // Call our exported function!
    instance.call("hello_wasm", &[])?;

    for i in 0..4 {
        // Reset panic information before every call to ensure a previous
        // panic doesn't spill
        (*panic_info.lock().unwrap()) = None;
        match instance.call("fails", &[]) {
            Ok(_) => panic!("calling 'fails' should have returned an error"),
            Err(_) => match *panic_info.lock().unwrap() {
                Some(ref pi) => {
                    println!("call #{} to 'fails' correctly captured panic '{}'", i, pi);
                }
                None => {
                    println!("call #{} to 'fails' failed to capture panic information", i);
                }
            },
        }
    }

    Ok(())
}

// Let's define our "print_str" function.
//
// The declaration must start with "extern" or "extern "C"".
fn print_str(ctx: &mut Ctx, ptr: WasmPtr<u8, Array>, len: u32) {
    // Get a slice that maps to the memory currently used by the webassembly
    // instance.
    //
    // Webassembly only supports a single memory for now,
    // but in the near future, it'll support multiple.
    //
    // Therefore, we don't assume you always just want to access first
    // memory and force you to specify the first memory.
    let memory = ctx.memory(0);

    // Use helper method on `WasmPtr` to read a utf8 string
    let string = ptr.get_utf8_string(memory, len).unwrap();

    // Print it!
    println!("{}", string);
}
