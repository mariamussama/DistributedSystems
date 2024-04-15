use std::net::TcpStream;
use std::io::{Read, Write};
use rand::Rng;

const CLIENT_ADDRESS: &str = "0.0.0.0:1030";
fn main() {
    let server_addresses = vec![
        "0.0.0.0:8080",
        "0.0.0.0:8081",
        "0.0.0.0:8082",
    ];

    for server_address in &server_addresses {
        //let command_to_execute = "1"; // Replace with the command you want to execute
        let mut rng = rand::thread_rng();
        let random_number = rng.gen_range(1..=100);
        // let number_to_send= random_number.to_string();
        let number_to_send= "1";
        match TcpStream::connect(server_address) {
            Ok(mut stream) => {
                println!("Connected to server: {} with request to increment {}", server_address, number_to_send);

                // Send the command to the server
                stream.write(number_to_send.as_bytes()).expect("Failed to send data to the server");

                // Receive and print the server's response
                let mut response = Vec::new();
                stream.read_to_end(&mut response).expect("Failed to read server response");
                println!("Server response for {}: {}", server_address, String::from_utf8_lossy(&response));
            }
            Err(e) => {
                eprintln!("Failed to connect to server {}: {}", server_address, e);
            }
        }
    }
}
