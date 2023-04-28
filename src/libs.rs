pub mod rsms {
    pub mod infra {
        macro_rules! log_error {
            ($msg:expr) => (
                eprintln!("{} <file:{}, line:{}>", $msg, file!(), line!());
            )
        }

        pub fn log(msg:&str) {
            log_error!(msg);
        }
    }

    pub mod core {
        pub fn push() {
            println!("push()");
        }
    }
}