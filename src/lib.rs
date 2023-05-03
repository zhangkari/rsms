/*
 * file name:  lib.rs
 */
#[allow(unused_variables, dead_code)]
pub mod rsms {
    pub mod infra {
        macro_rules! log_msg {
            ($msg:expr) => {
                eprintln!("{} <file:{}, line:{}>", $msg, file!(), line!());
            };
        }

        pub mod log {
            pub fn d(msg: &str) {
                log_msg!(msg);
            }

            pub fn v(msg: &str) {
                eprintln!("{}", msg);
            }
        }
    }

    pub mod core {
        use std::collections::LinkedList;
        use std::hash::{Hash, Hasher};
        use std::net::TcpListener;
        use std::thread;
        use std::time::Duration;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpStream;

        // region: Category
        #[repr(u8)]
        #[derive(PartialEq, Eq, Copy, Clone)]
        enum Category {
            INVALID,
            RTMP,
            HTTP,
            RTSP,
        }

        impl Category {
            fn from(name: &str) -> Category {
                return match name {
                    "RTMP" => Self::RTMP,
                    "HTTP" => Self::HTTP,
                    "RTSP" => Self::RTSP,
                    _ => Self::INVALID,
                };
            }
        }

        impl Hash for Category {
            fn hash<H: Hasher>(&self, state: &mut H) {
                (*self as u8).hash(state);
            }
        }
        // endregion: Category

        // region: Session
        struct Session {
            stream: TcpStream,
            category: Category,
            port: u16,
        }

        impl Session {
            fn new(stream: TcpStream, port: u16, category: Category) -> Session {
                Session {
                    stream,
                    category,
                    port,
                }
            }
        }
        impl Hash for Session {
            fn hash<H: Hasher>(&self, state: &mut H) {
                self.category.hash(state);
                self.stream.local_addr().unwrap().hash(state);
                self.stream.peer_addr().unwrap().hash(state);
                self.port.hash(state);
            }
        }

        impl Eq for Session {}

        impl PartialEq for Session {
            fn eq(&self, other: &Self) -> bool {
                self.port == other.port
                    && self.category == other.category
                    && self.stream.local_addr().unwrap() == other.stream.local_addr().unwrap()
                    && self.stream.peer_addr().unwrap() == other.stream.peer_addr().unwrap()
            }
        }
        // endregion: Session

        // region: Profile
        #[derive(Debug)]
        pub struct Profile {
            name: &'static str,
            port: u16,
            log: bool,
            enable: bool,
        }

        impl Profile {
            const RTMP: Profile = Profile {
                name: "RTMP",
                port: 1935,
                log: true,
                enable: true,
            };

            const HTTP: Profile = Profile {
                name: "HTTP",
                port: 80,
                log: true,
                enable: true,
            };

            const RTSP: Profile = Profile {
                name: "RTSP",
                port: 554,
                log: true,
                enable: true,
            };

            const GB28181: Profile = Profile {
                name: "GB28181",
                port: 5060,
                log: true,
                enable: true,
            };

            const API_ADMIN: Profile = Profile {
                name: "HTTP",
                port: 8080,
                log: true,
                enable: true,
            };

            fn new(name: &'static str, port: u16, log: bool, enable: bool) -> Profile {
                return Profile {
                    name,
                    port,
                    log,
                    enable,
                };
            }
        }
        // endregion: Profile

        // region: Context
        pub struct Context {
            sessions: LinkedList<Session>,
            watchdog: Watchdog,
            analyzer: Analyzer,
            pub incoming: Option<std::net::Incoming<'static>>,
            pub listener: Option<TcpListener>,
            read_buf: [u8; 1024],
            write_buf: [u8; 1024],
        }

        impl Context {
            pub fn new() -> Context {
                return Context {
                    sessions: LinkedList::new(),
                    watchdog: Watchdog::new(String::from("Watchdog")),
                    analyzer: Analyzer::new(),
                    read_buf: [0; 1024],
                    write_buf: [0; 1024],
                    incoming: None,
                    listener: None,
                };
            }
        }
        // endregion: Context

        // region: Analyzer
        struct Analyzer {
            publishers: u16,
            subscribers: u16,
            api_admins: u16,
            delay_ms: u16,
        }
        impl Analyzer {
            pub(crate) fn new() -> Analyzer {
                Analyzer {
                    publishers: 0,
                    subscribers: 0,
                    api_admins: 0,
                    delay_ms: 0,
                }
            }
        }
        // endregion: Analyzer

        // region: WatchDog
        struct Watchdog {
            name: String,
            status: u8,
            counter: u64,
            threshold: u16,
        }
        impl Watchdog {
            fn new(name: String) -> Watchdog {
                Watchdog {
                    name,
                    status: 0,
                    counter: 0,
                    threshold: 10,
                }
            }
        }
        // endregion: WatchDog

        pub trait Serve {
            fn init(&mut self);
            fn start(&mut self);
            fn stop(&mut self);
            fn destroy(&mut self);
            fn on_read(&mut self);
            fn on_write(&mut self);
            fn on_error(&mut self);
        }
        // region: Cotributor
        pub struct Contributor {
            profile: Profile,
            context: Context,
        }

        impl Contributor {
            pub fn from(profile: Profile) -> Contributor {
                Contributor {
                    profile,
                    context: Context::new(),
                }
            }

            pub async fn startup(&mut self) {
                let addr = format!("127.0.0.1:{}", self.profile.port);
                let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

                // listener.set_nonblocking(true);

                if self.profile.log {
                    println!("{} Bind {}", &self.profile.name, &addr);
                }

                // self.context.listener = Some(listener);
                // self.context.incoming = Some(&mut listener.incoming());

                loop {
                    let (socket, addr) = listener.accept().await.expect("accept error");
                    if self.profile.log {
                        println!("{} Request from:{}", &self.profile.name, addr.to_string());
                    }

                    let session =
                        Session::new(socket, self.profile.port, Category::from(self.profile.name));

                    let _handle = tokio::spawn(async move {
                        let mut buf = [0; 1024];
                        let mut socket = session.stream;
                        loop {
                            let n = match socket.read(&mut buf).await {
                                Ok(0) => return,
                                Ok(n) => n,
                                Err(e) => {
                                    eprintln!("failed to read from socket; err = {:?}", e);
                                    return;
                                }
                            };

                            println!("Recv:{}", std::str::from_utf8(&buf).unwrap());

                            let send_buf = "HTTP/1.1 200 OK\r\n\r\n\r\n<h1>Good</h1>";

                            if let Err(e) = socket.write_all(send_buf.as_bytes()).await {
                                eprintln!("failed to write to socket; err = {:?}", e);
                                return;
                            };

                            // self.context.sessions.push_back(session);
                        }
                    })
                    .await;
                }
            }
        }

        impl Serve for Contributor {
            fn init(&mut self) {}

            fn start(&mut self) {}

            fn stop(&mut self) {}

            fn destroy(&mut self) {}

            fn on_read(&mut self) {}

            fn on_write(&mut self) {}

            fn on_error(&mut self) {}
        }
        // endregion: Contributor

        // region: Commander
        pub struct Commander {
            pub this: Contributor,
            pub others: Vec<Contributor>,
        }

        impl Commander {
            fn from(profile: Profile) -> Commander {
                Commander {
                    this: Contributor::from(profile),
                    others: vec![],
                }
            }

            pub fn new() -> Commander {
                Commander::from(Profile::API_ADMIN)
            }

            pub fn run_loop(&mut self) {
                loop {
                    /*
                    select! {
                        Ok((socket, addr)) = self.this.context.listener.incoming().unwrap().next() => {

                        },
                    }
                    */

                    thread::sleep(Duration::from_secs(20));
                }
            }
        }

        impl Serve for Commander {
            fn init(&mut self) {
                self.others.push(Contributor::from(Profile::RTMP));
                self.others.push(Contributor::from(Profile::HTTP));
                self.others.push(Contributor::from(Profile::RTSP));

                self.this.init();
                for item in &mut self.others {
                    item.init();
                }
            }

            fn start(&mut self) {
                self.this.start();
                for item in &mut self.others {
                    item.start();
                }
            }

            fn stop(&mut self) {
                for item in &mut self.others {
                    item.stop();
                }
            }

            fn destroy(&mut self) {
                for item in &mut self.others {
                    item.destroy();
                }
            }

            fn on_read(&mut self) {}

            fn on_write(&mut self) {}

            fn on_error(&mut self) {}
        }
        // endregion: Commander
    }

    mod admin {
        use actix_web::{get, web, App, HttpServer, Responder};

        use super::core::Contributor;

        #[get("/hello/{name}")]
        async fn greet(name: web::Path<String>) -> impl Responder {
            format!("Hello {name}!")
        }
        pub async fn init() -> std::io::Result<()> {
            HttpServer::new(|| App::new().service(greet))
                .bind("127.0.0.1:9999")?
                .run()
                .await
        }

        struct AdminContributor {
            this: Contributor,
        }
    }
}
