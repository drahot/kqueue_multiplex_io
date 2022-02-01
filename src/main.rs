extern crate nix;

use std::collections::HashMap;
use std::net::TcpListener;
use nix::sys::event::{
    KEvent, kqueue, kevent, EventFilter, FilterFlag, EventFlag,
};
use std::os::unix::io::{AsRawFd, RawFd};
use std::io::{BufRead, BufReader, BufWriter, Write};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:10000").unwrap();

    let kfd = kqueue().unwrap();
    let listener_fd = listener.as_raw_fd();
    let event: KEvent = KEvent::new(
        listener_fd as usize,
        EventFilter::EVFILT_READ,
        EventFlag::EV_ADD | EventFlag::EV_ENABLE,
        FilterFlag::NOTE_NONE,
        0,
        0,
    );

    let mut fd2buf = HashMap::new();
    let mut events = vec![event];
    let _ = kevent(kfd, &mut events.as_slice(), &mut [], 0).unwrap();

    while let Ok(nfds) = kevent(kfd, &[], events.as_mut_slice(), 0) {
        for n in 0..nfds {
            if events[n].ident() == event.ident() {
                if let Ok((stream, _)) = listener.accept() {
                    let fd = stream.as_raw_fd();
                    let stream0 = stream.try_clone().unwrap();
                    let reader = BufReader::new(stream0);
                    let writer = BufWriter::new(stream);

                    fd2buf.insert(fd, (reader, writer));

                    println!("accept fd: {}", fd);

                    let event: KEvent = KEvent::new(
                        fd as usize,
                        EventFilter::EVFILT_READ,
                        EventFlag::EV_ADD | EventFlag::EV_ENABLE,
                        FilterFlag::NOTE_NONE,
                        0,
                        0,
                    );
                    events.push(event);
                    let _ = kevent(kfd, events.as_slice(), &mut [], 0).unwrap();
                }
            } else {
                let fd = events[n].ident() as RawFd;
                let (reader, writer) = fd2buf.get_mut(&fd).unwrap();
                let mut buf = String::new();
                let n = reader.read_line(&mut buf).unwrap();
                if n == 0 {
                    let event: KEvent = KEvent::new(
                        fd as usize,
                        EventFilter::EVFILT_READ,
                        EventFlag::EV_DELETE,
                        FilterFlag::NOTE_NONE,
                        0,
                        0,
                    );
                    events.push(event);
                    let _ = kevent(kfd, events.as_slice(), &mut [], 0).unwrap();
                    events.pop();
                    fd2buf.remove(&fd);
                    println!("closed fd = {}", fd);
                    continue;
                }
                println!("read fd = {}, buf = {}", fd, buf);
                writer.write(buf.as_bytes()).unwrap();
                writer.flush().unwrap();
            }
        }
    }
}
