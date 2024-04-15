use std::fs::{File, self};
use std::io::{ErrorKind, SeekFrom, Seek, Read, self, Write};
use std::net::{UdpSocket, SocketAddr};
// use std::os::unix::net::SocketAddr;
use glob::glob;
use image::codecs::png::PngEncoder;
//use std::ptr::metadata;
//use std::os::unix::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::mpsc;
// use std::time::Duration;
use std::time::{Duration, Instant};
use image::ImageResult;
use lazy_static::lazy_static;
use rand::Rng;
use rand::prelude::*;
use std::io::BufWriter;
use std::io::Cursor;
use serde_derive::{Deserialize, Serialize};
// use image::png::PNGEncoder;
use image::io::Reader as ImageReader;
use minifb::{Window, Scale};
use minifb::WindowOptions;
// use crate::thread::local_impl::Key;
use minifb::Key;
// use std::thread::local_impl::Key;

use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba,RgbaImage,self, ImageFormat, ImageError, imageops::FilterType, imageops::blur};
// use image::codecs::png::PngEncoder;
// use crate::image::codecs::png::PngEncoder;
lazy_static! {
    static ref STATUS: Mutex<bool> = Mutex::new(false);
    static ref PENDING_IP: Mutex<Option<SocketAddr>> = Mutex::new(None);
    static ref IMG_NAME: Mutex<Option<String>> = Mutex::new(None);
    static ref VIEWS: Mutex<Option<usize>> = Mutex::new(None);
    static ref IMGS_SIZE: Mutex<Option<Vec<u32>>> = Mutex::new(None);
    //send, receive
    static ref SND_SERVER_SOCKET: SocketAddr= "127.0.0.0:8000".parse().unwrap();
    static ref RCV_SERVER_SOCKET: SocketAddr= "127.0.0.0:9000".parse().unwrap();
    static ref RCV_SERVER_SOCKET_2: SocketAddr= "127.0.0.0:7000".parse().unwrap();
    static ref SND_CLIENT_SOCKET: SocketAddr= "127.0.0.0:5000".parse().unwrap();
    static ref RCV_CLIENT_SOCKET: SocketAddr= "127.0.0.0:6000".parse().unwrap();
    static ref PORTS: [SocketAddr; 1] = [*RCV_CLIENT_SOCKET];
    static ref SERVER_ADDRESSES: Mutex<Vec<(&'static str, &'static str)>> = Mutex::new(vec![
        ("127.0.0.0:8070", "127.0.0.0:8060"),
        ("127.0.0.0:8071", "127.0.0.0:8061"),
        ("127.0.0.0:8072", "127.0.0.0:8062"),
    ]);
    static ref RCV_IMGS: Mutex<Vec<(String, usize)>> = Mutex::new(Vec::new());
    static ref SND_IMGS: Mutex<Vec<(String, SocketAddr)>> = Mutex::new(Vec::new());
    static ref ACK: Mutex<bool> = Mutex::new(false);
    //send, rcv
    // static ref CLIENT_ADDRESSES: Vec<(SocketAddr, SocketAddr)> = vec![
    //     ("127.0.0.0:8000".parse().unwrap(), "127.0.0.0:9000".parse().unwrap()),
    //     ("127.0.0.0:8001".parse().unwrap(), "127.0.0.0:9001".parse().unwrap()),
    //     ("127.0.0.0:8002".parse().unwrap(), "127.0.0.0:9002".parse().unwrap()),
    // ];

}
#[derive(Debug, Serialize, Deserialize)]
struct ClientMessage {
    ip: String,
    id: usize,
    purpose: String,
    req_ip:String,
    req_id:usize,
    status:bool,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
struct ClientServer_IP{
    ip_rcv_clients: String,
    ip_rcv_servers: String,
    id: usize,
    purpose: String,
    req_ip:Vec<String>,
    req_id:Vec<String>,
    pending_ip:SocketAddr,
    pending_views:usize,
    pending_img_name: String,
}
#[derive(Debug, Serialize, Deserialize)]
struct Client_Client{
    ip_Sender: SocketAddr,
    ip_Reciever: SocketAddr,
    img_id: usize,
    img_name: String,
    purpose: String,
    size: usize,
    imgs_height:Vec<u32>,
    imgs_width:Vec<u32>,
    imgs_size:Vec<u32>,
    imgs_names:Vec<String>,
    views: usize,
}
const MAX_RETRIES: usize = 3;
const ID: usize = 0;

fn main() {
    let (sender, receiver) = mpsc::channel();
    let mut handles = vec![];
    for &port in PORTS.iter() {
        let port = port.to_string();

        let udp_socket = UdpSocket::bind(&port).expect("Failed to bind to address");

        println!("Client listening on port {}...", port);
        let handle = thread::spawn(move || {
            handle_connections(udp_socket);
        });

        handles.push(handle);
    }

        let sender_clone = sender.clone();

       // let sender_clone = sender.clone();
       println!("Enter '1' to wake up:");

       let mut input = String::new();
   
       match io::stdin().read_line(&mut input) {
           Ok(_) => {
               if input.trim() == "1" {
                notify_server_wakeup();
               } else {
                   println!("Invalid input. Please enter '1' to wake up.");
               }
           }
           Err(e) => println!("Error reading input: {}", e),
       }
       
       loop {
        println!("Enter 'A' to ask for an image or 'B' to go down or 'C' to view your images or 'D' for modify views for client:");

        let mut input = String::new();

        match io::stdin().read_line(&mut input) {
            Ok(_) => {
                match input.trim() {
                    "A" => {
                        for request_num in 0..1 {
                            let thread_id = format!("Client {}: Request {}", *SND_SERVER_SOCKET, request_num);
                            let mut retries = 0;
                
                            loop {
                                let recv_socket = UdpSocket::bind(*RCV_SERVER_SOCKET).expect("Failed to bind recv socket");
                                recv_socket.set_read_timeout(Some(Duration::from_secs(3))).expect("Failed to set timeout");
                                let request = format!("Request {} from {}", request_num, thread_id);
                                send_request_for_ip(*SND_SERVER_SOCKET, &request);
                                let (response, server_rcv) = receive_response_IPS(&recv_socket);
                                // println!("{}", response.req_ip);
                                if response.ip_rcv_clients !="0".to_string() {
                                    println!("{} received response: {} Clients available", thread_id, response.req_ip.len());
                                    if response.req_ip.len() !=0{
                                       // let server_address = get_send_server(&server_rcv.unwrap().to_string());
                                        sender_clone.send(()).expect("Failed to send signal");
                                        let mut rng = rand::thread_rng();
                                        let choosen_ip = response.req_ip.choose(&mut rng);
                                        //println!("{}", choosen_ip.unwrap());
                                        send_req_for_img(choosen_ip.unwrap().parse().unwrap());
                
                                    }
                                   // send_to_client(choosen_ip.unwrap().parse(), response);
                                    break;
                                } else {
                                    retries += 1;
                                    if retries >= MAX_RETRIES {
                                        println!("{} maximum retries reached. Exiting.", thread_id);
                                        break;
                                    }
                                    println!("{} timeout occurred. Retrying...", thread_id);
                                }
                            }
                        }
                
                    },
                    "B" => {
                        go_down();
                        println!("Enter '1' to wake up:");

                        let mut input = String::new();
                    
                        match io::stdin().read_line(&mut input) {
                            Ok(_) => {
                                if input.trim() == "1" {
                                 notify_server_wakeup();
                                } else {
                                    println!("Invalid input. Please enter '1' to wake up.");
                                }
                            }
                            Err(e) => println!("Error reading input: {}", e),
                        }

                    }
                    ,
                    "C"=>{
                        let max_choice = get_all_rcv_image_names().len();
                        print_rcv_image_data();
                        loop {
                            println!("Choose an item by entering its number (1... {}) or '0' to exit:", max_choice);
                    
                            let mut input = String::new();
                    
                            match io::stdin().read_line(&mut input) {
                                Ok(_) => {
                                    let choice: usize = match input.trim().parse() {
                                        Ok(num) => num,
                                        Err(_) => {
                                            println!("Invalid input. Please enter a number.");
                                            continue;
                                        }
                                    };
                    
                                    if choice == 0 {
                                        println!("Exiting...");
                                        break;
                                    } else if choice <= max_choice {
                                       // println!("You chose: {}", ge);
                                       let image_name = get_image_name_by_index(choice-1).unwrap();
                                       let views = get_views_for_rcv_image(&image_name.clone()).unwrap();
                                       let path = format!("received_imgs/{}",image_name.clone());
                                       show_image(path.clone());
                                       let v = views-1;
                                       if v == 0{
                                        remove_image_by_name(&image_name.clone());
                                        let _ = fs::remove_file(path);
                                        break;
                                       }else {
                                        change_views_for_rcv_image(&image_name, v);
                                       }
                                       
                                       
                                    } else {
                                        println!("Invalid choice. Please enter a number between 1 and {}.", max_choice);
                                    }
                                }
                                Err(e) => {
                                   // println!("Error reading input: {}", e);
                                    continue;
                                }
                            }
                        }
                    }

                    "D"=>{
                        let max_choice = get_snd_size();
                        print_snd_image();
                        loop {
                            println!("Choose an item by entering its number (1... {}) or '0' to exit:", max_choice);
                    
                            let mut input = String::new();
                    
                            match io::stdin().read_line(&mut input) {
                                Ok(_) => {
                                    let choice: usize = match input.trim().parse() {
                                        Ok(num) => num,
                                        Err(_) => {
                                            println!("Invalid input. Please enter a number.");
                                            continue;
                                        }
                                    };
                    
                                    if choice == 0 {
                                        println!("Exiting...");
                                        break;
                                    } else if choice <= max_choice {
                                       // println!("You chose: {}", ge);
                                       println!("Enter the views:");
                                       let mut input = String::new();
                                       io::stdin().read_line(&mut input).expect("Failed to read line");
                                       match input.trim().parse::<usize>() {
                                           Ok(new_view) => {
                                               let image_name = get_snd_img_by_index(choice-1).unwrap();
                                               let client_add = get_snd_client_by_index(choice-1).unwrap();
                                               println!("{},{}", image_name,client_add);
                                               send_modify_views_for_client(image_name, client_add, new_view);
                                               let mut start_time = Instant::now();
                                               let duration_to_reach = Duration::from_secs(10);
                                              // let mut ack_check_thread: Option<thread::JoinHandle<()>>;

                                               let ack_check_thread = Some(thread::spawn(move || {
                                                while start_time.elapsed() < duration_to_reach {
                                                   println!("waiting");
                                                }
                                                if get_ack()==false{
                                                    println!("ACK not received within 10 seconds.");
                                                    let ip = get_pending_ip();
                                                    let image = get_img_name();
                                                    let views = get_views();
                                                    send_server_to_buffer(ip.unwrap(), image.unwrap(), views.unwrap());
                                                }
                                            }));
                                            if let Err(e) = ack_check_thread.unwrap().join() {
                                                println!("Error occurred in ack_check_thread: {:?}", e);
                                            } else {
                                                println!("ack_check_thread finished successfully");
                                            }   
                                           }
                                           Err(_) => {
                                               println!("Please enter a valid number.");
                                           }
                                       }
                                       
                                       
                                    } else {
                                        println!("Invalid choice. Please enter a number between 1 and {}.", max_choice);
                                    }
                                }
                                Err(e) => {
                                   // println!("Error reading input: {}", e);
                                    continue;
                                }
                            }
                        }
                        // send_modify_views_for_client(image_name, send_addr, views)
                    }
                    _ => println!(""),
                }
            }
            Err(e) => {
                println!("Error reading input: {}", e);
                continue;
            }
        }


    }
        //need function to fetch for pending requests
        // for request_num in 0..1 {
        //     let thread_id = format!("Client {}: Request {}", *SND_SERVER_SOCKET, request_num);
        //     let mut retries = 0;

        //     loop {
        //         let recv_socket = UdpSocket::bind(*RCV_SERVER_SOCKET).expect("Failed to bind recv socket");
        //         recv_socket.set_read_timeout(Some(Duration::from_secs(3))).expect("Failed to set timeout");
        //         let request = format!("Request {} from {}", request_num, thread_id);
        //         send_request_for_ip(*SND_SERVER_SOCKET, &request);
        //         let (response, server_rcv) = receive_response_IPS(&recv_socket);
        //         // println!("{}", response.req_ip);
        //         if response.ip_rcv_clients !="0".to_string() {
        //             println!("{} received response: {} Clients available", thread_id, response.req_ip.len());
        //             if response.req_ip.len() !=0{
        //                // let server_address = get_send_server(&server_rcv.unwrap().to_string());
        //                 sender_clone.send(()).expect("Failed to send signal");
        //                 let mut rng = rand::thread_rng();
        //                 let choosen_ip = response.req_ip.choose(&mut rng);
        //                 //println!("{}", choosen_ip.unwrap());
        //                 send_req_for_img(choosen_ip.unwrap().parse().unwrap());

        //             }
        //            // send_to_client(choosen_ip.unwrap().parse(), response);
        //             break;
        //         } else {
        //             retries += 1;
        //             if retries >= MAX_RETRIES {
        //                 println!("{} maximum retries reached. Exiting.", thread_id);
        //                 break;
        //             }
        //             println!("{} timeout occurred. Retrying...", thread_id);
        //         }
        //     }
        // }

    //Main thread waits for signals and sends the next request
    // for _ in 0..3 {
    //     receiver.recv().expect("Failed to receive signal");
    // }
    for handle in handles {
        handle.join().expect("Thread panicked");
    }
}


fn send_request_for_ip(send_addr: SocketAddr, request: &str) {
    let server_addresses = SERVER_ADDRESSES.lock().unwrap();
    let send_socket = UdpSocket::bind(send_addr).expect("Failed to bind send socket");
    println!("{}", request);
    let request_message = ClientServer_IP {
        ip_rcv_clients: (*RCV_CLIENT_SOCKET).to_string(),
        ip_rcv_servers: (*RCV_SERVER_SOCKET).to_string(),
        id: ID, // ID of the server that's shutting down
        purpose: "request_for_IP".to_string(), // Indicate this is a shutdown message
        req_ip: Vec::new(),
        req_id:Vec::new(),
        pending_ip: "0.0.0.0:0".parse().unwrap(),
        pending_views: 0,
        pending_img_name: "".to_string(),
    };

    let response_bytes = bincode::serialize(&request_message).unwrap();
    // Simulate sending a request
    for &(server_address,_) in server_addresses.iter() {
        if let Err(e) = send_socket.send_to(&response_bytes, server_address) {
            eprintln!("Failed to send request to {}: {}", server_address, e);
        }
    }

    println!("sent");
   // &send_socket
}

fn receive_response_IPS(recv_socket: &UdpSocket) -> (ClientServer_IP, Option<SocketAddr>) {
    let mut retries = 0;
    loop {
        let mut buffer = [0; 1024];
        match recv_socket.recv_from(&mut buffer) {
            Ok((size, mut server_address)) => {                
                // let data = String::from_utf8_lossy(&buffer[..size]).to_string();
                let mut message: ClientServer_IP = match bincode::deserialize(&buffer) {
                    Ok(msg) => msg,
                    Err(_) => todo!(),
                };
                return (message, Some(server_address));}

            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                retries += 1;
                if retries >= MAX_RETRIES {
                    break;
                }
            },
            Err(e) => {
                eprintln!("Failed to receive response: {}", e);
                break;
            },
        }
    }
    let default_message = ClientServer_IP {
        ip_rcv_clients: "0".to_string(),
        ip_rcv_servers: "0".to_string(),
        id: ID, // ID of the server that's shutting down
        purpose: "request_for_IP".to_string(), // Indicate this is a shutdown message
        req_ip:Vec::new(),
        req_id:Vec::new(),
        pending_ip: "0.0.0.0:0".parse().unwrap(),
        pending_views: 0,
        pending_img_name: "".to_string(),
    };
    (default_message, None)
}

fn send_servers(message: ClientMessage) {
    let server_addresses = SERVER_ADDRESSES.lock().unwrap();
    let send_socket: UdpSocket = UdpSocket::bind(*SND_SERVER_SOCKET).expect("Failed to bind send socket");
    let response_bytes = bincode::serialize(&message).unwrap();
    // Simulate sending a request
    for &(server_address,_) in server_addresses.iter() {
        if let Err(e) = send_socket.send_to(&response_bytes, server_address) {
            eprintln!("Failed to send request to {}: {}", server_address, e);
        }
    }

    println!("sent to servers");
   // &send_socket
}

fn receive_response(recv_socket: &UdpSocket) -> (ClientMessage, Option<SocketAddr>) {
    let mut retries = 0;
    loop {
        let mut buffer = [0; 1024];
        match recv_socket.recv_from(&mut buffer) {
            Ok((size, mut server_address)) => {                
                // let data = String::from_utf8_lossy(&buffer[..size]).to_string();
                let mut message: ClientMessage = match bincode::deserialize(&buffer) {
                    Ok(msg) => msg,
                    Err(_) => todo!(),
                };
                return (message, Some(server_address));}

            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                retries += 1;
                if retries >= MAX_RETRIES {
                    break;
                }
            },
            Err(e) => {
                eprintln!("Failed to receive response: {}", e);
                break;
            },
        }
    }
    let default_message = ClientMessage {
        ip: "0".to_string(),
        id: ID, // ID of the server that's shutting down
        purpose: "request_for_IP".to_string(), // Indicate this is a shutdown message
        req_ip:"0".to_string(),
        req_id:0,
        status: false,
    };
    (default_message, None)
}


fn image_send(img_path: String, server_addr:SocketAddr){
    let img = image::open(img_path).expect("Failed to open image");
    let rgba_img = img.to_rgba8();

    let (fragments, width, height) = fragment_image(&rgba_img);
    let mut fragment_paths = Vec::new();

    for (i, fragment) in fragments.iter().enumerate() {
        let path = format!("fragment_{}.png", i);
        fragment.save(&path).expect("Failed to save fragment");
        fragment_paths.push(path);
    }
    send_image_fragments_over_udp(fragment_paths, server_addr);

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

    (fragments, width, height)
}


fn send_image_fragments_over_udp(fragment_paths: Vec<String>, server_addr:SocketAddr) {
   let socket = UdpSocket::bind(*SND_SERVER_SOCKET).expect("Failed to bind socket");
    for (i, path) in fragment_paths.iter().enumerate() {

        if let Ok(img) = image::open(path) {

            let image_data = std::fs::read(path).expect("Failed to read image data");
            println!("Size of fragment: {} bytes", image_data.len());
            // let socket = UdpSocket::bind("127.0.0.0:8000").expect("Failed to bind socket");

            // Send the image data over UDP
            //let addr = "127.0.0.0:8070"; // Replace this with the receiver's address

            // Send the image data over UDP
            if let Err(err) = socket.send_to(&image_data, server_addr) {
                eprintln!("Failed to send data: {}", err);
            } else {
                println!("Image data sent successfully!");
            }

        } else {
            eprintln!("Failed to load the image");
        }
    }
}

fn receive_image_fragments_over_udp(socket: &UdpSocket) -> &str {
   // let socket = UdpSocket::bind(client_rcv).expect("Failed to bind socket");
   
    let mut received_fragments = vec![vec![]; 40]; // Assuming you have 10 fragments

    let mut i=0;
    loop {
        let mut fragment_buffer = vec![0; 65000];
        match socket.recv_from(&mut fragment_buffer) {
            Ok((size, _)) => 
            {   received_fragments[i].extend_from_slice(&fragment_buffer[..size]);
                println!("Received fragment {} of size {}", i, size);
                i = i+1;
                if i==40
                {
                    break;
                }
            }
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
            },
            Err(e) => {
                eprintln!("Failed to receive response: {}", e);
                break;
            },
        }
    }
    // for i in 0..40 {
    //     let mut fragment_buffer = vec![0; 65000]; // Adjust the buffer size according to your needs
    //     let (size, _) = socket.recv_from(&mut fragment_buffer).expect("Failed to receive data");
    //     received_fragments[i].extend_from_slice(&fragment_buffer[..size]);
    //     println!("Received fragment {} of size {}", i, size);
    // }

    // Deserialize fragments into images
    let mut images = Vec::new();
    for fragment_data in received_fragments {
        let image = image::load_from_memory(&fragment_data)
            .expect("Failed to load image from received data");
        images.push(image);
    }

    let width = 500;
    let height = 500;


    let reassembled_image = reassemble_images(&images, width, height);

    // Save the reassembled image
    let received = "received_image.png";
    reassembled_image.save(received).expect("Failed to save reassembled image");

    return received;
    //decryption(received.to_owned());

}

fn receive_image_fragments_from_client(received_fragments: Vec<Vec<u8>>) -> String {
    //let socket = UdpSocket::bind("127.0.0.0:8070").expect("Failed to bind socket");

    // Deserialize fragments into images
    let mut images = Vec::new();
    for fragment_data in received_fragments {
        let image = image::load_from_memory(&fragment_data)
            .expect("Failed to load image from received data");
        images.push(image);
    }

    let width = 500;
    let height = 500;


    let reassembled_image = reassemble_images(&images, width, height);

    // Save the reassembled image
    let path = "received_image.png";
    reassembled_image.save(path).expect("Failed to save reassembled image");
    return path.to_string();
 }
 
fn receive_option_image_fragments_from_client(received_fragments: Vec<Vec<u8>>) {
    //let socket = UdpSocket::bind("127.0.0.0:8070").expect("Failed to bind socket");

    // Deserialize fragments into images
    let mut images = Vec::new();
    for fragment_data in received_fragments {
        let image = image::load_from_memory(&fragment_data)
            .expect("Failed to load image from received data");
        images.push(image);
    }

    let width = 500;
    let height = 500;


    let reassembled_image = reassemble_images(&images, width, height);
    
   // reassembled_image.save("reassembeled.png").expect("Failed to save extracted image");
    // let img = image::open(image_path.clone()).expect("Failed to open image");

    // // Convert the dynamic image to RGBA image buffer
    // let rgba_image = img.to_rgba8();

    // Get the RGBA image data as bytes
    // let rgba_data = reassembled_image.clone().into_vec();

    // // Convert the RGBA image data to a slice of u32
    // let rgba_data_u32: &[u32] =
    //     unsafe { std::slice::from_raw_parts(rgba_data.as_ptr() as *const u32, rgba_data.len()) };

    // // Create a window to display the image
    // let mut window = Window::new(
    //     &format!("Image Viewer - Opened {} times", 1),
    //     reassembled_image.width() as usize,
    //     reassembled_image.height() as usize,
    //     WindowOptions::default(),
    // )
    // .expect("Failed to create window");

    // while window.is_open() && !window.is_key_down(Key::Escape) {
    //     // Update the window with the image data
    //     window
    //         .update_with_buffer(rgba_data_u32, reassembled_image.width() as usize, reassembled_image.height() as usize)
    //         .expect("Failed to update window");
    // }


    // Convert the image to RGBA8 format for minifb
    let dynamic_image: DynamicImage = DynamicImage::ImageRgba8(reassembled_image);
    let (width, height) = dynamic_image.dimensions();
    let image_rgba = dynamic_image.to_rgba8();
    
    // Convert the image data to a slice of u32
    let buffer: Vec<u32> = image_rgba
        .pixels()
        .map(|p| ((p[0] as u32) << 16) | ((p[1] as u32) << 8) | (p[2] as u32))
        .collect();

    // Create a window with the same dimensions as the image
    let mut window = Window::new(
        "Image Viewer",
        width as usize,
        height as usize,
        WindowOptions {
            scale: Scale::X1,
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Update the window with image data
        window.update_with_buffer(&buffer, width as usize, height as usize).unwrap();
  
    }

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

fn decryption(path: String, img_to_save: String){
    let embedded_image = image::open(path).expect("Failed to open embedded image");
    let cover_image_path = "carrier.png"; // Replace with the actual file path
    let cover_image = image::open(cover_image_path).expect("Failed to open cover image");
    // Extract the hidden image from the embedded image
    let extracted_image = image_decrypt(embedded_image, cover_image);

    // Save the extracted image to a file
    let extracted_image_path = format!("received_imgs/{}", img_to_save);
    extracted_image.save(extracted_image_path.clone()).expect("Failed to save extracted image");
}

fn image_decrypt(image: DynamicImage, cover_image: DynamicImage) -> DynamicImage {
    let image_buffer = image.to_rgba8();
   
    let mut extracted_buffer = ImageBuffer::<Rgba<u8>, _>::new(image.width(), image.height());
   
    for (x, y, pixel) in image_buffer.enumerate_pixels() {
        let cover_pixel = cover_image.get_pixel(x, y);
   
        let (r, g, b, a) = (cover_pixel[0], cover_pixel[1], cover_pixel[2], cover_pixel[3]);
        let (hr, hg, hb, ha) = ((pixel[0] & 0xF) << 4, (pixel[1] & 0xF) << 4, (pixel[2] & 0xF) << 4, 255);
   
        let rgba_pixel = Rgba([hr, hg, hb, ha]);
        extracted_buffer.put_pixel(x, y, rgba_pixel);
    }
   
    DynamicImage::ImageRgba8(extracted_buffer)
   }

fn get_send_server(second_str: &str) -> Option<&'static str> {
    let addresses = SERVER_ADDRESSES.lock().unwrap();

    // Iterate through the addresses and find the first string based on the second string
    let result = addresses.iter()
        .find(|&&(_, s)| s == second_str)
        .map(|&(first_str, _)| first_str);

    // Return the result
    result
}

fn send_to_client(send_addr:SocketAddr){
    //let (sender, receiver) = mpsc::channel();
    let mut message = client_msg_to_client(send_addr);
    match UdpSocket::bind(*SND_CLIENT_SOCKET) {
        Ok(udp_socket) => {
            udp_socket.connect(send_addr).expect("Failed to connect to client");
            message.purpose = "request for image".to_string();
            let response_bytes = bincode::serialize(&message).unwrap();
            udp_socket.send(&response_bytes).expect("client: Write error");
            println!("sent to client {}", send_addr);
            //sender.send(()).expect("Failed to send signal");
        }
        Err(e) => {
            eprintln!("Failed to connect to client {}: {}", send_addr, e);
        }
    }
  //  receiver.recv().expect("Failed to receive signal");
}

fn client_msg_to_client(receiver: SocketAddr) -> Client_Client {
    
    let message = Client_Client {
        ip_Sender: *RCV_CLIENT_SOCKET, // Replace with the actual sender IP
        ip_Reciever: receiver,
        img_id: 0, // Replace with the actual image ID
        img_name: "".to_string(),
        purpose: String::new(), // Replace with the actual purpose
        imgs_height: Vec::new(),
        imgs_width: Vec::new(),
        imgs_size: Vec::new(),
        imgs_names: Vec::new(),
        size: 0,
        views: 0,
        
    };

    return message;
}


fn send_req_for_img(send_addr:SocketAddr){
    //println!("{}", request);
    let request_message = Client_Client {
        ip_Sender: (*RCV_CLIENT_SOCKET),
        ip_Reciever: send_addr,
        img_id: ID,
        img_name: "".to_string(),
        purpose: "req_for_imgs".to_string(),
        imgs_height: Vec::new(),
        imgs_width: Vec::new(),
        imgs_size: Vec::new(),
        imgs_names: Vec::new(),
        size: 0,
        views: 0,
    };

    match UdpSocket::bind(*SND_CLIENT_SOCKET) {
        Ok(udp_socket) => {
            udp_socket.connect(send_addr).expect("Failed to connect to client");
            let response_bytes = bincode::serialize(&request_message).unwrap();
            udp_socket.send(&response_bytes).expect("client: Write error");
            println!("sent to client {}", send_addr);
            //sender.send(()).expect("Failed to send signal");
        }
        Err(e) => {
            eprintln!("Failed to connect to client {}: {}", send_addr, e);
        }
    }

}

// fn request_to_encrypt(){
//     let (sender, receiver) = mpsc::channel();
//     let mut retries = 0;
//     loop {
//         let recv_socket = UdpSocket::bind(recv_addr).expect("Failed to bind recv socket");
//         recv_socket.set_read_timeout(Some(Duration::from_secs(3))).expect("Failed to set timeout");
//         let request = format!("Request from {}", *SND_CLIENT_SOCKET);
//         send_request_for_encrypt(send_addr, &request);
//         let (response, server_rcv) = receive_response_for_encrypt(&recv_socket);
//         if !response.is_empty() {
//             println!("received response: {}", response);
//             let server_address = get_send_server(&server_rcv.unwrap().to_string());
//             println!("{}", server_address.unwrap());
//             sender.send(()).expect("Failed to send signal");
//             image_send(&send_addr, server_address.unwrap().parse().unwrap());
//             receive_image_fragments_over_udp(&recv_socket);
//             break;
//         } else {
//             retries += 1;
//             if retries >= MAX_RETRIES {
//                 println!("maximum retries reached. Exiting.");
//                 break;
//             }
//             println!("timeout occurred. Retrying...");
//         }
//     }
//     receiver.recv().expect("Failed to receive signal");
// }

fn send_request_for_encrypt(request: ClientServer_IP) {
    let server_addresses = SERVER_ADDRESSES.lock().unwrap();
    let send_socket = UdpSocket::bind(*SND_SERVER_SOCKET).expect("Failed to bind send socket");
    println!("{}", request.purpose);

    // Simulate sending a request
    for &(server_address,_) in server_addresses.iter() {
        match UdpSocket::bind(*SND_CLIENT_SOCKET) {
            Ok(udp_socket) => {
                udp_socket.connect(server_address).expect("Failed to connect to server");
                let response_bytes = bincode::serialize(&request).unwrap();
                udp_socket.send(&response_bytes).expect("client: Write error");
                //sender.send(()).expect("Failed to send signal");
            }
            Err(e) => {
                eprintln!("Failed to connect to client {}: {}", server_address, e);
            }
        }
    }

    println!("sent");
}

// fn receive_response_for_encrypt(recv_socket: &UdpSocket) -> (ClientServer_IP, Option<SocketAddr>) {
//     let mut retries = 0;
//     loop {
//         let mut buffer = [0; 1024];
//         match recv_socket.recv_from(&mut buffer) {
//             Ok((size, mut server_address)) => {                
//                 // let data = String::from_utf8_lossy(&buffer[..size]).to_string();
//                 let mut message: ClientMessage = match bincode::deserialize(&buffer) {
//                     Ok(msg) => msg,
//                     Err(_) => todo!(),
//                 };
//                 return (message, Some(server_address));}

//             Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
//                 retries += 1;
//                 if retries >= MAX_RETRIES {
//                     break;
//                 }
//             },
//             Err(e) => {
//                 eprintln!("Failed to receive response: {}", e);
//                 break;
//             },
//         }
//     }
//     let default_message = ClientMessage {
//         ip: "0".to_string(),
//         id: ID, // ID of the server that's shutting down
//         purpose: "request_for_IP".to_string(), // Indicate this is a shutdown message
//         req_ip:"0".to_string(),
//         req_id:0,
//         status: false,
//     };
//     (default_message, None)
// }


fn handle_connections(udp_socket: UdpSocket) {
    let mut buffer = [0; 65000];
    let mut wait_img = false;
    let mut received_fragments = vec![vec![]; 40];
    let mut  count=0;
    let mut img_count =0;
    let mut imgs_names:Vec<String>= vec![String::new(); 1];
    let mut sender_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
    let mut i =0;
    let mut receievd_num=0;
    let mut choosen_img="".to_string();
    let mut wait_ACK = false;
    let mut start_time = Instant::now();
    let duration_to_reach = Duration::from_secs(10);
    let mut ack_check_thread: Option<thread::JoinHandle<()>>;
    // let mut image_path;
    // image_path();
    loop {
        println!("{}", wait_ACK);
        // if wait_ACK {
        //     ack_check_thread = Some(thread::spawn(move || {
        //         while start_time.elapsed() < duration_to_reach {
        //            // println!("waiting");
        //         }
        //         if get_ack()==false{
        //             println!("ACK not received within 10 seconds.");
        //             let ip = get_pending_ip();
        //             let image_name = get_img_name();
        //             let views = get_views();
        //             send_server_to_buffer(ip.unwrap(), image_name.unwrap(), views.unwrap());
        //         }
        //     }));
        // }
        match udp_socket.recv_from(&mut buffer) {
            Ok(( size, receievd_address)) => {
                // let message: Client_Client = match bincode::deserialize(&buffer) {
                //     Ok(msg) => msg,
                //     Err(_) => todo!(),
                // };
                //println!("{}", wait_img);
                if get_status() == true{
                    let size = size; 
                    if wait_img == false{
                        (wait_img, img_count, imgs_names, sender_addr,wait_ACK) = handle_client_connections(&udp_socket, receievd_address, &buffer);       
                        println!("{}, {}, {}", wait_img, img_count, wait_ACK);         
                        if wait_ACK == true{
                            println!("waiting");
                            start_time = Instant::now();
                        }
                    }
                    else {
                        if i < img_count{
                            println!("option: {}", i);
                            // Your code for each iteration goes here
                            if count <40{
                                received_fragments[count].extend_from_slice(&buffer[..size]);
                                println!("Received fragment {} of size {}", count, size);
                                count = count+1;
                            }
                            //println!("{}", count);
                            if count ==40{
                                receive_option_image_fragments_from_client(received_fragments.clone());
                                received_fragments = vec![vec![]; 40];
                                count = 0;
                                i = i+1;
                                if i==img_count {

                                    loop {
                                        println!("Enter the index of the image (between 0 and {}):", img_count - 1);
                                        let mut input = String::new();
                                        io::stdin().read_line(&mut input).expect("Failed to read line");
                                        match input.trim().parse::<usize>() {
                                            Ok(idx) => {
                                                if idx < img_count {
                                                    choosen_img = imgs_names[idx].to_string();
                                                    println!("Chosen image: {}", choosen_img);
                                                    send_req_for_choosen_img(choosen_img.to_string(), sender_addr);

                                                    break; // Break the loop if a valid index is entered
                                                } else {
                                                    println!("Please enter a valid index between 0 and {}.", img_count - 1);
                                                }
                                            }
                                            Err(_) => {
                                                println!("Please enter a valid number.");
                                            }
                                        }
                                    }
                                    // let chooses_idx = get_random_index(img_count);

                                }
                            }
                            
                        } else {
                            if count <40{
                                received_fragments [count].extend_from_slice(&buffer[..size]);
                                println!("Received fragment {} of size {}", count, size);
                                count = count+1;
                            }
                            //println!("{}", count);
                            else if count ==40{
                                let n_views = receive_views(&buffer[..size]);
                                println!("views allowed = {}", n_views);
                                let image_path = receive_image_fragments_from_client(received_fragments.clone());
                                received_fragments = vec![vec![]; 40];
                                //println!("{}", image_path);
                                decryption(image_path.clone(), choosen_img.to_owned());
                                receievd_num = receievd_num +1;
                                wait_img = false;
                                count = 0;
                                add_rcv_image_data(choosen_img.clone().to_owned(), n_views);
                               // print_rcv_image_data();
                               // let path = format!("received_imgs/{}",choosen_img);
                                // for i in 0..n_views {
                                //     show_image( path.clone()); 
                                // }
                                
                                i = 0;
                            }
                           
                        }

                        // else if count==40
                        // {
                        //     image_path = receive_image_fragments_from_client(&mut received_fragments);
                        //     //println!("{}", image_path);
                        //     decryption(image_path);
                        //     wait_img = false;
                        //     count = 0;
                        
                        // }   
                    }
                }
                
            }
            
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
            }
        }
        //ack_check_thread.join().unwrap();
    }

}

// fn handle_server_connections(udp_socket: &UdpSocket, sender_address: SocketAddr, data: &[u8], message: ClientMessage) {
//     // Handle server connections here
//     println!("Recieved from server port: {},{}", message.req_ip, message.purpose);
// }

fn handle_client_connections(udp_socket: &UdpSocket, client_address: SocketAddr, data: &[u8]) -> (bool,usize,Vec<String>, SocketAddr,bool) {
    // Handle client connections here
    let mut message: Client_Client = match bincode::deserialize(data) {
        Ok(msg) => msg,
        Err(_) => todo!(),
    };
    let mut wait_img = false;
    let mut size = 0;
    let mut imgs_names:Vec<String>= vec![String::new(); 1];
    let mut reciever: SocketAddr =  "0.0.0.0:0".parse().unwrap();
    let mut wait_for_ack=false;
    if message.purpose =="req_for_imgs".to_string(){
        send_all_images_low_quality(message.ip_Sender);
    }
    else if message.purpose == "approved".to_string(){
    // println!("Heights: {:?}", message.imgs_height);
    // println!("Widths: {:?}", message.imgs_width);
    // println!("Sizes: {:?}", message.imgs_size);
    // println!("names: {:?}", message.imgs_names);
    wait_img = true;
    size = message.size;
    imgs_names = message.imgs_names;
    reciever = message.ip_Sender;
    }
    else if message.purpose == "req_for_choosen_img" {
       println!("choosen {}", message.img_name );
       add_snd_image(message.img_name.clone(), message.ip_Sender.clone());
       send_choosen_img( message.img_name.clone() , message.ip_Sender);
       thread::sleep(Duration::from_secs(10));
       let c_addr = get_client_by_snd_image_name(&message.img_name.clone());
      // let v =2;
      // send_modify_views_for_client(message.img_name.clone(), c_addr.unwrap(), v);
      // wait_for_ack = true;
       //wait_img = true;
    }
    else if message.purpose == "modify_views" {

        if message.views ==0{
            let path = format!("received_imgs/{}",message.img_name.clone());
            remove_image_by_name(&message.img_name.clone());
            let _ = fs::remove_file(path);
        }else{
            change_views_for_rcv_image(&message.img_name, message.views);
            println!("New vector");
            print_rcv_image_data();
            send_ACK_client(message.ip_Sender);
        }
    }
    else if message.purpose == "ACK"{
        set_ack(true);
    }
    return  (wait_img, size, imgs_names, reciever,wait_for_ack);

}



fn notify_server_wakeup() {
    // Construct a wakeup message
    set_status(true);
    let udp_socket = UdpSocket::bind(*RCV_SERVER_SOCKET).expect("Failed to bind to address");
    let server_addresses = SERVER_ADDRESSES.lock().unwrap();
    let send_socket = UdpSocket::bind(*SND_SERVER_SOCKET).expect("Failed to bind send socket");
    //println!("{}", request);
    let request_message = ClientServer_IP {
        ip_rcv_clients: (*RCV_CLIENT_SOCKET).to_string(),
        ip_rcv_servers: (*RCV_SERVER_SOCKET).to_string(),
        id: ID, // ID of the server that's shutting down
        purpose: "wakeup_client".to_string(), // Indicate this is a shutdown message
        req_ip: Vec::new(),
        req_id:Vec::new(),
        pending_ip: "0.0.0.0:0".parse().unwrap(),
        pending_views: 0,
        pending_img_name: "".to_string(),
    };

    let response_bytes = bincode::serialize(&request_message).unwrap();
    // Simulate sending a request
    for &(server_address,_) in server_addresses.iter() {
        if let Err(e) = send_socket.send_to(&response_bytes, server_address) {
            eprintln!("Failed to send request to {}: {}", server_address, e);
        }
    }
    listen_for_pending(&udp_socket);
    println!("wakeup");

}

fn listen_for_pending(udp_socket: &UdpSocket){
    let mut buffer = [0; 65000];
    let mut should_break = false;
    // let mut image_path;
    // image_path();
    loop {
        println!("Wait for pending");
        match udp_socket.recv_from(&mut buffer) {
            Ok(( size, receievd_address)) => {
                let mut message: ClientServer_IP = match bincode::deserialize(&buffer) {
                    Ok(msg) => msg,
                    Err(_) => todo!(),
                };
                println!("{}",message.purpose);
                if message.purpose == "finished"{
                    //println!("No more pending requests");
                    should_break = true;
                    break;
                }
                else if message.purpose ==  "Pending req"{
                    println!("pending request");
                    change_views_for_rcv_image(&message.pending_img_name, message.pending_views);
                    print_rcv_image_data();
                }
            }
            
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
            }
        }
        if should_break {
            break;
        }
        //ack_check_thread.join().unwrap();
    }
}
fn go_down(){
    set_status(false);
}


fn send_all_images_low_quality(receiver: SocketAddr){
    let mut message = client_msg_to_client(receiver);
    message.purpose = "approved". to_string();
    message.ip_Sender= *RCV_CLIENT_SOCKET;
    let (heights, widths, sizes, names) = get_imgs_info();
    message.size = names.len();
    // message.imgs_height = heights;
    // message.imgs_width = widths;
    // message.imgs_size = sizes;
    message.imgs_names = names.clone();

     match UdpSocket::bind(*SND_CLIENT_SOCKET) {
        Ok(udp_socket) => {
            udp_socket.connect(receiver).expect("Failed to connect to client");

            // Send lengths followed by elements for names
            let response_bytes = bincode::serialize(&message).unwrap();
            udp_socket.send(&response_bytes).expect("client: Write error");
            println!("sent to client {}", receiver);
            //sender.send(()).expect("Failed to send signal");
        }
        Err(e) => {
            eprintln!("Failed to connect to client {}: {}", receiver, e);
        }
    }

    for name in names {
        println!("image sent: {}", name);
        image_send_low_quality(name, receiver);
    }
    // match UdpSocket::bind(*SND_CLIENT_SOCKET) {
    //     Ok(udp_socket) => {
    //         udp_socket.connect(receiver).expect("Failed to connect to client");
    //         // let response_bytes = bincode::serialize(&message).unwrap();
    //         // udp_socket.send(&response_bytes).expect("client: Write error");

    //         // Send lengths followed by elements for names
    //         udp_socket.send(&bincode::serialize(&names.len()).unwrap()).expect("client: Write error");
    //         for name in &names {
    //             let serialized = bincode::serialize(name).unwrap();
    //             udp_socket.send(&serialized).expect("client: Write error");
    //         }
            
    //         println!("sent to client {}", receiver);
    //         //sender.send(()).expect("Failed to send signal");
    //     }
    //     Err(e) => {
    //         eprintln!("Failed to connect to client {}: {}", receiver, e);
    //     }
    // }

    // fn image_send_low_quality(img_path: String, server_addr:SocketAddr){

}

fn get_imgs_info() -> (Vec<u32>, Vec<u32>, Vec<u32>, Vec<String>) {
    let mut imgs_height: Vec<u32> = Vec::new();
    let mut imgs_width: Vec<u32> = Vec::new();
    let mut imgs_size: Vec<u32> = Vec::new();
    let mut imgs_name: Vec<String> = Vec::new();
    let source_directory = "images";

    // Extensions to process
    let extensions = vec!["jpg", "png", "gif", "bmp"];

    for extension in extensions {
        let pattern = format!("{}/*.{}", source_directory, extension);
        for entry in glob(&pattern).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    println!("Processing file: {:?}", path);
                    if let Ok(file) = File::open(&path) {
                        if let Ok(metadata) = file.metadata() {
                            let file_size = metadata.len();
                            println!("File Size: {} bytes", file_size);

                            if let Ok(img) = image::open(&path) {
                                let (width, height) = img.dimensions();
                                // let color_type = img.color();
                                // let bit_depth = img.color().bits_per_pixel();

                                imgs_height.push(height);
                                imgs_width.push(width);
                                imgs_size.push(file_size as u32);
                                imgs_name.push(path.file_name().unwrap().to_string_lossy().into_owned());
                            } else {
                                println!("Failed to open image: {:?}", path);
                            }
                        } else {
                            println!("Failed to read file metadata: {:?}", path);
                        }
                    } else {
                        println!("Failed to open file: {:?}", path);
                    }
                }
                Err(e) => println!("Error processing file: {:?}", e),
            }
        }
    }

    (imgs_height, imgs_width, imgs_size, imgs_name)
}

fn get_random_index(vector_size: usize) -> usize {
    let mut rng = rand::thread_rng();
    let random_index = rng.gen_range(0..vector_size);
    random_index
}

fn send_req_for_choosen_img(path: String, send_addr:SocketAddr){
    let request_message = Client_Client {
        ip_Sender: (*RCV_CLIENT_SOCKET),
        ip_Reciever: send_addr,
        img_id: ID,
        img_name: path,
        purpose: "req_for_choosen_img".to_string(),
        imgs_height: Vec::new(),
        imgs_width: Vec::new(),
        imgs_size: Vec::new(),
        imgs_names: Vec::new(),
        size: 0,
        views: 0,
    };

    match UdpSocket::bind(*SND_CLIENT_SOCKET) {
        Ok(udp_socket) => {
            udp_socket.connect(send_addr).expect("Failed to connect to client");
            let response_bytes = bincode::serialize(&request_message).unwrap();
            udp_socket.send(&response_bytes).expect("client: Write error");
            println!("sent to client {}", send_addr);
            //sender.send(()).expect("Failed to send signal");
        }
        Err(e) => {
            eprintln!("Failed to connect to client {}: {}", send_addr, e);
        }
    }

}

fn send_choosen_img(img_name: String, send_addr_client: SocketAddr){

    // let image = read_image_by_index( img_index);
    let request_message = ClientServer_IP {
        ip_rcv_clients: (*RCV_CLIENT_SOCKET).to_string(),
        ip_rcv_servers: (*RCV_SERVER_SOCKET).to_string(),
        id: ID, // ID of the server that's shutting down
        purpose: "req_for_encrypt".to_string(), // Indicate this is a shutdown message
        req_ip: Vec::new(),
        req_id:Vec::new(),
        pending_ip: "0.0.0.0:0".parse().unwrap(),
        pending_views: 0,
        pending_img_name: "".to_string(),
    };
    let img_path = format!("images/{}", img_name.clone());
    let mut retries = 0;
    loop {
        let recv_socket = UdpSocket::bind(*RCV_SERVER_SOCKET).expect("Failed to bind recv socket");
        recv_socket.set_read_timeout(Some(Duration::from_secs(3))).expect("Failed to set timeout");
        send_request_for_encrypt(request_message.clone());
        let (response, server_rcv) = receive_response_IPS(&recv_socket);
        // println!("{}", response.req_ip);
        if response.purpose =="send_fragments".to_string() {
            println!("received response from server : {}", server_rcv.unwrap());
            let server_address = get_send_server(&server_rcv.unwrap().to_string());
            image_send( img_path, server_address.unwrap().parse().unwrap());
            let encrypt_path=receive_image_fragments_over_udp(&recv_socket);
            image_send(encrypt_path.to_owned(), send_addr_client);
            let mut rng = rand::thread_rng();
            let random_views = 3;
            send_n_views(random_views, send_addr_client);
            break;
        } else {
            retries += 1;
            if retries >= MAX_RETRIES {
                println!("maximum retries reached. Exiting.");
                break;
            }
            println!("timeout occurred. Retrying...");
        }
    }
    // send_request_for_encrypt(request_message);
    // receive_response(recv_socket)
}

fn send_n_views(random_views: usize, send_addr:SocketAddr){
    println!("views allowed = {}", random_views);
    match UdpSocket::bind(*SND_CLIENT_SOCKET) {
        Ok(udp_socket) => {
            udp_socket.connect(send_addr).expect("Failed to connect to client");
            let response_bytes = bincode::serialize(&random_views).unwrap();
            udp_socket.send(&response_bytes).expect("client: Write error");
            println!("sent to client {}", send_addr);
            //sender.send(()).expect("Failed to send signal");
        }
        Err(e) => {
            eprintln!("Failed to connect to client {}: {}", send_addr, e);
        }
    }
}

fn receive_views(data: &[u8]) -> usize{

    let mut views: usize = match bincode::deserialize(data) {
        Ok(msg) => msg,
        Err(_) => todo!(),
    };

    return views;
}

fn show_image(img_path: String) {

    let image = image::open(img_path).expect("Failed to open image");
    // Get image dimensions
    let (width, height) = image.dimensions();

    // Convert the image to RGBA8 format for minifb
    let image_rgba = image.to_rgba8();
    
    // Convert the image data to a slice of u32
    let buffer: Vec<u32> = image_rgba
        .pixels()
        .map(|p| ((p[0] as u32) << 16) | ((p[1] as u32) << 8) | (p[2] as u32))
        .collect();

    // Create a window with the same dimensions as the image
    let mut window = Window::new(
        "Image Viewer",
        width as usize,
        height as usize,
        WindowOptions {
            scale: Scale::X1,
            ..Default::default()
        },
    )
    .unwrap_or_else(|e| {
        panic!("{}", e);
    });

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Update the window with image data
        window.update_with_buffer(&buffer, width as usize, height as usize).unwrap();
  
    }
}

fn image_send_low_quality(img_name: String, server_addr:SocketAddr){
    let img_path = format!("images/{}", img_name);
    let img = image::open(img_path).expect("Failed to open image");

    let new_width = 50; // Set your desired width
    let new_height = 50; // Set your desired height
    
    let resized_img = img.resize_exact(new_width, new_height, image::imageops::FilterType::Triangle);
    let rgba_img = resized_img.to_rgba8();
    // Save the resized image
    // resized_img.save("resized_image.jpg").unwrap();

    let (fragments, width, height) = fragment_image(&rgba_img);
    let mut fragment_paths = Vec::new();

    for (i, fragment) in fragments.iter().enumerate() {
        let path = format!("fragment_{}.png", i);
        fragment.save(&path).expect("Failed to save fragment");
        fragment_paths.push(path);
    }
    send_image_fragments_over_udp(fragment_paths, server_addr);

    //reassemble 
    // let reassembled = reassemble_image(&fragments, width, height);
    // reassembled.save("reassembled.png").expect("Failed to save reassembled image");
}

fn set_status(value: bool) {
    let mut status = STATUS.lock().unwrap();
    *status = value;
}

// Getter function to retrieve the value
fn get_status() -> bool {
    let status = STATUS.lock().unwrap();
    *status
}

// Function to add a string and corresponding number to the vector
fn add_rcv_image_data(image_name: String, image_number: usize) {
    let mut data = RCV_IMGS.lock().unwrap();
    data.push((image_name, image_number));
}

// Function to get all image names from the vector
fn get_all_rcv_image_names() -> Vec<String> {
    let data = RCV_IMGS.lock().unwrap();
    data.iter().map(|&(ref name, _)| name.clone()).collect()
}

// Function to get the number corresponding to a specific image name from the vector
fn get_views_for_rcv_image(image_name: &str) -> Option<usize> {
    let data = RCV_IMGS.lock().unwrap();
    data.iter()
        .find(|&&(ref name, _)| name == image_name)
        .map(|&(_, num)| num)
}

// Function to remove an item by name from the vector
fn remove_image_by_name(image_name: &str) {
    let mut data = RCV_IMGS.lock().unwrap();
    data.retain(|&(ref name, _)| name != image_name);
}

// Function to change the number corresponding to an image by name
fn change_views_for_rcv_image(image_name: &str, new_number: usize) {
    let mut data = RCV_IMGS.lock().unwrap();
    if let Some(index) = data.iter().position(|&(ref name, _)| name == image_name) {
        data[index].1 = new_number;
    }
}

fn print_rcv_image_data() {
    let data = RCV_IMGS.lock().unwrap();
    for (image_name, image_number) in data.iter() {
        println!("Image Name: {}, Image View: {}", image_name, image_number);
    }
}

// Function to add a string and corresponding SocketAddr to the vector
fn add_snd_image(image_name: String, socket_address: SocketAddr) {
    let mut data = SND_IMGS.lock().unwrap();
    data.push((image_name, socket_address));
}

// Function to get SocketAddr by image name
fn get_client_by_snd_image_name(image_name: &str) -> Option<SocketAddr> {
    let data = SND_IMGS.lock().unwrap();
    data.iter()
        .find(|&&(ref name, _)| name == image_name)
        .map(|&(_, socket)| socket)
}

// Function to print the vector contents
fn print_snd_image() {
    let data = SND_IMGS.lock().unwrap();
    for (image_name, socket) in data.iter() {
        println!("Image Name: {}, Client Address: {}", image_name, socket);
    }
}

fn send_modify_views_for_client(image_name: String, send_addr: SocketAddr, views: usize) -> bool{

    let mut message = client_msg_to_client(send_addr);
    message.img_name = image_name.clone();
    message.purpose = "modify_views".to_string();
    message.views = views;

    let mut iterate = 1;
    match UdpSocket::bind(*SND_CLIENT_SOCKET) {
        Ok(udp_socket) => {
            udp_socket.connect(send_addr).expect("Failed to connect to client");
            let response_bytes = bincode::serialize(&message).unwrap();
            udp_socket.send(&response_bytes).expect("client: Write error");
            println!("sent to client {}", send_addr);
            //sender.send(()).expect("Failed to send signal");
        }
        Err(e) => {
            eprintln!("Failed to connect to client {}: {}", send_addr, e);
        }
    }
    
    change_pending(send_addr, image_name, views);
        // thread::sleep(Duration::from_secs(10));
        // if get_ack() ==true{
        //     println!("ACK recieved");
        //     break;
        // }
        // if (get_ack() == false) && (iterate <3){
        //     iterate = iterate + 1;
        //     println!("client not responded, retry");
        // }
        // else {
        //     //send server for DoS
        //     println!("Client offline");
        //     break;
        // }
    // set_ack(false);
    return true;
    
}

fn set_ack(value: bool) {
    let mut ack = ACK.lock().unwrap();
    *ack = value;
}

// Getter function to retrieve the value
fn get_ack() -> bool {
    let ack = ACK.lock().unwrap();
    *ack
}

fn send_ACK_client(send_addr:SocketAddr){
    //let (sender, receiver) = mpsc::channel();
    let mut message = client_msg_to_client(send_addr);
    message.purpose = "ACK".to_string();
    match UdpSocket::bind(*SND_CLIENT_SOCKET) {
        Ok(udp_socket) => {
            udp_socket.connect(send_addr).expect("Failed to connect to client");
            let response_bytes = bincode::serialize(&message).unwrap();
            udp_socket.send(&response_bytes).expect("client: Write error");
            println!("sent to client {}", send_addr);
            //sender.send(()).expect("Failed to send signal");
        }
        Err(e) => {
            eprintln!("Failed to connect to client {}: {}", send_addr, e);
        }
    }
  //  receiver.recv().expect("Failed to receive signal");
}

fn send_server_to_buffer(pend_ip: SocketAddr, img_name:String, views: usize) {
    let server_addresses = SERVER_ADDRESSES.lock().unwrap();
    let send_socket = UdpSocket::bind(*SND_SERVER_SOCKET).expect("Failed to bind send socket");
    println!("Server request to save in buffer");
    let request_message = ClientServer_IP {
        ip_rcv_clients: (*RCV_CLIENT_SOCKET).to_string(),
        ip_rcv_servers: (*RCV_SERVER_SOCKET).to_string(),
        id: ID, // ID of the server that's shutting down
        purpose: "save_in_buffer".to_string(), // Indicate this is a shutdown message
        req_ip: Vec::new(),
        req_id:Vec::new(),
        pending_ip: pend_ip,
        pending_views: views,
        pending_img_name: img_name,
    };

    let response_bytes = bincode::serialize(&request_message).unwrap();
    // Simulate sending a request
    for &(server_address,_) in server_addresses.iter() {
        if let Err(e) = send_socket.send_to(&response_bytes, server_address) {
            eprintln!("Failed to send request to {}: {}", server_address, e);
        }
    }

    println!("sent");
   // &send_socket
}

fn change_pending(ip: SocketAddr, name: String, views: usize) {
    *PENDING_IP.lock().unwrap() = Some(ip);
    *IMG_NAME.lock().unwrap() = Some(name);
    *VIEWS.lock().unwrap() = Some(views);
}

fn get_pending_ip() -> Option<SocketAddr> {
    (*PENDING_IP.lock().unwrap()).clone()
}

fn get_img_name() -> Option<String> {
    (*IMG_NAME.lock().unwrap()).clone()
}

fn get_views() -> Option<usize> {
    (*VIEWS.lock().unwrap()).clone()
}

fn get_image_name_by_index(index: usize) -> Option<String> {
    let img_list = RCV_IMGS.lock().unwrap();

    if let Some((name, _)) = img_list.get(index) {
        Some(name.clone())
    } else {
        None
    }
}

fn get_snd_img_by_index(index: usize) -> Option<String> {
    let snd_imgs = SND_IMGS.lock().unwrap();

    if let Some((img_name,_)) = snd_imgs.get(index) {
        Some(img_name.to_string())
    } else {
        None
    }
}

fn get_snd_client_by_index(index: usize) -> Option<SocketAddr> {
    let snd_imgs = SND_IMGS.lock().unwrap();

    if let Some((_, socket_addr)) = snd_imgs.get(index) {
        Some(*socket_addr)
    } else {
        None
    }
}
fn get_snd_size() -> usize {
    let snd_imgs = SND_IMGS.lock().unwrap();
    snd_imgs.len()
}