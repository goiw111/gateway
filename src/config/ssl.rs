use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod, SslAcceptorBuilder};

pub fn get_acceptor<T: AsRef<str>>(key: T,cert: T) -> SslAcceptorBuilder {
    let mut acceptor = SslAcceptor::mozilla_modern_v5(SslMethod::tls())
        .unwrap();
    acceptor.set_private_key_file(key.as_ref(), SslFiletype::PEM)
        .unwrap();
    acceptor.set_certificate_chain_file(cert.as_ref())
        .unwrap();
    acceptor
}
