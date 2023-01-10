use std::thread;
use std::io::{Error, ErrorKind};
use paho_mqtt::{Client, ConnectOptionsBuilder, CreateOptionsBuilder, message::Message};
use tokio::sync::watch;
use tokio::sync::watch::Receiver;

/**
    This is to manage the connection to the Chirpstack Application Server.
*/
#[derive(Clone)]
pub struct ChirpstackConnection {
    /// The API token which must be created in the Chirpstack Application Server.
    api_token: String,
    /// The URI on which the Chirpstack Application Server is reached.
    uri: String,
}

impl ChirpstackConnection {

    /// Creates a new connection. <br/>
    /// The API token can be created in the Chirpstack Application Server.<br/>
    /// The uri string is where the the Chirpstack Application Server can be reached.
    pub fn new(api_token: &str, uri: &str) -> Self {
        let con = ChirpstackConnection {
            api_token: api_token.to_string(),
            uri: uri.to_string(),
        };
        return con;
    }

    /// Gets a copy of uri.
    pub fn get_uri(&self) -> String {
        return self.uri.clone();
    }

    /// Sets a new uri. <br/> The uri where the Chirpstack Application Server can be reached.
    pub fn change_uri(&mut self, uri: &str)  {
        self.uri = uri.to_string();
    }

    /// Gets a copy of API token.
    pub fn get_api_token(&self) -> String {
        return self.api_token.clone();
    }

    /// Sets new api token. <br/> The API token can be created in the Chirpstack Application Server.
    pub fn set_api_token(&mut self, api_token: &str) {
        self.api_token = api_token.to_string();
    }

}

/**
    This is to manage the connection to a Mqtt broker.
*/
pub struct Mqtt {
    /// The URI on which the broker is reached.
    uri: String,
    /// The username to access the broker.
    username: String,
    /// The password to access the broker.
    password: String,
}

impl Mqtt {

    /// Creates a new mqtt.<br/>
    /// The uri, username and password for the connection to the mqtt broker.<br/>
    /// The username must be valid and the respective user has to have permission to sent
    /// and receive messages.
    pub fn new(uri: &str, username: &str, password: &str) -> Self {
        return Mqtt {
            uri: uri.to_string(),
            username: username.to_string(),
            password: password.to_string(),
        }
    }

    /// Gets a copy of the uri.
    pub fn get_uri(&self) -> String {
        return self.uri.clone();
    }

    /// Sets a new uri. <br/> The uri where to reach the Mqtt broker.
    pub fn change_uri(&mut self, uri: &str)  {
        self.uri = uri.to_string();
    }

    /// Changes the username. <br/>
    /// The username must be valid and the respective user has to have permission to sent
    /// and receive messages.
    pub fn change_username(&mut self, username: &str) {
        self.username = username.to_string();
    }

    /// Changes the password.
    pub fn change_password(&mut self, password: &str) {
        self.password = password.to_string();
    }


    /// Connects to the mqtt broker and publishes all messages to a `tokio::sync::watch` channel
    pub fn start_receiving(&self) -> Result<Receiver<Message>, Error> {
        let res = self.create_client();
        let client = match res {
            Ok(client) => client,
            Err(e) => return Err(Error::new(ErrorKind::Other, "There was a problem with the creation of the mqtt client: ".to_owned() + &e.to_string())),
        };
        let mut topics = Vec::new();
        let topic = "application/+/device/+/event/up";
        topics.push(topic);
        let result = client.subscribe_many(topics.as_slice(), &[0]);
        if let Err(e) = result {
            return Err(Error::new(ErrorKind::Other, "There was a problem with the subscription to mqtt: ".to_owned() + &e.to_string()));
        }
        let (tx, rx) = watch::channel(Message::new("", "", 0));

        thread::spawn(move|| {
            'mqtt_loop: loop {
                if !client.is_connected() {
                    let res = client.reconnect().unwrap();
                    println!("{:?}", res.reason_code().to_string());
                }

                let receiver = client.start_consuming();
                let rec = receiver.recv();
                println!("Message received!");
                let message = match rec {
                    Ok(mes) => mes,
                    Err(e) => {
                        println!("RecvError: {}", e.to_string());
                        continue 'mqtt_loop
                    }
                };
                println!("Message: {}", message.clone().unwrap().topic());
                if message.is_some() {
                    if let Err(e) = tx.send(message.unwrap()) {
                        println!("Sending error: {}", e.to_string());
                        continue 'mqtt_loop
                    }
                }
            }
        });

        Ok(rx)
    }

    /// Creates a new client and connects to the Mqtt Broker.
    fn create_client(&self) -> Result<Client, Error> {
        let option = CreateOptionsBuilder::new().server_uri(self.uri.clone()).client_id("elorapi").finalize();
        let client_resp = Client::new(option);
        let client = match client_resp {
            Ok(a) => a,
            Err(_) => return Err(Error::new(ErrorKind::NotConnected, "Something went wrong!")),
        };
        let connection_option = ConnectOptionsBuilder::new().user_name(self.username.clone()).password(self.password.clone()).finalize();
        let conn_resp = client.connect(connection_option);
        let server_response = match conn_resp {
            Ok(a) => a,
            Err(_) => return Err(Error::new(ErrorKind::NotConnected, "Something went wrong!")),
        };
        if server_response.connect_response().is_none() {
            return Err(Error::new(ErrorKind::NotConnected, "Not the right response was getting back"));
        }
        Ok(client)
    }
}