mod fixtures;

use gandolf_consensus::raft::State;

use tokio::time::{Duration, sleep};

use fixtures::kvs_helpers::kvs_cluster_of_nth;

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
async fn test_time_out() -> gandolf_consensus::Result<()> {
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

    node2.set_state(State::Candidate);

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

    Ok(())
}
