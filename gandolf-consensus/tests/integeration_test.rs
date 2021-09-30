mod fixtures;

use gandolf_consensus::client::kvs::{KvsParser, KvsTracker}; 
use gandolf_consensus::Raft;

use gandolf_kvs::Frame;

use tokio::time::{Duration, sleep};

use std::net::SocketAddr;

use fixtures::{create_kvs_server, NodeConfig, create_cluster};

async fn kvs_cluster_of_nth(nth: u16) ->  gandolf_consensus::Result<Vec<(Raft<Frame, KvsTracker>, SocketAddr)>> {
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cluster_bootstrap() -> gandolf_consensus::Result<()>{
    let cluster = &mut kvs_cluster_of_nth(5).await?;

    let mut node5 = cluster.pop().unwrap();
    let mut node4 = cluster.pop().unwrap();
    let mut node3 = cluster.pop().unwrap();
    let mut node2 = cluster.pop().unwrap();
    let mut node1 = cluster.pop().unwrap();

    tokio::select! {
        _ = node1.0.run() => {
        },
        _ = node2.0.run() => {
        },
        _ = node3.0.run() => {
        },
        _ = node4.0.run() => {
        },
        _ = node5.0.run() => {
        },
        _ = sleep(Duration::from_secs(10)) => {
        }
    }

    Ok(()) 
}
