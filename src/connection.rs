use redis::cluster::{ClusterClient, ClusterConnection};
use redis::{Client, ClientTlsConfig, Connection, RedisResult, TlsCertificates};
use std::fs;

/// Connection parameters for creating Redis connections
#[derive(Clone, Debug)]
pub struct ConnParams {
    pub host: String,
    pub port: String,
    pub password: Option<String>,
    pub unix: Option<String>,
    pub tls: bool,
    pub tls_ca_cert: Option<String>,
    pub tls_client_cert: Option<String>,
    pub tls_client_key: Option<String>,
}

impl ConnParams {
    pub fn from_config(config: &super::Config) -> Self {
        ConnParams {
            host: config.host.clone(),
            port: config.port.clone(),
            password: config.password.clone(),
            unix: config.unix.clone(),
            tls: config.tls,
            tls_ca_cert: config.tls_ca_cert.clone(),
            tls_client_cert: config.tls_client_cert.clone(),
            tls_client_key: config.tls_client_key.clone(),
        }
    }
}

pub fn connect(
    host: &str,
    port: &str,
    password: Option<&str>,
    unix: Option<&str>,
    tls: bool,
    tls_ca_cert: Option<&str>,
    tls_client_cert: Option<&str>,
    tls_client_key: Option<&str>,
) -> RedisResult<Connection> {
    let client = if let Some(unix_path) = unix {
        Client::open(format!("unix://{}", unix_path))?
    } else {
        let protocol = if tls { "rediss" } else { "redis" };
        let url = if let Some(password) = password {
            format!("{}://:{}@{}:{}", protocol, password, host, port)
        } else {
            format!("{}://{}:{}", protocol, host, port)
        };

        if tls && (tls_ca_cert.is_some() || tls_client_cert.is_some() || tls_client_key.is_some()) {
            let root_cert = tls_ca_cert.map(|path| fs::read(path)).transpose()?;
            let client_tls = match (tls_client_cert, tls_client_key) {
                (Some(cert_path), Some(key_path)) => Some(ClientTlsConfig {
                    client_cert: fs::read(cert_path)?,
                    client_key: fs::read(key_path)?,
                }),
                _ => None,
            };
            let certs = TlsCertificates {
                client_tls,
                root_cert,
            };
            Client::build_with_tls(url.as_str(), certs)?
        } else {
            Client::open(url.as_str())?
        }
    };

    client.get_connection()
}

pub fn connect_cluster(nodes: &[&str], password: Option<&str>) -> RedisResult<ClusterConnection> {
    let mut cluster_urls: Vec<String> = Vec::new();

    for node in nodes {
        if let Some(password) = password {
            cluster_urls.push(format!("redis://:{}@{}", password, node));
        } else {
            cluster_urls.push(format!("redis://{}", node));
        }
    }

    let client = ClusterClient::new(cluster_urls)?;
    client.get_connection()
}
