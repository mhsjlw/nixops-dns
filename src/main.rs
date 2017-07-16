extern crate trust_dns_server;
extern crate trust_dns;
extern crate rusqlite;

use std::env;
use std::net::{UdpSocket, Ipv4Addr};
use std::str::FromStr;

use rusqlite::{Connection, SQLITE_OPEN_READ_ONLY};
use trust_dns::op::Message;
use trust_dns::op::op_code::OpCode;
use trust_dns::op::response_code::ResponseCode;
use trust_dns::rr::resource::Record;
use trust_dns::rr::record_type::RecordType;
use trust_dns::op::header::MessageType;
use trust_dns::rr::record_data::RData;
use trust_dns_server::ServerFuture;
use trust_dns_server::server::{RequestHandler, Request};

fn main() {
    let mut path = match env::home_dir() {
      Some(path) => path,
      None => panic!()
    };

    path.push(".nixops");
    path.push("deployments");
    path.set_extension("nixops");

    let connection = Connection::open_with_flags(path, SQLITE_OPEN_READ_ONLY).unwrap();

    let handler = Handler { connection: connection };
    let mut server_future = ServerFuture::new(handler).unwrap();

    let mut socket = UdpSocket::bind("127.0.0.1:5300").unwrap();

    server_future.register_socket(socket);
    server_future.listen();
}

fn resolve_hostname(hostname: String, connection: &Connection) -> Result<Ipv4Addr, ()> {
  let mut statement = connection.prepare("
    select value from ResourceAttrs where machine = (select id from Resources where name = ?) and name = 'publicIpv4'
  ").unwrap();

  let mut rows = statement.query(&[&hostname]).unwrap();

  match rows.next() {
    Some(row) => {
      let unparsed: String = row.unwrap().get(0);
      match Ipv4Addr::from_str(&unparsed[..]) {
        Ok(address) => Ok(address),
        Err(error) => Err(())
      }
    },
    None => Err(())
  }
}

struct Handler {
  connection: Connection
}

impl RequestHandler for Handler {
  fn handle_request(&self, request: &Request) -> Message {
    let question = &request.message;

    let name = match question.queries().first() {
      Some(query) => query.name(),
      None => panic!()
    };

    let mut truncated = name.to_string();
    let size = truncated.len() - 1;
    truncated.truncate(size);

    println!("Received question {:?}, {}", question.message_type(), truncated);

    match resolve_hostname(truncated, &self.connection) {
      Ok(address) => {
        let mut message = Message::new();

        let mut record = Record::with(name.to_owned(), RecordType::A, 30);
        record.set_rdata(RData::A(address));

        message.set_id(question.id());
        message.set_message_type(MessageType::Response);
        message.add_answer(record);

        return message;
      },
      Err(error) => Message::error_msg(question.id(), OpCode::Query, ResponseCode::NXDomain)
    }
  }
}