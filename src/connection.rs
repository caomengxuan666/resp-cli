use redis::{Client, Connection, RedisResult};
use redis::cluster::{ClusterClient, ClusterConnection};

pub fn connect(
    host: &str,
    port: &str,
    password: Option<&str>,
    unix: Option<&str>,
    tls: bool,
    _tls_ca_cert: Option<&str>,
    _tls_client_cert: Option<&str>,
    _tls_client_key: Option<&str>,
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
        
        Client::open(url)?
    };

    client.get_connection()
}

pub fn connect_cluster(
    nodes: &[&str],
    password: Option<&str>,
) -> RedisResult<ClusterConnection> {
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
