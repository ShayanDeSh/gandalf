use gandalf_consensus::client::kvs::{KvsParser, KvsTracker}; 
use gandalf_consensus::Raft;

use gandolf_kvs::Frame;

use tokio::time::{Duration, sleep};

use std::net::SocketAddr;

use super::common::{create_kvs_server, NodeConfig, create_cluster};

use std::cell::RefCell;

use gandolf_kvs::client;

pub async fn kvs_cluster_of_nth(nth: u16) ->  gandalf_consensus::Result<Vec<(RefCell<Raft<Frame, KvsTracker>>, SocketAddr)>> {
    let mut ts = Vec::new();
    let mut cs = Vec::new();

    for i in 0..nth {
        let k = create_kvs_server().await;
        let nodes = Some((0..nth).into_iter()
            .filter_map(|x| 
                if x != i { Some(format!("127.0.0.1:{}", 7900 + x).to_string()) } 
                else { None }
                )
            .collect()
            );

        let c = NodeConfig {
            port: 7900 + i,
            host: "127.0.0.1".to_string(),
            nodes,
            heartbeat: 500,
            snapshot_offset: 100,
            timeout: 1500,
            client_port: k.port(),
            client_host: "127.0.0.1".to_string(),
            connection_port: 9876 + i,
            connection_host: "127.0.0.1".to_string(),
            snapshot_path: std::env::temp_dir().to_str().unwrap().to_string()
        };
        let a = format!("{}:{}", c.client_host, c.client_port).parse()?;
        let t = KvsTracker::new(a, c.snapshot_path.clone(), c.snapshot_offset);

        cs.push(c);
        ts.push(t);

    }

    create_cluster(cs, ts, KvsParser).await
}

pub async fn client_write_requset(count: u32, addr: String, sleep_duration: Duration) -> gandalf_consensus::Result<()> {
    sleep(sleep_duration).await;
    let mut con = client::connect(addr).await?;
    for i in 0..count {
        let key = format!("foo{}", i);
        let value = format!("{}", i);
        con.set(&key, value.into()).await?;
    }
    sleep(Duration::from_secs(2)).await;
    Ok(())
}

pub async fn client_read_requset(count: u32, addr: String, sleep_duration: Duration) -> gandalf_consensus::Result<()> {
    sleep(sleep_duration).await;
    let mut con = client::connect(addr).await?;
    for i in 0..count {
        let key = format!("foo{}", i);
        if let None = con.get(&key).await? {
            assert!(false);
        }
    }
    sleep(Duration::from_secs(2)).await;
    Ok(())
}
