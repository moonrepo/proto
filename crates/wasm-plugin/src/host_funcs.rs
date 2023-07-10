use extism::{CurrentPlugin, Error, Function, UserData, Val, ValType};

pub fn create_functions() -> Vec<Function> {
    vec![
        Function::new("log", [], [], None, log),
        Function::new(
            "exec_command",
            [ValType::I64],
            [ValType::I64],
            None,
            exec_command,
        ),
    ]
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

// Commands

fn exec_command(
    _plugin: &mut CurrentPlugin,
    _inputs: &[Val],
    _outputs: &mut [Val],
    _user_data: UserData,
) -> Result<(), Error> {
    Ok(())
}
