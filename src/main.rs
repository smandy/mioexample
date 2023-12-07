use mio::{Events, Poll, PollOpt, Ready, Token};
use mio::net::{TcpListener, TcpStream};
use mio_uds::UnixListener;
use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::thread;

const SERVER_TOKEN: Token = Token(0);

struct ChatServer {
    poll: Poll,
    listener: TcpListener,
    clients: HashMap<Token, TcpStream>,
    next_token: usize,
}

impl ChatServer {
    fn new(listener: TcpListener) -> Self {
        let poll = Poll::new().expect("Failed to create poll instance");

        poll.register(
            &listener,
            SERVER_TOKEN,
            Ready::readable(),
            PollOpt::edge(),
        )
        .expect("Failed to register listener with poll");

        ChatServer {
            poll,
            listener,
            clients: HashMap::new(),
            next_token: 1,
        }
    }

    fn accept_new_client(&mut self) {
        match self.listener.accept() {
            Ok((stream, _)) => {
                let token = Token(self.next_token);
                self.next_token += 1;

                self.poll
                    .register(&stream, token, Ready::readable(), PollOpt::edge())
                    .expect("Failed to register client with poll");

                self.clients.insert(token, stream);
                println!("New client connected with token: {:?}", token);
            }
            Err(e) => eprintln!("Failed to accept new client: {:?}", e),
        }
    }

    fn handle_client_event(&mut self, token: Token) {
        if let Some(mut client) = self.clients.get_mut(&token) {
            let mut buffer = [0; 1024];
            match client.read(&mut buffer) {
                Ok(0) => {
                    // Client disconnected
                    self.clients.remove(&token);
                    println!("Client disconnected: {:?}", token);
                }
                Ok(bytes_read) => {
                    // Echo the received data to all clients
                    for (other_token, other_client) in &self.clients {
                        if *other_token != token {
                            other_client
                                .write_all(&buffer[0..bytes_read])
                                .expect("Failed to write to client");
                        }
                    }
                }
                Err(e) => eprintln!("Error reading from client: {:?}", e),
            }
        }
    }

    fn run(&mut self) {
        let mut events = Events::with_capacity(1024);

        loop {
            self.poll.poll(&mut events, None).expect("Failed to poll events");

            for event in &events {
                match event.token() {
                    SERVER_TOKEN => self.accept_new_client(),
                    token => self.handle_client_event(token),
                }
            }
        }
    }
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").expect("Failed to bind to address");

    let mut chat_server = ChatServer::new(listener);
    println!("Chat server is running on 127.0.0.1:8080");

    // Run the server in a separate thread
    thread::spawn(move || chat_server.run());

    // Keep the main thread alive
    loop {
        thread::sleep(std::time::Duration::from_secs(10));
    }
}
