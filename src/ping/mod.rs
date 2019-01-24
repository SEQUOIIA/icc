mod deps;
pub mod model;
use self::deps::*;

pub enum PingResult {
    Timeout{addr: IpAddr},
    Response{addr: IpAddr, rtt: Duration, sequence: u16, identifier: u16},
    Request{addr: IpAddr, sequence: u16, identifier: u16, sent_success: bool}
}

pub type PingUtilityResult = Result<(PingUtility, Receiver<PingResult>), String>;

pub struct PingUtility {
    // Time before ICMP request gets dropped
    timeout: Arc<Duration>,

    // Holds IP addresses to be pinged
    addresses: Arc<Mutex<BTreeMap<IpAddr, bool>>>,

    // Size of ICMP payload to be sent
    size: i32,

    // Sender of results channel
    results_channel_sender: Sender<PingResult>,

    // Sender of icmp v4
    tx_sender: Arc<Mutex<TransportSender>>,

    // Receiver of icmp v4
    rx_receiver: Arc<Mutex<TransportReceiver>>,

    // Sender of icmp v6
    txv6_sender: Arc<Mutex<TransportSender>>,

    // Receiver of icmp v6
    rxv6_receiver: Arc<Mutex<TransportReceiver>>,

    // Sender for passing data between threads
    thread_tx: Sender<PingResult>,

    // Receiver for passing data between threads,
    thread_rx: Arc<Mutex<Receiver<PingResult>>>,

    // Timer for tracking RTT,
    timer: Arc<RwLock<Instant>>,

    flag_stop: Arc<Mutex<bool>>,

    flag_ipv6_enable : bool,
}

impl PingUtility {
    pub fn new(max_timeout: Option<u64>) -> PingUtilityResult {
        let timeout : Arc<Duration>;
        if let Some(timeout_value) = max_timeout {
            timeout = Arc::new(Duration::from_millis(timeout_value));
        } else {
            timeout = Arc::new(Duration::from_millis(1000));
        }

        let protocol = Layer4(Ipv4(IpNextHeaderProtocols::Icmp));
        let (tx, rx) = match transport_channel(4096, protocol) {
            Ok((tx, rx)) => (tx, rx),
            Err(e) => return Err(e.to_string()),
        };

        let protocolv6 = Layer4(Ipv6(IpNextHeaderProtocols::Icmpv6));
        let (txv6, rxv6) = match transport_channel(4096, protocolv6) {
            Ok((txv6, rxv6)) => (txv6, rxv6),
            Err(e) => return Err(e.to_string()),
        };

        let (sender, receiver) = channel();
        let (thread_tx, thread_rx) = channel();

        let mut payload = PingUtility {
            timeout: timeout,
            addresses: Arc::new(Mutex::new(BTreeMap::new())),
            size: 16,
            results_channel_sender: sender,
            tx_sender: Arc::new(Mutex::new(tx)),
            rx_receiver: Arc::new(Mutex::new(rx)),
            txv6_sender: Arc::new(Mutex::new(txv6)),
            rxv6_receiver: Arc::new(Mutex::new(rxv6)),
            thread_rx: Arc::new(Mutex::new(thread_rx)),
            thread_tx: thread_tx,
            timer: Arc::new(RwLock::new(Instant::now())),
            flag_stop: Arc::new(Mutex::new(false)),
            flag_ipv6_enable: false
        };

        payload.start_listener();

        Ok((payload, receiver))
    }

    fn start_listener(&self) {
        // IPV4 ICMP packet dumping
        let thread_tx : Sender<PingResult> = self.thread_tx.clone();
        let rx : Arc<Mutex<TransportReceiver>> = self.rx_receiver.clone();
        let timer : Arc<RwLock<Instant>> = self.timer.clone();

        thread::spawn(move || {
            let mut receiver = rx.lock().unwrap();
            let mut iter = icmp_packet_iter(&mut receiver);

            loop {
                match iter.next() {
                    Ok((packet, addr)) => {
                        let mut identifier : u16 = 0;
                        let mut seq : u16 = 0;
                        match packet.get_icmp_type() {
                            IcmpTypes::EchoReply => {
                                let echo_reply_packet = echo_reply::EchoReplyPacket::new(packet.packet()).unwrap();
                                seq = echo_reply_packet.get_sequence_number();
                                identifier = echo_reply_packet.get_identifier();
                                //debug!("EchoReply -> {:?}", echo_reply_packet);
                            },
                            _ => {}
                        };

                        info!("{:?}", packet);
                        let start_time = timer.read().unwrap();
                        match thread_tx.send(PingResult::Response{addr: addr, rtt: Instant::now().duration_since(*start_time), sequence: seq, identifier: identifier}) {
                            Ok(_) => {},
                            Err(e) => {
                                error!("Error sending ping result on channel: {}", e)
                            }
                        }
                    },
                    Err(e) => {
                        // This will keep spamming on Windows the following:
                        // "ERROR icc::ping > An error occurred while reading: An invalid argument was supplied. (os error 10022)"
                        if !cfg!(windows) {
                            error!("An error occurred while reading: {}", e);
                        }
                    }
                }
            }
        });


        if self.flag_ipv6_enable {
            //IPV6 ICMP packet dumping
            let thread_txv6 = self.thread_tx.clone();
            let rxv6 = self.rxv6_receiver.clone();
            let timerv6 = self.timer.clone();
            thread::spawn(move || {
                let mut receiver = rxv6.lock().unwrap();
                let mut iter = icmpv6_packet_iter(&mut receiver);
                loop {
                    match iter.next() {
                        Ok((packet, addr)) => {
                            let mut identifier : u16 = 0;
                            let mut seq : u16 = 0;
                            match packet.get_icmpv6_type() {
                                Icmpv6Types::EchoReply => {
                                    let echo_reply_packet = echo_reply::EchoReplyPacket::new(packet.packet()).unwrap();
                                    seq = echo_reply_packet.get_sequence_number();
                                    identifier = echo_reply_packet.get_identifier();
                                    //debug!("EchoReply -> {:?}", echo_reply_packet);
                                },
                                _ => {}
                            };

                            let start_time = timerv6.read().unwrap();
                            match thread_txv6.send(PingResult::Response{addr: addr, rtt: Instant::now().duration_since(*start_time), sequence: seq, identifier: identifier}) {
                                Ok(_) => {},
                                Err(e) => {
                                    error!("Error sending ping result on channel: {}", e)
                                }
                            }
                        },
                        Err(e) => {
                            // This will keep spamming on Windows the following:
                            // "ERROR icc::ping > An error occurred while reading: An invalid argument was supplied. (os error 10022)"
                            if !cfg!(windows) {
                                error!("An error occurred while reading: {}", e);
                            }
                        }
                    }
                }
            });
        }

    }

    pub fn start_pinging(&self) {
        {
            let mut stop = self.flag_stop.lock().unwrap();
            *stop = false;
        }

        let thread_rx = self.thread_rx.clone();
        let tx_sender = self.tx_sender.clone();
        let txv6_sender = self.txv6_sender.clone();
        let results_channel_sender = self.results_channel_sender.clone();
        let flag_stop = self.flag_stop.clone();
        let addresses = self.addresses.clone();
        let timer = self.timer.clone();
        let timeout = self.timeout.clone();

        // While on Windows pnet only receives the pings it sends itself, that is not the case on Linux/OSX.
        // Therefore this keeps track of sequence numbers and identifiers that have been sent, to make sure that only pings that have
        // been sent by icc, is monitored.
        let mut ping_track : HashMap<String, bool> = HashMap::new();

        thread::spawn(move || {
            loop {
                for (address, seen) in addresses.lock().unwrap().iter_mut() {
                    if address.is_ipv4() {
                        let res : PingResult = Self::send_echo_request(&mut tx_sender.lock().unwrap(), *address);

                        if let PingResult::Request{addr, sequence, identifier, sent_success} = res {
                            ping_track.insert(format!("{};{};{}", addr.to_string(), sequence, identifier).to_owned(), true);
                        }

                    } else if address.is_ipv6() {
                        Self::send_echov6_request(&mut txv6_sender.lock().unwrap(), *address);
                    }
                    *seen = false;
                }

                {
                    let mut timer = timer.write().unwrap();
                    *timer = Instant::now();
                }

                loop {
                    match thread_rx.lock().unwrap().try_recv() {
                        Ok(result) => {
                            match result {
                                PingResult::Response {addr: addr, rtt: _, sequence: sequence, identifier: identifier} => {
                                    if let Some(seen) = addresses.lock().unwrap().get_mut(&addr) {
                                        *seen = true;
                                    }

                                    let sum = format!("{};{};{}", addr.to_string(), sequence, identifier);
                                    if ping_track.contains_key(sum.as_str()) {
                                        match results_channel_sender.send(result) {
                                            Ok(_) => {
                                                debug!("PingResult sent to results_channel_receiver")
                                            },
                                            Err(e) => {
                                                error!("Error sending ping result on channel: {}", e)
                                            }
                                        }
                                    }

                                },
                                _ => {}
                            }
                        },
                        Err(_) => {
                            let start_time = timer.read().unwrap();
                            if Instant::now().duration_since(*start_time) > *timeout {
                                break
                            }
                            use std::{thread, time};
                            thread::sleep(time::Duration::from_millis(50));
                        }
                    }
                }

                for (addr, seen) in addresses.lock().unwrap().iter() {
                    if *seen == false {
                        match results_channel_sender.send(PingResult::Timeout {addr: *addr}) {
                            Ok(_) => {},
                            Err(e) => {
                                error!("Error sending ping Idle result on channel: {}", e)
                            }
                        }
                    }
                }


                if *flag_stop.lock().unwrap() {
                    debug!("flag_stop activated");
                    return
                }
            }
        });
    }

    pub fn send_echo_request(tx: &mut TransportSender, address: IpAddr) -> PingResult {
        let mut buf : Vec<u8> = vec![0; 16];

        let mut echo_request_packet = echo_request::MutableEchoRequestPacket::new(&mut buf[..]).unwrap();
        echo_request_packet.set_sequence_number(random::<u16>());
        echo_request_packet.set_identifier(random::<u16>());
        echo_request_packet.set_icmp_type(IcmpTypes::EchoRequest);

        let csum = Self::icmp_checksum(&echo_request_packet);
        echo_request_packet.set_checksum(csum);

        let sequence_number = echo_request_packet.get_sequence_number();
        let identifier_number = echo_request_packet.get_identifier();

        match tx.send_to(echo_request_packet, address) {
            Ok(n) => {
                debug!("Using payload {} {} {}", &n, sequence_number, identifier_number);
                PingResult::Request {
                    addr: address.clone(),
                    sequence: sequence_number,
                    identifier: identifier_number,
                    sent_success: true
                }
            },
            Err(e) => panic!("failed to send packet: {}", e),
        }
    }

    pub fn send_echov6_request(tx: &mut TransportSender, address: IpAddr) {
        let mut buf : Vec<u8> = vec![0; 16];

        let mut echo_request_packet = MutableIcmpv6Packet::new(&mut buf[..]).unwrap();
        echo_request_packet.set_icmpv6_type(Icmpv6Types::EchoRequest);

        let csum = Self::icmpv6_checksum(&echo_request_packet);
        echo_request_packet.set_checksum(csum);

        match tx.send_to(echo_request_packet, address) {
            Ok(n) => {
                debug!("Using payload {}", &n);
            },
            Err(e) => panic!("failed to send packet: {}", e),
        }
    }

    fn icmp_checksum(packet: &echo_request::MutableEchoRequestPacket) -> u16 {
        util::checksum(packet.packet(), 1)
    }

    fn icmpv6_checksum(packet: &MutableIcmpv6Packet) -> u16 {
        util::checksum(packet.packet(), 1)
    }

    pub fn enable_ipv6(&mut self) {
        self.flag_ipv6_enable = true;
    }

    pub fn disable_ipv6(&mut self) {
        self.flag_ipv6_enable = false;
    }

    pub fn add_ipaddress(&self, ipaddress: &str) {
        let address = ipaddress.parse::<IpAddr>();
        match address {
            Ok(valid_address) => {
                debug!("Address added {}", valid_address);
                self.addresses.lock().unwrap().insert(valid_address, true);
            },
            Err(e) => {
                error!("Error adding ip address {}. Error: {}", ipaddress, e);
            }
        }
    }

    pub fn remove_ipaddress(&self, ipaddress: &str) {
        let address = ipaddress.parse::<IpAddr>();
        match address {
            Ok(valid_address) => {
                debug!("Address removed {}", valid_address);
                self.addresses.lock().unwrap().remove(&valid_address);
            },
            Err(e) => {
                error!("Error adding ip address {}. Error: {}", ipaddress, e);
            }
        }
    }
}