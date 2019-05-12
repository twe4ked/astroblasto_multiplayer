//! An Asteroids-ish example game to show off ggez.
//! The idea is that this game is simple but still
//! non-trivial enough to be interesting.
use astroblasto_multiplayer::{HashMapCodec, MainState};
use futures::sync::mpsc::unbounded;
use ggez::{conf, event, ContextBuilder, GameResult};
use std::{
    collections::HashMap,
    env,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path,
    sync::mpsc::channel,
};
use tokio::net::{UdpFramed, UdpSocket};
use tokio::prelude::*;
// use tokio_codec::LinesCodec;

const DEFAULT_MULTICAST: &'static str = "239.255.42.98";
const IP_ALL: [u8; 4] = [0, 0, 0, 0];

fn bind_multicast(
    addr: &SocketAddrV4,
    multi: &SocketAddrV4,
) -> Result<std::net::UdpSocket, std::io::Error> {
    use socket2::{Domain, Protocol, Socket, Type};

    let socket = Socket::new(Domain::ipv4(), Type::dgram(), Some(Protocol::udp()))?;

    socket.set_reuse_address(true)?;
    socket.bind(&socket2::SockAddr::from(*addr))?;
    socket.set_multicast_loop_v4(true)?;
    socket.join_multicast_v4(multi.ip(), addr.ip())?;

    Ok(socket.into_udp_socket())
}

fn main() -> GameResult {
    // We add the CARGO_MANIFEST_DIR/resources to the resource paths so that ggez will look in our
    // cargo project directory for files.
    let resource_dir = if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let mut path = path::PathBuf::from(manifest_dir);
        path.push("resources");
        path
    } else {
        path::PathBuf::from("./resources")
    };

    let hidpi_factor: f32;
    {
        // Create a dummy window so we can get monitor scaling information
        let cb = ContextBuilder::new("", "");
        let (_ctx, events_loop) = &mut cb.build()?;
        hidpi_factor = events_loop.get_primary_monitor().get_hidpi_factor() as f32;
    }

    let cb = ContextBuilder::new("astroblasto", "ggez")
        .window_setup(conf::WindowSetup::default().title("Astroblasto!"))
        .window_mode(
            conf::WindowMode::default().dimensions(800.0 * hidpi_factor, 600.0 * hidpi_factor),
        )
        .add_resource_path(resource_dir);

    let port = 1234;
    let addr = SocketAddrV4::new(IP_ALL.into(), port);
    let maddr = SocketAddrV4::new(
        DEFAULT_MULTICAST.parse::<Ipv4Addr>().expect("Invalid IP"),
        port,
    );

    assert!(maddr.ip().is_multicast(), "Must be multcast address");

    println!("Starting server on: {}", addr);
    println!("Multicast address: {}\n", maddr);

    let std_socket = bind_multicast(&addr, &maddr).expect("Failed to bind multicast socket");

    let socket = UdpSocket::from_std(std_socket, &tokio::reactor::Handle::default()).unwrap();

    let framed = UdpFramed::new(socket, HashMapCodec {});
    let (udp_tx, udp_rx) = Stream::split(framed);
    let (chn_tx, chn_rx) = unbounded::<HashMap<String, f64>>();

    let send = chn_rx
        .map(move |s| (s, SocketAddr::from(maddr)))
        .forward(udp_tx.sink_map_err(|e| println!("Error receiving UDP packet: {:?}", e)))
        .map(|_| ());

    let (tx, rx) = channel();

    let recv = udp_rx
        .for_each(move |(s, ip)| {
            let mut map = s.clone();
            map.insert(format!("ip-{}", ip), 0.0);
            tx.send(map).unwrap();
            Ok(())
        })
        .map_err(|e| println!("Error sending UDP packet: {:?}", e));

    let serve = send.select(recv).map(|_| ()).map_err(|_| ());

    std::thread::spawn(move || {
        tokio::run(serve);
    });

    let (ctx, events_loop) = &mut cb.build()?;

    let game = &mut MainState::new(ctx, chn_tx, rx, hidpi_factor)?;
    event::run(ctx, events_loop, game)
}
