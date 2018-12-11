# About
This is a distributed chat system made in [Rust](https://www.rust-lang.org/)!

# Instructions
## how to build

```sh
cargo build
```

# how to use
## first create a room with
```sh
peas --create-room ROOMNAME
```
which creates a file *ROOMNAME.peas-room*

## maybe start a tracker
to start a new one
```sh
tracker
```

## connect
lastly conenct via the tracker
```sh
peas --username USER --room ROOMNAME.peas-room --tracker xxx.xxx.xxx.xxx.ppp
```
