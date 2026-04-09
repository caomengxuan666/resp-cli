use redis::{Client, Connection, RedisResult};

pub fn connect(host: &str, port: &str, password: Option<&str>, unix: Option<&str>) -> RedisResult<Connection> {
    let client = if let Some(unix_path) = unix {
        Client::open(format!("unix://{}", unix_path))?
    } else {
        let url = if let Some(password) = password {
            format!("redis://:{}@{}:{}", password, host, port)
        } else {
            format!("redis://{}:{}", host, port)
        };
        Client::open(url)?
    };
    
    client.get_connection()
}
