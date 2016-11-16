use std::io;
use std::io::{BufRead, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use wydyd::command::{WCommand, WLocation};

// TODO Clean this crappy code with Error code, etc..

/// Make a connection with the server.
pub fn connect_to_server<A: ToSocketAddrs>(addr: A) -> Result<TcpStream, String> {
    let mut stream = match TcpStream::connect(addr) {
        Ok(s) => s,
        Err(_) => return Err("Server isn't available, run wydyd".to_string()),
    };

    if !confirmation_process(&mut stream) {
        return Err("Error in confirmation process".to_string());
    }
    Ok(stream)
}

/// Send and receive confirmation process after connection.
pub fn confirmation_process(stream: &mut TcpStream) -> bool {
    stream.write(b"WYDY").unwrap();
    let mut confirmation = [0; 4];
    stream.read(&mut confirmation).unwrap();
    let confirmation = confirmation.to_vec();
    let confirmation = String::from_utf8(confirmation).unwrap();
    confirmation == "WYDY"
}

/// Do a presence check to continue communication with the server
pub fn presence_check(stream: &mut TcpStream) {
    match stream.write(&[1]) {
        Ok(_) => {}
        Err(e) => {
            println!("!!! Can't send presence: {}", e);
            ::std::process::exit(1);
        }
    };
    let mut buf = [0];
    stream.read(&mut buf).unwrap();
    if buf[0] != 1 {
        println!("Invalid presence response {}", buf[0]);
        ::std::process::exit(1);
    }
}

/// Send a command to the server
pub fn send_command(stream: &mut TcpStream, command: &str, locally: bool) {
    presence_check(stream);
    let command = format!("{}\n", command);
    match stream.write(command.as_bytes()) {
        Ok(_) => {}
        Err(e) => {
            println!("!!! Can't send command: {}", e);
            ::std::process::exit(1);
        }
    };
    send_location_flag(stream, locally);
}

/// Send location flag
pub fn send_location_flag(stream: &mut TcpStream, locally: bool) {
    if locally {
        stream.write(&[2]).unwrap();
    } else {
        stream.write(&[1]).unwrap();
    }
}


/// Receive command response.
/// Read response and make a choice based on it.
pub fn command_response(stream: &mut TcpStream) {
    let mut response = [0];
    stream.read(&mut response).unwrap();
    match response[0] {
        1 => {
            // Server is executing the command
            receive_command_process(stream);
        }
        2 => {
            // There is multiple command, server needs to receive the choice
            handle_multiple_commands(stream);
        }
        3 => {
            // Used to do output
        }
        _ => {
            // Invalid command
            // Is this reachable?
            println!("Please, run a valid command");
            println!("Type 'list commands' to get the list of all commands");
            return;
        }
    }
}

/// Handle multiple commands
fn handle_multiple_commands(stream: &mut TcpStream) {
    let commands = receive_commands(stream);
    for (i, command) in commands.iter().enumerate() {
        println!("[{}] {}", i + 1, command);
    }
    println!("[_] Exit");

    // Read response
    let stdin = io::stdin();
    let mut lock = stdin.lock();
    let mut input = String::new();
    lock.read_line(&mut input).unwrap();
    let input = input.trim();
    let choice = match input.parse::<u8>() {
        Ok(c) => c,
        Err(e) => {
            println!("Invalid input, exiting: {}", e);
            u8::max_value()
        }
    };
    stream.write(&[choice]).unwrap();
    let choice = choice as usize;
    if choice >= 1 && choice <= commands.len() {
        receive_command_process(stream);
    } else {
        println!("Exiting...");
    }
}

/// Receive all the commands from the server
fn receive_commands(stream: &mut TcpStream) -> Vec<String> {
    // Receive number of commands
    let mut num_commands = [0];
    stream.read(&mut num_commands).unwrap();

    // Receive all the commands
    let mut commands = Vec::new();
    let mut reader = io::BufReader::new(stream);
    for _ in 0..num_commands[0] {
        let mut read = String::new();
        reader.read_line(&mut read).unwrap();
        let read = read.trim().to_string();
        commands.push(read);
    }
    commands
}

/// Receive command process, description + status code
fn receive_command_process(stream: &mut TcpStream) {
    let location = receive_command_location(stream);
    let code = match location {
        1 => {
            let mut reader = io::BufReader::new(stream);
            let mut command = String::new();
            reader.read_line(&mut command).unwrap();
            let mut desc = String::new();
            reader.read_line(&mut desc).unwrap();
            println!("{}", desc);
            let command = WCommand::new(command, desc, WLocation::Client);
            command.run()
        }
        2 => {
            let cmd_desc = receive_running_command_desc(stream);
            print!("{}", cmd_desc);
            presence_check(stream);
            receive_status(stream)
        }
        _ => -1,
    };
    println!("Command executed with code {}", code);
}

fn receive_command_location(stream: &mut TcpStream) -> u8 {
    presence_check(stream);
    let mut buf = [0];
    stream.read(&mut buf).unwrap();
    println!("Command location {}", buf[0]);
    buf[0]
}

/// Receive description of the running command
fn receive_running_command_desc(stream: &mut TcpStream) -> String {
    let mut response = String::new();
    let mut reader = io::BufReader::new(stream);
    reader.read_line(&mut response).unwrap();
    response
}

/// Receive the status code of the running command
fn receive_status(stream: &mut TcpStream) -> i32 {
    let mut reader = io::BufReader::new(stream);
    let mut status = String::new();
    reader.read_line(&mut status).unwrap();
    let status = status.trim();
    status.parse().unwrap()
}
