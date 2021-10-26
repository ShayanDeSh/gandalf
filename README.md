# Gandolf
Gandolf is an implemention of Raft algorithm which can be used to bring consensus over any database.

## Overal
This project is consisted of two module `gandolf-consensus`  and `gandolf-kvs` which is the gandolf own database which can be replaced by others. The overala architecture of project is like the image below.
<p align="center">
  <img src="/doc/gandolf.jpg" width="70%" height="70%">
</p>

Each number represent a `RaftMessage` which is emmited in the system.

<div align="center"> 
  
| No. | Name |
| :-: | :--: |
|  1  | VoteMsg |
|  2  | VoteResp |
|  3  | AppendMsg |
|  4  | AppendResp |
|  5  | ClientReadMsg |
|  6  | ClientWriteMsg |
|  7  | ClientResp     |
|  8  | ClientError    |
|  9  | SnapMsg        |
|  10 | InstallSnapshot |
|  11 | InstallSnapshotResp |
  
</div>

## Gandolf-consensus
Gandol-consensus is the implemention of Raft algorithm so, it is responsible for bringing consensus to our system. It's also responsible for interacting with database and perform the command on it after the entry got commited. For connecting gandolf to your database you need to implement `Parser` and `Tracker` traits  for your database, then connect gandolf to your database and point your client to gandolf instead. You can use gandolf.conf files existing in repo to launch a cluster.

### Parser
Parser trait is responsible for converting client reqeust using the database protocol to somthing understandable for gandolf, so it can decide whether the request is from `Read` or `Write` kind. Then it will append the request to log and replicate it to other nodes if it's `Write` one or just perfrom it if it's a `Read`. 

### Tracker
Tracker trait is responsible for managing the raft log and also comunicating with database.

## Gandolf-KVS
Gandolf-KVS is a redis like key-value store which is highly ispired from tokio mini-redis and is used as the currently only supported database for Gandolf-onsensus module. It uses `RESP` for comunicating over tcp with client and also the consesnsus module. This module is consisted of two binary file which `gandolf-kvs-server` which is used for starting server and, `gandolf-kvs` which is the client for interacting with the server.  \
Currently supported commands are:

| Command | Functionality |
| :-----: | :----------: |
|   Set   | Set a value for a key |
|   Get   | Retrieve a value for a key |
|   Load  | Perform multiple set at one or load an snappshot |
|   Snap  | Take an snappshot |

The overal Architecture of kvs is like image below:

<p align="center">
  <img src="/doc/ganolf-kvs.jpg" width="70%" height="70%">
</p>

## Install

To install gandolf-kvs use:
```
cargo install gandolf-kvs
```

To install gandolf use:
```
cargo install gandolf
```
