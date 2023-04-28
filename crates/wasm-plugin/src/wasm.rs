use extism::{CurrentPlugin, Error, Function, UserData, Val};

pub fn create_functions() -> Vec<Function> {
    vec![Function::new("log", [], [], None, log)]
}

// Logging

pub fn log(
    _plugin: &mut CurrentPlugin,
    _inputs: &[Val],
    _outputs: &mut [Val],
    _user_data: UserData,
) -> Result<(), Error> {
    println!("Hello from Rust!");
    dbg!(_inputs);
    dbg!(_outputs);
    // outputs[0] = inputs[0].clone();
    Ok(())
}
