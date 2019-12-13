use std::{fs, io, str};

use rustls::internal::pemfile;

// Load public certificate from file.
pub fn load_certs(filename: &str) -> io::Result<Vec<rustls::Certificate>> {
    // Open certificate file.
    let certfile = fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
    let mut reader = io::BufReader::new(certfile);

    // Load and return certificate.
    let certs = pemfile::certs(&mut reader)
        .map_err(|_| error("failed to load certificate".into()))
        .unwrap();
    info!("Certs = {:?}", certs.len());
    if certs.len() == 0 {
        return Err(error("expected at least one certificate".into()));
    }
    Ok(certs)
}

// Load private key from file.
pub fn load_private_key(filename: &str) -> io::Result<rustls::PrivateKey> {
    // Open keyfile.
    let keyfile = fs::File::open(filename)
        .map_err(|e| error(format!("failed to open {}: {}", filename, e)))?;
    let mut reader = io::BufReader::new(keyfile);

    // Load and return a single private key.
    let keys = pemfile::rsa_private_keys(&mut reader);

    let keys = match keys {
        Ok(keys) => keys,
        Err(error) => panic!("There was a problem with reading private key: {:?}", error),
    };
    info!("Keys = {:?}", keys.len());
    if keys.len() != 1 {
        return Err(error("expected a single private key".into()));
    }
    Ok(keys[0].clone())
}

fn error(err: String) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}
