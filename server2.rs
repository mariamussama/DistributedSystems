use std::net::{UdpSocket, SocketAddr};
use std::io::{Read, Write};
use std::thread;
use std::time::{Duration, Instant};
use image::{ImageBuffer, Rgba, Pixel, DynamicImage, RgbaImage, GenericImageView};
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};
use serde_derive::{Deserialize, Serialize};
use sysinfo::{System, SystemExt, CpuExt};
use ipc_channel::ipc;
use std::io::prelude::*;

lazy_static! {
    static ref FLAG: Mutex<bool> = Mutex::new(true);
    static ref CPU_LOAD: Mutex<f32> = Mutex::new(0.0);
    static ref CPU_LOADS: Mutex<Vec<f32>> = Mutex::new(vec![0.0; 3]);
    static ref SERVER_ADDRESSES: Mutex<Vec<&'static str>> = Mutex::new(vec![
        "127.0.0.0:8080",
        "127.0.0.0:8081",
        "127.0.0.0:8082",
    ]);
    static ref FLAGS: Mutex<Vec<bool>> = Mutex::new(vec![
        true,
        true,
        true,
    ]);
    static ref SERVER_SEND: Mutex<Vec<(&'static str,&'static str, bool)>> = Mutex::new(vec![
        ("127.0.0.0:8090","127.0.0.0:8080",true),
        ("127.0.0.0:8092","127.0.0.0:8082",true),
    ]);
    static ref CLIENT_ADDRESSES: Vec<(SocketAddr, SocketAddr)> = vec![
        ("127.0.0.0:8000".parse().unwrap(), "127.0.0.0:9000".parse().unwrap()),
        ("127.0.0.0:8001".parse().unwrap(), "127.0.0.0:9001".parse().unwrap()),
        ("127.0.0.0:8002".parse().unwrap(), "127.0.0.0:9002".parse().unwrap()),
    ];
}

#[derive(Debug, Serialize, Deserialize)]
struct ServerMessage {
    load_update: f32,
    ip: String,
    id: usize,
    content: String,
}

const SERVER_ADDRESS: &str = "127.0.0.0";
const SERVER_PORT: &str = "8081";
const SERVERRCV_SOCKET: &str = "127.0.0.0:8081";
const SERVERSND_SOCKET: &str = "127.0.0.0:8091";
const CLIENTRCV_SOCKET: &str = "127.0.0.0:8071";
const CLIENTSND_SOCKET: &str = "127.0.0.0:8061";
const PORTS: [&str; 2] = [CLIENTRCV_SOCKET, SERVERRCV_SOCKET]; // ports to listen on
const ID: usize = 1;

fn main() {
    let mut handles = vec![];
    update_my_CPU();
    update_CPU(ID, CPU_LOAD.lock().unwrap().clone());
    send_load();
    println!("{}", CPU_LOAD.lock().unwrap());
// notify_server_wakeup();
    for &port in PORTS.iter() {
        let port = port.to_string();

        let udp_socket = UdpSocket::bind(&port).expect("Failed to bind to address");

        println!("Server listening on port {}...", port);
        let handle = thread::spawn(move || {
            handle_connections(udp_socket);
        });

        handles.push(handle);
    }

    // Wait for all threads to finish
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
// receive_image_fragments_over_udp();
// encryption_and_send();

}

fn handle_connections(udp_socket: UdpSocket) {
    let mut buffer = [0; 65000];
    //let mut buffer_client = [0u8; 1024];
    let mut received_fragments = vec![vec![]; 40];
    let mut  count=0;
    let mut req_count=0;
    let mut busy = false;
    // notify_server_shutdown();
    loop {
        match udp_socket.recv_from(&mut buffer) {
            Ok((size, client_address)) => {
                let received_data = String::from_utf8_lossy(&buffer[..size]);
                req_count = req_count+ 1;
                // println!("Received message from {}: {}", client_address, received_data);
                // println!("request new");
                // if (req_count == 5)
                // {
                //     notify_server_wakeup();
                //     req_count =0;
                // }
                // else if req_count == 2
                // {
                //     notify_server_shutdown();
                // }
                if FLAGS.lock().unwrap()[1] == true
                {
                    if client_address.port() == 8090 || client_address.port() == 8091 ||client_address.port() == 8092 {
                        // Handle server connections
                        handle_server_connections(&udp_socket, client_address, &buffer[..size]);
                    } else if client_address.port() == 8000{
                        // Handle client connections
                        //println!("busy: {}", busy);
                        if busy == true {
                            received_fragments[count].extend_from_slice(&buffer[..size]);
                            println!("Received fragment {} of size {}", count, size);
                            count = count+1;
                            if count==40
                            {
                                receive_image_fragments_over_udp(&mut received_fragments);
                                let client_send = find_send_addr_by_recv_addr(&client_address).unwrap();
                                encryption_and_send(client_send);
                                busy = false;
                                count = 0;
                            
                            }
                        }
                        else {
                            busy = handle_client_encrypt(&udp_socket, client_address, &buffer[..size], size);
                        } 
                    }
                    else {
                        if busy == false{
                            handle_client_connections(&udp_socket, client_address, &buffer[..size]);
                        }
                    }
                }

            }
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
            }
        }
    }
}

fn handle_server_connections(udp_socket: &UdpSocket, sender_address: SocketAddr, data: &[u8]) {
    // Handle server connections here
    print!("in server communication");
    // let recieved_data = String::from_utf8_lossy(data);
    // println!("request from server: {}", recieved_data);

    // Deserialize the message
    let message: ServerMessage = match bincode::deserialize(data) {
        Ok(msg) => msg,
        Err(_) => todo!(),
    };
    if message.content == "shutdown" {
        println!("Received shutdown message from {}", sender_address);
        update_server_flag(message.id, false);
        let server_send = SERVER_SEND.lock().unwrap();
        for (_,address, status) in server_send.iter() {
            println!("Server: {} - Status: {}", address, status);
        }
        // Update flags or handle shutdown logic here
    }
    else if message.content == "wakeup" {
        println!("Received wakeup message from {}", sender_address);
        update_server_flag(message.id, true);
        // Update flags or handle shutdown logic here
    }
    else if message.content == "update_load" {
        println!("Received update load  message from {}", sender_address);
        update_CPU(message.id, message.load_update);
        // Update flags or handle shutdown logic here
    }
    else 
    {
        println!("request from server: {}",message.content);
    }

    // Respond to the server if needed
    // let response = "Server response";
    // udp_socket.send_to(response.as_bytes(), client_address).expect("Failed to send response to server");
}

fn handle_client_connections(udp_socket: &UdpSocket, client_address: SocketAddr, data: &[u8]) {
    // Handle client connections here
    // You can use the 'data' parameter to process the received data.

    print!("in client communication");
    // connect_to_servers();
    let recieved_data = String::from_utf8_lossy(data);
    println!("{}", recieved_data);
    update_my_CPU();
    update_CPU(ID, CPU_LOAD.lock().unwrap().clone());
    send_load();
    // let id = find_least_load_index();
    let mut id;
    loop {
        id = find_least_load_index();
        if id != None
        {
            println!("least = {}",id.unwrap());
            break;
        }
    }
    if id == Some(ID)
    {
        let send_addr = find_send_addr_by_recv_addr(&client_address).unwrap();
        send_to_client(send_addr);
    }
   
}

fn handle_client_encrypt(udp_socket: &UdpSocket, client_address: SocketAddr, buffer: &[u8], size:usize) -> bool{
    // Handle client connections here
    // You can use the 'data' parameter to process the received data.

    print!("in client communication");
    //connect_to_servers();
    let recieved_data = String::from_utf8_lossy(buffer);
    println!("{}", recieved_data);
    // if *busy
    // {
    //     receive_image_fragments_over_udp();
    //     encryption_and_send();
    // }
    let mut busy = false;
    update_my_CPU();
    update_CPU(ID, CPU_LOAD.lock().unwrap().clone());
    send_load();
    thread::sleep(Duration::from_secs(2));
    let mut id = find_least_load_index();
    loop {
        id = find_least_load_index();
        if id != None
        {
            println!("least = {}",id.unwrap());
            break;
        }
    }
    if id == Some(ID)
    {
        let send_addr = find_send_addr_by_recv_addr(&client_address).unwrap();
        busy = true;
        send_to_client(send_addr);
    }
    return busy;
    
}

fn connect_to_servers(message: &ServerMessage) {
    let server_addresses = SERVER_ADDRESSES.lock().unwrap();
    let id = ID;
    for i in 0..3 {
        if i != id && get_flag(i)
        {
            let server_address = server_addresses[i];
            // println!("{}", server_address);
            match UdpSocket::bind(SERVERSND_SOCKET) {
                Ok(udp_socket) => {
                    udp_socket
                        .connect(server_address)
                        .expect("Failed to connect to server");

                    println!("Connected to server: {}", server_address);
                    let response_bytes = bincode::serialize(&message).unwrap();
                    udp_socket.send(&response_bytes).expect("Server 1: Write error");
                }
                Err(e) => {
                    eprintln!("Failed to connect to server {}: {}", server_address, e);
                }
            }
        }
    }
}
fn notify_servers(message: &ServerMessage) {
    let server_addresses = SERVER_ADDRESSES.lock().unwrap();
    // print!("here");
    let id = ID;
    for i in 0..3 {
        if i != id
        {
            let server_address = server_addresses[i];
            // println!("{}", server_address);
            match UdpSocket::bind(SERVERSND_SOCKET) {
                Ok(udp_socket) => {
                    udp_socket
                        .connect(server_address)
                        .expect("Failed to connect to server");

                    println!("Connected to server: {}", server_address);
                    let response_bytes = bincode::serialize(&message).unwrap();
                    udp_socket.send(&response_bytes).expect("Server 1: Write error");
                }
                Err(e) => {
                    eprintln!("Failed to connect to server {}: {}", server_address, e);
                }
            }
        }
    }
}

fn send_to_client(client_address: SocketAddr){
    match UdpSocket::bind(CLIENTSND_SOCKET) {
        Ok(udp_socket) => {
            udp_socket.connect(client_address).expect("Failed to connect to server");

            println!("Connected to client: {}", client_address);
            let response = CLIENTRCV_SOCKET.to_string();
            let response_bytes = bincode::serialize(&response).unwrap();
            udp_socket.send(&response_bytes).expect("Server: Write error");
        }
        Err(e) => {
            eprintln!("Failed to connect to server {}: {}", client_address, e);
        }
    }
}

fn find_send_addr_by_recv_addr(recv_addr: &SocketAddr) -> Option<SocketAddr> {
    CLIENT_ADDRESSES.iter()
        .find(|&&(ref sa, _)| sa == recv_addr)
        .map(|&(_, send_addr)| send_addr)
}

fn notify_server_shutdown() {
    // First, update the local server flag to indicate it's going down
    let mut flags = FLAGS.lock().unwrap();
    if let Some(flag) = flags.get_mut(ID) {
        *flag = false;
    }
    // Construct a shutdown message
    let shutdown_message = ServerMessage {
        // Fill in the appropriate fields for your ServerMessage struct
        load_update: 0.0, // example field
        ip: SERVERSND_SOCKET.to_string(),
        id: ID, // ID of the server that's shutting down
        content: "shutdown".to_string(), // Indicate this is a shutdown message
    };
    // notify others.
    notify_servers(&shutdown_message);
    println!("SHUTDOWN");

}

fn notify_server_wakeup() {
    // First, update the local server flag to indicate it's going down
    let mut flags = FLAGS.lock().unwrap();
    if let Some(flag) = flags.get_mut(ID) {
        *flag = true;
    }
    // Construct a wakeup message
    let wakeup_message = ServerMessage {
        // Fill in the appropriate fields for your ServerMessage struct
        load_update: 0.0, // example field
        ip: SERVERSND_SOCKET.to_string(),
        id: ID, // ID of the server that's shutting down
        content: "wakeup".to_string(), // Indicate this is a shutdown message
    };
    // notify others.
    //println!("call function");
    notify_servers(&wakeup_message);
    println!("WAKEUP");

}

fn update_server_flag(index: usize, value: bool) {
    let mut flags = FLAGS.lock().unwrap();
    if let Some(flag) = flags.get_mut(index) {
        *flag = value;
    }

}
fn get_flag(index: usize) -> bool{
    let mut flags = FLAGS.lock().unwrap();
    return flags[index];
}

fn update_my_CPU() {
    let mut system = System::new_all();
    system.refresh_all();

    let total_cpu_usage: f32 = system.cpus().iter().map(|cpu| cpu.cpu_usage()).sum();
    let average_cpu_usage = total_cpu_usage / system.cpus().len() as f32;

    *CPU_LOAD.lock().unwrap() = average_cpu_usage;
    println!("Updated CPU Load: {:.2}%", average_cpu_usage);
}

fn update_CPU(id:usize, load:f32){
    let mut cpu_loads = CPU_LOADS.lock().unwrap();
    
    // Check if the provided index is valid
    if id < cpu_loads.len() {
        // Update the specified entry
        cpu_loads[id] = load;

        println!("Updated CPU Loads: {:?}", *cpu_loads);
    } else {
        println!("Invalid entry index");
    }

}

fn send_load() {
    // Construct a load message
    let load_message = ServerMessage {
        // Fill in the appropriate fields for your ServerMessage struct
        load_update: CPU_LOAD.lock().unwrap().clone(), // example field
        ip: SERVERSND_SOCKET.to_string(),
        id: ID, // ID of the server 
        content: "update_load".to_string(), // Indicate this is a shutdown message
    };
    // notify others.
    connect_to_servers(&load_message);

}

fn find_least_load_index() -> Option<usize> {
    let cpu_loads = CPU_LOADS.lock().unwrap();
    let flags = FLAGS.lock().unwrap();

    // Find indices where load != 0.0 and flag is true
    let valid_indices: Vec<usize> = cpu_loads
        .iter()
        .zip(flags.iter())
        .enumerate()
        .filter(|(_, (&load, &flag))| load != 0.0 && flag)
        .map(|(index, _)| index)
        .collect();

    if let Some(min_index) = valid_indices
        .iter()
        .min_by(|&idx1, &idx2| cpu_loads[*idx1].partial_cmp(&cpu_loads[*idx2]).unwrap())
    {
        Some(*min_index)
    } else {
        None
    }
}

fn receive_image_fragments_over_udp(received_fragments: &mut Vec<Vec<u8>>,) {
    //let socket = UdpSocket::bind("127.0.0.0:8070").expect("Failed to bind socket");

    // Deserialize fragments into images
    let mut images = Vec::new();
    for fragment_data in received_fragments {
        let image = image::load_from_memory(&fragment_data)
            .expect("Failed to load image from received data");
        images.push(image);
    }

    let width = 1200;
    let height = 1200;


    let reassembled_image = reassemble_images(&images, width, height);

    // Save the reassembled image
    reassembled_image.save("received_image.png").expect("Failed to save reassembled image");

}

fn reassemble_images(images: &[DynamicImage], width: u32, height: u32) -> RgbaImage {
    let mut reassembled = RgbaImage::new(width, height);

    for (i, image) in images.iter().enumerate() {
        for y in 0..image.height() {
            for x in 0..image.width() {
                let pixel = image.get_pixel(x, y);
                reassembled.put_pixel(x, y + i as u32 * image.height(), pixel);
            }
        }
    }

    reassembled
}

fn encryption_and_send(client_address: SocketAddr){
    // Load the cover image (the image that will hide the secret image)
    let cover_image_path = "carrier.png"; // Replace with the actual file path
    let cover_image = image::open(cover_image_path).expect("Failed to open cover image");

    // Load the hidden image (the image to be hidden inside the cover image)
    let hidden_image_path = "received_image.png"; // Replace with the actual file path
    let hidden_image = image::open(hidden_image_path).expect("Failed to open hidden image");


    // Clone the cover image to use later
    let cover_image_clone = cover_image.clone();
    
    // Embed the hidden image within the cover image
    let embedded_image = encrypt_image(cover_image, hidden_image);

    // Save the embedded image to a file
    let embedded_image_path = "encrypted_image.png";
    embedded_image.save(embedded_image_path).expect("Failed to save embedded image");
    println!("Embedded image saved to {}", embedded_image_path);
    image_send(client_address, embedded_image_path.to_owned());

    // // Load the embedded image for extraction
    // let embedded_image = image::open(embedded_image_path).expect("Failed to open embedded image");

    // Extract the hidden image from the embedded image
    // let extracted_image = image_decrypt(embedded_image, cover_image_clone);

    // // Save the extracted image to a file
    // let extracted_image_path = "/home/rawanalaax/Downloads/image_decrypt/Images/extracted_image.png";
    // extracted_image.save(extracted_image_path).expect("Failed to save extracted image");
    // println!("Extracted image saved to {}", extracted_image_path);
}
fn encrypt_image(cover_image: DynamicImage, hidden_image: DynamicImage) -> DynamicImage {

    let (width, height) = cover_image.dimensions();
    let hidden_image = hidden_image.resize_exact(width, height, image::imageops::FilterType::Lanczos3);
   
    let mut cover_buffer = cover_image.to_rgba8();
    let hidden_buffer = hidden_image.to_rgba8();
   
    for (x, y, cover_pixel) in cover_buffer.enumerate_pixels_mut() {
    let hidden_pixel = hidden_buffer.get_pixel(x, y);
   
    let (r, _g, _b, _a) = (cover_pixel[0], cover_pixel[1], cover_pixel[2], cover_pixel[3]);
    let (hr, hg, hb, _ha) = (hidden_pixel[0], hidden_pixel[1], hidden_pixel[2], hidden_pixel[3]);
   
    cover_pixel[0] = (r & 0xF0) | (hr >> 4);
    cover_pixel[1] = (_g & 0xF0) | (hg >> 4);
    cover_pixel[2] = (_b & 0xF0) | (hb >> 4);
    }
   
    DynamicImage::ImageRgba8(cover_buffer)
   }

fn image_send(send_addr: SocketAddr, path:String ){
    let img = image::open(path).expect("Failed to open image");
    let rgba_img = img.to_rgba8();

    let (fragments, width, height) = fragment_image(&rgba_img);
    let mut fragment_paths = Vec::new();

    for (i, fragment) in fragments.iter().enumerate() {
        let path = format!("fragment_{}.png", i);
        fragment.save(&path).expect("Failed to save fragment");
        fragment_paths.push(path);
    }
    send_image_fragments_over_udp(fragment_paths,send_addr);

    //reassemble 
    // let reassembled = reassemble_image(&fragments, width, height);
    // reassembled.save("reassembled.png").expect("Failed to save reassembled image");
}

fn fragment_image(img: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> (Vec<ImageBuffer<Rgba<u8>, Vec<u8>>>, u32, u32) {
    let (width, height) = img.dimensions();
    println!("{}, {}", width, height);
    let fragment_height = height / 40;

    let mut fragments = Vec::new();
    for i in 0..40 {
        let fragment = img.view(0, i * fragment_height, width, fragment_height).to_image();
        fragments.push(fragment);
    }
    // let reassembled = reassemble_image(&fragments, width, height);
    // reassembled.save("reassembled.png").expect("Failed to save reassembled image");
    (fragments, width, height)
}

fn send_image_fragments_over_udp(fragment_paths: Vec<String>, send_addr: SocketAddr) {
//    let socket = UdpSocket::bind("127.0.0.0:8000").expect("Failed to bind socket");
    for (i, path) in fragment_paths.iter().enumerate() {

        if let Ok(img) = image::open(path) {

            let image_data = std::fs::read(path).expect("Failed to read image data");
            println!("Size of fragment: {} bytes", image_data.len());
            let socket = UdpSocket::bind(CLIENTSND_SOCKET).expect("Failed to bind socket");

            // Send the image data over UDP
            if let Err(err) = socket.send_to(&image_data, send_addr) {
                eprintln!("Failed to send data: {}", err);
            } else {
                println!("Image data sent successfully!");
            }

        } else {
            eprintln!("Failed to load the image");
        }
    }
}

