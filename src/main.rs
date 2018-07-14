extern crate dirs;
extern crate rusqlite;
extern crate tokio_udp;
extern crate trust_dns;
extern crate trust_dns_server;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;

use tokio_udp::UdpSocket;

use rusqlite::{Connection, OpenFlags};

use trust_dns::op::Message;
use trust_dns::op::MessageType;
use trust_dns::op::OpCode;
use trust_dns::op::ResponseCode;
use trust_dns::rr::record_data::RData;
use trust_dns::rr::record_type::RecordType;
use trust_dns::rr::resource::Record;
use trust_dns_server::server::{Request, RequestHandler, ResponseHandler};
use trust_dns_server::ServerFuture;

fn main() {
    let mut path = match dirs::home_dir() {
        Some(path) => path,
        None => panic!(),
    };

    path.push(".nixops");
    path.push("deployments");
    path.set_extension("nixops");

    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();

    let handler = Handler {
        connection: connection,
    };
    let server_future = ServerFuture::new(handler);

    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 5030);
    let socket = UdpSocket::bind(&address).unwrap();

    server_future.register_socket(socket);
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
                Err(_error) => Err(()),
            }
        }
        None => Err(()),
    }
}

struct Handler {
    connection: Connection,
}

impl RequestHandler for Handler {
    fn handle_request<'a, 'q, R: ResponseHandler>(
        &self,
        request: &Request<'q>,
        response: R,
    ) -> Result<(), std::io::Error> {
        let question = &request.message;

        let name = match question.queries().first() {
            Some(query) => query.name(),
            None => panic!(),
        };

        let mut truncated = name.to_string();
        let size = truncated.len() - 1;
        truncated.truncate(size);

        println!(
            "Received question {:?}, {}",
            question.message_type(),
            truncated
        );

        match resolve_hostname(truncated, &self.connection) {
            Ok(address) => {
                let mut message = Message::new();

                let mut record = Record::with(name.to_owned().into(), RecordType::A, 30);
                record.set_rdata(RData::A(address));

                message.set_id(question.id());
                message.set_message_type(MessageType::Response);
                message.add_answer(record);

                response.send(message)
            }
            Err(_error) => response.send(Message::error_msg(
                question.id(),
                OpCode::Query,
                ResponseCode::NXDomain,
            )),
        }
    }
}
