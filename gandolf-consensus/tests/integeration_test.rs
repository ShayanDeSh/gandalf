mod fixtures;

use gandolf_consensus::client::kvs::{KvsParser, KvsTracker}; 
use gandolf_consensus::Raft;
use gandolf_consensus::raft::State;

use gandolf_kvs::Frame;

use tokio::time::{Duration, sleep};

use std::net::SocketAddr;

use fixtures::{create_kvs_server, NodeConfig, create_cluster};

use std::cell::RefCell;

use gandolf_kvs::client;

async fn kvs_cluster_of_nth(nth: u16) ->  gandolf_consensus::Result<Vec<(RefCell<Raft<Frame, KvsTracker>>, SocketAddr)>> {
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

async fn client_write_requset(count: u32, addr: String, sleep_duration: Duration) -> gandolf_consensus::Result<()> {
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

#[tokio::test(flavor = "multi_thread", worker_threads = 5)]
async fn test_cluster_bootstrap() -> gandolf_consensus::Result<()>{
    let cluster = kvs_cluster_of_nth(5).await?;

    let mut node1 = cluster.get(0).unwrap().0.borrow_mut();
    let mut node2 = cluster.get(1).unwrap().0.borrow_mut();
    let mut node3 = cluster.get(2).unwrap().0.borrow_mut();
    let mut node4 = cluster.get(3).unwrap().0.borrow_mut();
    let mut node5 = cluster.get(4).unwrap().0.borrow_mut();
    
    tokio::select! {
        _ = node1.run() => {
            assert!(false);
        },
        _ = node2.run()  => {
            assert!(false);
        },
        _ = node3.run()  => {
            assert!(false);
        },
        _ = node4.run()  => {
            assert!(false);
        },
        _ = node5.run()  => {
            assert!(false);
        },
        _ = sleep(Duration::from_secs(3)) => {
        }
    }

    drop(node1);
    drop(node2);
    drop(node3);
    drop(node4);
    drop(node5);

    let follower_number = cluster
        .iter()
        .fold(0, |acc, c| if c.0.borrow().state == State::Follower { acc + 1 } else { acc });

    assert_eq!(cluster.len() - 1, follower_number);

    Ok(()) 
}

#[tokio::test(flavor = "multi_thread", worker_threads = 5)]
async fn test_re_election() -> gandolf_consensus::Result<()> {
    let cluster = kvs_cluster_of_nth(5).await?;

    let mut node1 = cluster.get(0).unwrap().0.borrow_mut();
    let mut node2 = cluster.get(1).unwrap().0.borrow_mut();
    let mut node3 = cluster.get(2).unwrap().0.borrow_mut();
    let mut node4 = cluster.get(3).unwrap().0.borrow_mut();
    let mut node5 = cluster.get(4).unwrap().0.borrow_mut();

    node1.set_state(State::Leader);
    
    node1.current_term = 1;
    node2.current_term = 1;
    node3.current_term = 1;
    node4.current_term = 1;
    node5.current_term = 1;

    tokio::select! {
        _ = node1.run() => {
            assert!(false);
        },
        _ = node2.run()  => {
            assert!(false);
        },
        _ = node3.run()  => {
            assert!(false);
        },
        _ = node4.run()  => {
            assert!(false);
        },
        _ = node5.run()  => {
            assert!(false);
        },
        _ = sleep(Duration::from_secs(5)) => {
        }
    }

    node1.set_state(State::Follower);

    tokio::select! {
        _ = node1.run() => {
            assert!(false);
        },
        _ = node2.run()  => {
            assert!(false);
        },
        _ = node3.run()  => {
            assert!(false);
        },
        _ = node4.run()  => {
            assert!(false);
        },
        _ = node5.run()  => {
            assert!(false);
        },
        _ = sleep(Duration::from_secs(5)) => {
        }
    }

    drop(node1);
    drop(node2);
    drop(node3);
    drop(node4);
    drop(node5);

    let follower_number = cluster
        .iter()
        .fold(0, |acc, c| if c.0.borrow().state == State::Follower { acc + 1 } else { acc });

    assert_eq!(cluster.len() - 1, follower_number);

    let _: Vec<_> = cluster.
        iter()
        .map(|c| assert!(c.0.borrow().current_term > 1)).collect();

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 5)]
async fn test_direct_write() -> gandolf_consensus::Result<()> {
    let cluster = kvs_cluster_of_nth(5).await?;

    let mut node1 = cluster.get(0).unwrap().0.borrow_mut();
    let mut node2 = cluster.get(1).unwrap().0.borrow_mut();
    let mut node3 = cluster.get(2).unwrap().0.borrow_mut();
    let mut node4 = cluster.get(3).unwrap().0.borrow_mut();
    let mut node5 = cluster.get(4).unwrap().0.borrow_mut();

    node1.current_term = 1;
    node1.set_state(State::Leader);

    node2.current_term = 1;
    node2.current_leader = Some(node1.id.clone());

    node3.current_term = 1;
    node3.current_leader = Some(node1.id.clone());

    node4.current_term = 1;
    node4.current_leader = Some(node1.id.clone());

    node5.current_term = 1;
    node5.current_leader = Some(node1.id.clone());
    
    let connection_addr = format!("127.0.0.1:{}", 9876).to_string();

    tokio::select! {
        _ = node1.run() => {
            assert!(false);
        },
        _ = node2.run()  => {
            assert!(false);
        },
        _ = node3.run()  => {
            assert!(false);
        },
        _ = node4.run()  => {
            assert!(false);
        },
        _ = node5.run()  => {
            assert!(false);
        },
        res = client_write_requset(10, connection_addr, Duration::from_secs(0)) => {
            res?
        }
    }

    drop(node1);
    drop(node2);
    drop(node3);
    drop(node4);
    drop(node5);

    let _: Vec<_> = cluster.
        iter()
        .map(|c| assert!(c.0.borrow().get_commit_index() == 10)).collect();

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 5)]
async fn test_indirect_write() -> gandolf_consensus::Result<()> {
    let cluster = kvs_cluster_of_nth(5).await?;

    let mut node1 = cluster.get(0).unwrap().0.borrow_mut();
    let mut node2 = cluster.get(1).unwrap().0.borrow_mut();
    let mut node3 = cluster.get(2).unwrap().0.borrow_mut();
    let mut node4 = cluster.get(3).unwrap().0.borrow_mut();
    let mut node5 = cluster.get(4).unwrap().0.borrow_mut();

    
    node1.current_term = 1;
    node1.set_state(State::Leader);

    node2.current_term = 1;
    node2.current_leader = Some(node1.id.clone());

    node3.current_term = 1;
    node3.current_leader = Some(node1.id.clone());

    node4.current_term = 1;
    node4.current_leader = Some(node1.id.clone());

    node5.current_term = 1;
    node5.current_leader = Some(node1.id.clone());
    
    let connection_addr = format!("127.0.0.1:{}", 9877).to_string();

    tokio::select! {
        _ = node1.run() => {
            assert!(false);
        },
        _ = node2.run()  => {
            assert!(false);
        },
        _ = node3.run()  => {
            assert!(false);
        },
        _ = node4.run()  => {
            assert!(false);
        },
        _ = node5.run()  => {
            assert!(false);
        },
        res = client_write_requset(10, connection_addr, Duration::from_secs(0)) => {
            res?
        }
    }

    drop(node1);
    drop(node2);
    drop(node3);
    drop(node4);
    drop(node5);

    let _: Vec<_> = cluster.
        iter()
        .map(|c| assert!(c.0.borrow().get_commit_index() == 10)).collect();

    Ok(())
}
