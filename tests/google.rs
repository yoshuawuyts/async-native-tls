#![warn(rust_2018_idioms)]

use std::io;
use std::net::ToSocketAddrs;

use async_native_tls;
use async_std::net::TcpStream;
use async_std::prelude::*;
use cfg_if::cfg_if;
use env_logger;
use native_tls;
use native_tls::TlsConnector;

macro_rules! t {
    ($e:expr) => {
        match $e {
            Ok(e) => e,
            Err(e) => panic!("{} failed with {:?}", stringify!($e), e),
        }
    };
}

cfg_if! {
    if #[cfg(any(feature = "force-openssl",
                        all(not(target_os = "macos"),
                            not(target_os = "windows"),
                            not(target_os = "ios"))))] {
        fn assert_bad_hostname_error(err: &io::Error) {
            let err = err.get_ref().unwrap();
            let err = err.downcast_ref::<native_tls::Error>().unwrap();
            assert!(format!("{}", err).contains("certificate verify failed"));
        }
    } else if #[cfg(any(target_os = "macos", target_os = "ios"))] {
        fn assert_bad_hostname_error(err: &io::Error) {
            let err = err.get_ref().unwrap();
            let err = err.downcast_ref::<native_tls::Error>().unwrap();
            assert!(format!("{}", err).contains("was not trusted."));
        }
    } else {
        fn assert_bad_hostname_error(err: &io::Error) {
            let err = err.get_ref().unwrap();
            let err = err.downcast_ref::<native_tls::Error>().unwrap();
            assert!(format!("{}", err).contains("CN name"));
        }
    }
}

#[async_std::test]
async fn fetch_google() {
    drop(env_logger::try_init());

    // First up, resolve google.com
    let addr = t!("google.com:443".to_socket_addrs()).next().unwrap();

    let socket = TcpStream::connect(&addr).await.unwrap();

    // Send off the request by first negotiating an SSL handshake, then writing
    // of our request, then flushing, then finally read off the response.
    let builder = TlsConnector::builder();
    let connector = t!(builder.build());
    let connector = async_native_tls::TlsConnector::from(connector);
    let mut socket = t!(connector.connect("google.com", socket).await);
    t!(socket.write_all(b"GET / HTTP/1.0\r\n\r\n").await);
    let mut data = Vec::new();
    t!(socket.read_to_end(&mut data).await);

    // any response code is fine
    assert!(data.starts_with(b"HTTP/1.0 "));

    let data = String::from_utf8_lossy(&data);
    let data = data.trim_end();
    assert!(data.ends_with("</html>") || data.ends_with("</HTML>"));
}

fn native2io(e: native_tls::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, e)
}
