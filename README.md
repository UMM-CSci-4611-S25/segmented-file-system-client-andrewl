[![Review Assignment Due Date](https://classroom.github.com/assets/deadline-readme-button-22041afd0340ce965d47ae6ef1cefeee28c7c493a6346c4f15d667ab976d596c.svg)](https://classroom.github.com/a/wTmqQ3Sy)
# Segmented-File-Server-client <!-- omit in toc -->

The starter code and (limited) tests for the Segmented File System client.

- [Background](#background)
- [Segmenting the files](#segmenting-the-files)
- [The OutOfMoney.com protocol](#the-outofmoneycom-protocol)
  - [The packet structure](#the-packet-structure)
  - [How to construct packet numbers](#how-to-construct-packet-numbers)
- [Writing the client backend](#writing-the-client-backend)
  - [Establishing the connection](#establishing-the-connection)
  - [Starting the conversation](#starting-the-conversation)
  - [Processing the packets you receive](#processing-the-packets-you-receive)
  - [Data structures for packets in Rust](#data-structures-for-packets-in-rust)
  - [Data structures for files (packet groups) in Rust](#data-structures-for-files-packet-groups-in-rust)
- [Testing](#testing)
  - [Unit test your work](#unit-test-your-work)
  - [Check your work by running your client by hand](#check-your-work-by-running-your-client-by-hand)
  - [Check your work using `bats` tests](#check-your-work-using-bats-tests)

## Background

Those wacky folks at OutOfMoney.com are at it again, and have come up with another awesome money making scheme. This time they're setting up a service where users will use a client program to contact a new OutOfMoney.com server (using another old computer they found in the basement at the high school). Every time the client contacts the server, the server will send back three randomly chosen bits of of Justin Bieber arcana. These could be sound files, videos, photos, or text files (things like lyrics). They've got the server up and running, but the kid that had prototyped the client software has moved away suddenly. (His mom works for the CIA and they move a lot, usually with little notice.) Unfortunately he took all the code with him, and isn't responding to any attempts to contact him on Facebook.

In a rare fit of sanity, they've brought you in to help them out by building the backend of the client.

This starter code comes with some simple `bats` tests, but as discussed below you'll almost certainly want to add additional unit tests of your own to test the design and implementation of your data management tools.

## Segmenting the files

They're using socket-based connections between the client and server, but someone
who is long since fired decided that it was important that the server break the
files up into 1K chunks and send each chunk separately using UDP. Because of
various sources of asynchrony such as threads on the server and network delays,
you can't be sure you'll receive the chunks in the right order, so you'll need
to collect and re-assemble them before writing the contents out to the file.

Unlike the TCP socket connections you probably used for the Echo Client/Server labs
in Practicum, this system
uses UDP or Datagram sockets. TCP sockets provide you with a stable two-way connection that remains active
until you disconnect. TCP also guarantees that packets are delivered in the same
order that they are sent, and a good faith effort is made to re-send packets
that are lost along the way. Datagram sockets are less structured and provide no
guarantees about order or lost packets, and you essentially
just send out packets of information which can arrive at their destination in an
arbitrary order, much like the description of packets in Chapter 7 of
[Saltzer and Kaashoek](http://ocw.mit.edu/resources/res-6-004-principles-of-computer-system-design-an-introduction-spring-2009/).
On the listening end, instead of reading a stream of data, data arrives in packets,
and it's your job to interpret their contents. Typically there is some sort of protocol
that describes how packets are formed and interpreted; otherwise we end up with an impossible guessing game.

Rust provides direct support for UPD/datagram sockets primarily through the
[`UdpSocket` type](https://doc.rust-lang.org/stable/std/net/struct.UdpSocket.html). The
documentation includes examples of the methods; see
[this example](https://gist.github.com/lanedraex/bc01eb399614359470cfacc9d95993fb)
for a simple working client/server system.

> [!WARNING]
> If you're working on lab computers, make sure you shut down all your
> client processes before you leave the lab. If you leave them running you can
> block ports and make it impossible for other people to work on their lab on that computer.

## The OutOfMoney.com protocol

Your job is to write a (Rust) program that sends a UDP packet to
the server, and then waits and receives packets from the server until
all three expected files are completely received. When a file is complete,
it should be written to disk using the file name sent in the header
packet. When all three files have been written to disk, the client
should terminate cleanly. As mentioned above, since the file will be
broken up into chunks and sent using UDP, we need a protocol to tell
us how to interpret the packets we receive so we can correctly assemble
them. Those clever clogs at OutOfMoney.com didn't have much experience
(ok, any experience) designing these kinds of protocols, so theirs isn't
necessarily the greatest, but it gets the job done.

### The packet structure

In this protocol there are essentially two kinds of packets:

- A header packet with a unique file ID for the file being transferred, and
  the actual name of the file so we'll know what to call it after we've assembled
  the pieces
- A data packet, with the unique file ID (so we know what file this is part of),
  the packet number, and the data for that chunk.

Each packet starts with a status byte that indicates which type of packet it is:

- If the status byte is even (i.e., the least significant bit is 0), then this is a header packet
- If the status byte is odd (i.e., the least significant bit is 1), then this is a data packet
- If the status byte's second bit is also 1 (i.e., it's 3 mod 4), then this is the *last* data packet for this file
  They could have included the number of packets in the header packet, but they chose to to mark the last packet
  instead. Note that the last data packet (in terms of being the last bytes in the file) isn't guaranteed to come
  last, and might come anywhere in the stream including possibly being the *first* packet to arrive.

The packet numbers are consecutive and start from 0. So if a file is split into 18 chunks, there will be 18 data packets numbered 0 through 17, as well as the header packet for that file (for a total of 19 packets). The file IDs do *not* start with any particular value or run in any particular order, so you can't assume for example that they'll be 0, 1, and 2.

The structure of a header packet is then:

| status byte | file ID | file name                           |
|:------------|:--------|:------------------------------------|
| 1 byte      | 1 byte  | the rest of the bytes in the packet |

The structure of a data packet is:

| status byte | file ID | packet number | data                                |
|:------------|:--------|:--------------|:------------------------------------|
| 1 byte      | 1 byte  | 2 bytes       | the rest of the bytes in the packet |

> [!TIP]
> Note that you'll need to look at the length returned by `recv_from`
> to figure out how many bytes are in "the rest of the bytes in the
> packet". Most of the received packets will probably be "full", but the last
> packet is likely to be "short". You may assume that the maximum packet size,
> however, is 1028 bytes (a data packet with 4 bytes of bookkeeping and 1024
> bytes of data).

The decision to only use 1 byte for the file ID means that there can't be more than 256 files being transferred to a given client at a time. Given that the current business plan is to always send exactly three files that shouldn't be a problem, but they'll need to be aware of the limitation if they want to expand the service later.

> [!NOTE]
> Question to think about: Given that we're using 2 bytes for the packet number, and breaking files into 1K chunks, what's the largest file we can transfer using this system?

### How to construct packet numbers

A data packet has two bytes that specify the packet number, but how do you take
those two bytes and make a packet number out of them? It's "fairly"
straightforward although we do need to be clear about which byte
is the most significant byte and which is the least significant byte.
This protocol uses "big endian" ordering, where the first byte is the most
significant byte.

| most significant byte | least significant byte |
|:----------------------|:-----------------------|
| X                     | Y                      |

The question of whether the most significant bytes (or bits) come first
or last is important; either works, but it is crucial for a given protocol that everyone agrees on
an approach. This is what [arguments about "little endian" and "big endian"
systems](https://en.wikipedia.org/wiki/Endianness) are all about.

Rust provides a nice method for converting an array
of two bytes (represented as `u8`s) into a `u16`:

```rust
// Assume `bytes` is the array of bytes in a data packet.
let packet_number_bytes: [u8; 2] = [bytes[2], bytes[3]];
let packet_number = u16::from_be_bytes(packet_number_bytes);
```

The "be" in `from_be_bytes` stands for "big endian"; there is also a
`from_le_bytes` if you have a "little endian" protocol.

## Writing the client backend

As mentioned above, your Rust program starts things off by connecting (binding) a UDP socket to the server, and then sending a UDP packet to the server. It then waits and receives packets from the server until all three files are completely received. When a file is complete, it should be written to disk using the file name sent in the header packet. When all three files have been written to disk, the client should terminate cleanly.

### Establishing the connection

To establish a connection with the server requires two steps:

- Binding to port `7077` on the server with [`UdpSocket::bind()`](https://doc.rust-lang.org/stable/std/net/struct.UdpSocket.html#method.bind), which returns the desired `UdpSocket` value in a `std::io::Result` type.
- Call [`.connect()`](https://doc.rust-lang.org/stable/std/net/struct.UdpSocket.html#method.connect) on the returned socket to establish the two-way connection. This returns a `std::io::Result<()>` type so the system can indicate errors if necessary.

To do this you'll need to know the name of the server you're connecting to, and the port to use for the connection. We'll
use `127.0.0.1` (i.e., `localhost`) for the server, and port `7077`.

### Starting the conversation

You start things off by sending a (mostly empty) packet to the server as a way of saying "Hello – send me stuff!".

What should that initial packet look like in order to start
things off? Actually, it can be completely empty, since all you're doing is
announcing that you're interested. The only thing the server needs in order to respond
to your request is your IP address and port number, and all that is encoded in your
outgoing package "for free" by the operating system's implementation of the network stack.

So just create an
empty buffer, and send it out on the `UdpSocket` that you got from the call to
`bind()` above using [`.send()`](https://doc.rust-lang.org/stable/std/net/struct.UdpSocket.html#method.send).

### Processing the packets you receive

The main complication when receiving the packets is we don't control the order in which packets will be received. This means, among other things, that:

- The header packet won't necessarily come first, so we might start receiving
  data for a file before we've gotten the header for it (and know the file
  name). In an extreme case, we might get *all* the data packets (including the
  one with the "last packet" bit set) before we get the header packet.
- The data packets can arrive in random order, so we'll have to store them in
  some fashion until we have them all, and then put them in order before we
  write them out to the file.

> [!NOTE]
> It's possible to lose packets with UDP which, unlike TCP, won't request retransmission
> of lost packets. We're going to ignore that potential concern here, but it would
> need to be addressed if one wanted a more robust implementation of the client.

Other issues include:

- Packets will arrive from all three files interleaved, so we need to make sure
  we can store them sensibly so we can separate out packets for the different
  files.
- We don't know how many packets a file has been split up into until we see the
  packet with the "last packet" bit set.

As far as actually receiving the packets, you need to keep calling
[`socket.recv(&mut buf)`](https://doc.rust-lang.org/stable/std/net/struct.UdpSocket.html#method.recv)
on the `UdpSocket` until you've received
all the packets. Since you know that each
packet has no more than 1024 bytes of data, the buffer in the packet needs to
be big enough for the 1024 bytes of data plus the maximum amount of header
information as discussed in the packet structure description up above, i.e.,
at least 1028 bytes.

### Data structures for packets in Rust

There are a lot of ways you could do this in Rust; I'm going to sketch parts of one
approach in the hopes that this is useful.

First, I want a `Packet` type that is either a `Header` or `Data` packet:

```rust
pub enum Packet {
    Header(Header),
    Data(Data)
}
```

Then `Header` is a struct with a `file_id` and a `file_name` (as a `String`).

```rust
pub struct Header {
    file_id: u8,
    file_name: OsString
}
```

> [!TIP]
> In `Header` we're using [`OsString`](https://doc.rust-lang.org/stable/std/ffi/struct.OsString.html)
> instead of `String`, since `OsString` is the recommended way of representing operating system
> specific strings, such as file names. See [the `OsString` part of "Representations of non-Rust strings"](https://doc.rust-lang.org/stable/std/ffi/index.html#representations-of-non-rust-strings)
> in the FFI (foreign function interface) documentation for more.
>
> We can convert a `String` (e.g., `s`) to an `OsString` with expressions like `s.into()`.
> Sometimes Rust can't infer what type you're trying to convert `s` into, so you have to
> be explicit, either through declaring the type of the target variable, or using `OsString::from()`.
>
> ```rust
> let s = "My file name".to_string();
> // Declaring the type of `t` makes it clear what `into()` should convert to.
> let t: OsString = s.into();
> // Using `OsString::from()` avoids the need for declaring the type of `u`.
> let u = OsString::from(s);
> ```

And finally, `Data` is a similar struct:

```rust
pub struct Data {
    file_id: u8,
    packet_number: u16,
    is_last_packet: bool,
    data: Vec<u8>
}
```

Note that `packet_number` is `u16` since it is formed using two bytes of data.

> [!TIP]
> For each of these, you probably want to implement the `TryFrom<&[u8]>` trait which
> you can then use to convert `buf` into the relevant packet type. You probably also
> want a `::new()` constructor for `Header` and `Data` and it may be useful to have
> ["get" methods](https://rust-lang.github.io/api-guidelines/naming.html#getter-names-follow-rust-convention-c-getter)
> to retrieve the relevant parts of packets.

### Data structures for files (packet groups) in Rust

> [!TIP]
> Most of this is really a data structures problem. Before you start banging on the keyboard, take some time to talk about how you're going to unmarshal the packets and store their data. Having a good plan for that will make a huge difference.

You'll also want some kind of data structure that collects packets for a specific
file; I called my `PacketGroup` but you may find a better name:

```rust
pub struct PacketGroup {
    file_name: Option<OsString>,
    expected_number_of_packets: Option<usize>,
    packets: HashMap<u16, Vec<u8>>
}
```

Note that I use `Option<OsString>` for the `file_name` because we don't initially know what the
file name is (so we use `None`), and once we learn the file name we can replace it with a
`Some` variant. The same is true for `expected_number_of_packets`.

The `packets` `HashMap` maps from
packet number (a `u16`) to the data for that packet (`Vec<u8>`).

You'll probably want to implement a variety of methods on this type.
I had the following, but your mileage may vary:

- `received_all_packets(&self) -> bool`
- `process_packet(&mut self, packet: Packet)`
  - Basically add this packet to the packet group. It works by figuring
    out which kind of packet this is, and then calling one of the
    two following methods:
  - `process_header_packet(&mut self, header: Header)`
  - `process_data_packet(&mut self, data: Data)`
- `write_file(&self) -> io::Result<()>`

## Testing

### Unit test your work

> [!IMPORTANT]
> If you don't write tests the little bugs in your code become alive and attack you during your sleep!
> -- @JustusFluegel

While the network stuff is difficult to test, all the parsing and packet/file
assembly logic is entirely testable. I would *strongly* encourage you to write
some tests for that "data structures" part to help define the desired behavior
and identify logic issues. Debugging logic problems when you're interacting
with the actual server will be a real nuisance, so isolating that part as
much as possible would be a Good Idea.

You might, for example, have a
`Data` struct (as I suggested above)
which can be construction an array of bytes using `try_from`.
That `try_from` logic could then be
responsible for extracting the status bytes, file ID, packet number, and data,
and storing them in fields that are accessible through various "get"
methods. You could then write tests
that construct instances of the `Data` struct and verify that the resulting
structs have the correct status bytes, file ID, packet number, and data.

In this test, for example, we create a vector of `u8` that contains:

- 0 as the status byte (which indicates that it's a header packet because it's even)
- (the second) 0 as the file ID
- four bytes of data that represent the unicode for the sparkle heart emoji.

We then use our `try_from` implementation to convert that to a `Header` struct,
and assert that we get a `Header` struct with the correct file ID (12) and the
correct file name (`"This file is lovely 💖"`).

> [!TIP]
> `\xPQ` is a byte whose value is 16*P+Q where P and Q are both hexadecimal
> digits. So `\x00` is the byte having value 0, and `\x0C` is the byte having
> value 12 (in decimal).
>
> In this example, we're setting the status byte to
> 0, the file ID byte to 12, and the file name to the string containing all
> the remaining characters, i.e., "This file is lovely 💖". Note that because
> Rust strings support full Unicode, we can include things like emojis in
> our packets.
>
> The `.as_bytes()` call converts the string to a reference to an
> array of bytes, correctly handling multi-byte characters like the emoji
> (which converts to four bytes: `[240, 159, 146, 150]`).
>
> Be aware, however, that not all operating systems support emojis
> in places like file names, so you might want to be careful about creating
> files with "interesting" names like this.

```rust
    #[test]
    fn emoji_in_file_name() {
        let sparkle_heart: &[u8] = "\x00\x0CThis file is lovely 💖".as_bytes();
        let result = Header::try_from(sparkle_heart);
        assert_eq!(
            result,
            Ok(Header {
                file_id: 12,
                file_name: "This file is lovely 💖".to_string().into()
            })
        );
    }
```

You could also have a `FileManager` type that you hand packets to and which
manages organizing and storing all the packets for a group of files.
You could then hand it a small
set of test packets that you make up, and verify that it assembles the correct
files. The `FileManager` could, for example, create `PacketGroup`s, one per file.
A `PacketGroup` could contain the packets for a file, and have getter methods
for the file name, the number of packets (what if it isn't known yet?), the
number actually received, whether the file is complete, and the data from those
packets after sorting them in the correct order.

All of these ideas are just that: ideas. Your group should definitely spend
some time discussing how you want to organize all this, and how you want to
test that. If you're not clear on how you'd structure something for testing,
*come ask* rather than just banging out a bunch of code that will just confuse
us all later.

You don't have to do things like test `UdpSocket` or file writing. The
Rust (and operating systems) folks are responsible for the correctness
of those system calls and we'll
trust them on that. Testing that you're writing out the correct files is
essentially handled in the `bats` tests below, so don't bother writing
unit tests for that.

### Check your work by running your client by hand

In addition to your unit tests, you can run your program "by hand" and see if
the files you get back match the expected files. Assuming your server is running,
you can run your client with

```bash
cargo run
```

If your client is working correctly, this script should terminate gracefully,
if slowly (there are lots of packets to process), leaving three files in
the directory you ran it in:

- `small.txt`
- `AsYouLikeIt.txt`
- `binary.jpg`

The `tests/target-files` folder in the repository contains three files with
the same names – these are copies of the expected files and the files your
program downloaded should match these three files exactly.
So, for example, running a command like this

```bash
diff binary.jpg tests/target-files/binary.jpg
```

should return no differences. You should also be able to examine the contents
of the files you received and assembled and confirm that they look reasonable.

A common problem is that you didn't write the last few bytes of data to the
file. This might show up as the long text file missing the last few characters
or lines. In `binary.jpg` this might show up as some black pixels in the bottom
right of the image.

### Check your work using `bats` tests

There's a (quite simplistic) `bats` test that you can use to run your client
and check that the files you get match the expected files.

> [!WARNING]
> Run it from the top-level directory, i.e.,
>
> ```bash
> bats tests/client_tests.sh
> ```

It basically does the "hand test" described above against your local server,
and `diff`s the files you downloaded against the three expected files.

If these pass, then your code is probably in good shape from a correctness
standpoint, but you should still make sure you have reasonable unit tests
and clean, well-organized code.
