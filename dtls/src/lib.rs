use openssl::ssl::{
    SslMethod,
    SslConnector
};

fn client() {
    SslConnector::builder(SslMethod::dtls()).unwrap()
}
