# Peer2Peer
A peer-to-peer application that lists all of the users that allows users to send messages among each other.

## Authors
Wesley Ng
Reed Maniscalchi

## Usage
Run with `RUST_LOG=info cargo run` in multiple terminals with each containing a `users.json` file, same, different, or newly fresh.

When starting, hit "enter" to start the app.

A `users.json` file must in the folder where it's executed, otherwise an error will be thrown.

Command List:

* `ls p` - list all peers
* `ls u` - list local users
* `ls u all` - list all public users from all known peers
* `ls u {peerId}` - list all public users from the given peer
* `create u Name|Pronouns|Phone_Number` - create a new user with the given data, the `|` are important as separators
* `send u Name|Message` - send a user a message with with the specified name, the `|` spearates the name and message
* `edit u id|[name,phone,about]|[replacement]` - edits the user profile of an individual based on their id, the `|` separates the id, what attribute to edit, and its replacement
* `publish u {userId}` - publish user with the given user ID

## Proposal

&nbsp;&nbsp;&nbsp;&nbsp;In our Peer-to-Peer Network Project, we plan to make a concurrent private messaging app that allows users to send as many text messages amongst the group without needing to wait for others to respond back. This is similar to how other texting apps work, such as iPhone Messages, Messenger, WhatsApp, Discord, and more. 
&nbsp;&nbsp;&nbsp;&nbsp;To implement this project, we will use Rust and libraries such as tokio and tokio-tungstenite for asynchronous I/O, networking, and websocket communication. Using these can allow the receiver to get the text messages that were stored in the host’s device when both the host and the receiver is online. This means that if a host wants to send 4 text messages to the receiver, those text messages will be put on hold until both the receiver and the host gains a connection between them.
&nbsp;&nbsp;&nbsp;&nbsp;This proposal was meant to be a tentative projection of what is feasible within the given two weeks of time, excluding Easter holiday. We can attempt to add more features such as sending photos, videos, or files, but what is stated in this proposal serves as the main goal for the project.


## Resources used for GUI (Slint)
https://releases.slint.dev/1.5.1/docs/tutorial/rust/introduction
https://releases.slint.dev/1.5.1/docs/slint/